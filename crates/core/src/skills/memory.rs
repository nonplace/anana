use serde::{Deserialize, Serialize};

use crate::{Consciousness, CoreError, Phenotype, SkillId, Skills, Tick};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum PracticeKind {
    Restudy,
    Retrieval,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
pub struct SkillMemory {
    pub last_practiced: Option<Tick>,
    pub stability: u16,
    pub baseline_xp: u32,
}

#[must_use]
pub fn spacing_strength_gain(gap: u32, intended_retention: u32) -> u16 {
    let retention = intended_retention.max(1);
    let target = (retention / 5).max(1);
    let score = if gap <= target {
        u64::from(gap)
            .saturating_mul(u64::from(gap))
            .saturating_mul(1000)
            / u64::from(target).saturating_mul(u64::from(target)).max(1)
    } else {
        1000_u64.saturating_sub(
            u64::from(gap.saturating_sub(target)).saturating_mul(300)
                / u64::from(retention.saturating_sub(target).max(1)),
        )
    };
    score.min(1000) as u16
}

#[must_use]
pub fn retained_experience(baseline_xp: u32, elapsed: u64, stability: u16) -> u32 {
    let root = integer_sqrt(elapsed);
    let durable = u64::from(stability).saturating_add(100);
    u64::from(baseline_xp)
        .saturating_mul(durable)
        .saturating_div(durable.saturating_add(root.saturating_mul(10)).max(1))
        .min(u64::from(u32::MAX)) as u32
}

fn integer_sqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut low = 1_u64;
    let mut high = value.min(u64::from(u32::MAX)).saturating_add(1);
    while low.saturating_add(1) < high {
        let middle = low.saturating_add(high.saturating_sub(low) / 2);
        if middle <= value / middle {
            low = middle;
        } else {
            high = middle;
        }
    }
    low
}

pub fn practise_skill(
    skills: &mut Skills,
    consciousness: &Consciousness,
    phenotype: &Phenotype,
    skill: SkillId,
    base_xp: u32,
    tick: Tick,
    kind: PracticeKind,
) -> Result<(), CoreError> {
    let was_learned = skills.levels.get(&skill).is_some_and(|state| state.learned);
    let adjusted_xp = match kind {
        PracticeKind::Restudy => base_xp,
        PracticeKind::Retrieval => base_xp.saturating_mul(4) / 5,
    };
    let adjusted_xp = if was_learned {
        adjusted_xp.saturating_mul(2)
    } else {
        adjusted_xp
    };
    super::apply_learning(skills, consciousness, phenotype, skill, adjusted_xp)?;
    let previous_tick = skills
        .memories
        .get(&skill)
        .and_then(|memory| memory.last_practiced);
    let gap = previous_tick.map_or(20, |previous| {
        tick.0.saturating_sub(previous.0).min(u64::from(u32::MAX)) as u32
    });
    let spacing = u32::from(spacing_strength_gain(gap, 100));
    let stability_gain = match kind {
        PracticeKind::Restudy => spacing.saturating_mul(5) / 1000,
        PracticeKind::Retrieval => spacing.saturating_mul(80) / 1000,
    };
    let current_xp = skills.levels.get(&skill).map_or(0, |state| state.xp);
    let memory = skills.memories.entry(skill).or_default();
    memory.last_practiced = Some(tick);
    memory.stability = memory
        .stability
        .saturating_add(stability_gain.min(u32::from(u16::MAX)) as u16);
    memory.baseline_xp = current_xp;
    Ok(())
}

pub fn decay_unpractised(skills: &mut Skills, now: Tick) {
    let memories = skills
        .memories
        .iter()
        .map(|(skill, memory)| (*skill, memory.clone()))
        .collect::<Vec<_>>();
    for (skill, memory) in memories {
        let Some(last) = memory.last_practiced else {
            continue;
        };
        let retained = retained_experience(
            memory.baseline_xp,
            now.0.saturating_sub(last.0),
            memory.stability,
        );
        if let Some(state) = skills.levels.get_mut(&skill) {
            state.xp = state.xp.min(retained);
        }
    }
}

#[cfg(test)]
mod tests {
    //! Skill memory forgets quickly then slowly, while spacing, retrieval, and savings make knowledge durable.

    use super::*;
    use crate::{DiseaseStatus, EyeColor, Handedness, Permille, Phenotype, Sex, SkillState};

    fn mind() -> Consciousness {
        Consciousness {
            awareness: 100,
            focus: 100,
            memory_capacity: 1_000,
        }
    }

    fn phenotype() -> Phenotype {
        Phenotype {
            sex: Sex::Female,
            eye_color: EyeColor::Brown,
            handedness: Handedness::Right,
            disease_x: DiseaseStatus::Clear,
            robustness: 4,
            aptitude: 0,
            base_max_health: 100,
            learning_rate: Permille::ONE,
            lifespan_ticks: 2_400,
        }
    }

    fn remembering_skills() -> Skills {
        let mut skills = Skills::default();
        skills.levels.insert(
            SkillId::Recall,
            SkillState {
                xp: 100,
                learned: true,
            },
        );
        skills
    }

    #[test]
    fn forgetting_is_steep_at_first_and_then_flattens_instead_of_draining_linearly() {
        let early = 1_000 - retained_experience(1_000, 1, 100);
        let later_increment = retained_experience(1_000, 100, 100)
            .saturating_sub(retained_experience(1_000, 121, 100));
        assert!(early > later_increment);
    }

    #[test]
    fn greater_stability_preserves_more_experience_over_the_same_delay() {
        assert!(retained_experience(1_000, 100, 500) > retained_experience(1_000, 100, 20));
    }

    #[test]
    fn an_unpractised_skill_loses_experience_without_being_erased_from_memory() {
        let mut skills = remembering_skills();
        skills.levels.insert(
            SkillId::Motor,
            SkillState {
                xp: 500,
                learned: true,
            },
        );
        skills.memories.insert(
            SkillId::Motor,
            SkillMemory {
                last_practiced: Some(Tick(0)),
                stability: 20,
                baseline_xp: 500,
            },
        );
        decay_unpractised(&mut skills, Tick(100));
        assert!(skills.levels[&SkillId::Motor].xp < 500);
        assert!(skills.levels[&SkillId::Motor].learned);
    }

    #[test]
    fn spacing_near_a_fraction_of_the_needed_retention_interval_builds_most_strength() {
        let too_soon = spacing_strength_gain(2, 100);
        let near_optimal = spacing_strength_gain(20, 100);
        let late = spacing_strength_gain(40, 100);
        assert!(near_optimal > too_soon);
        assert!(near_optimal > late);
        assert!(late > too_soon);
    }

    #[test]
    fn equal_practice_spread_over_time_builds_more_stability_than_massed_practice() {
        let mut massed = remembering_skills();
        let mut spaced = remembering_skills();
        for tick in [1, 2, 3] {
            practise_skill(
                &mut massed,
                &mind(),
                &phenotype(),
                SkillId::Motor,
                100,
                Tick(tick),
                PracticeKind::Retrieval,
            )
            .expect("massed retrieval is available");
        }
        for tick in [1, 21, 41] {
            practise_skill(
                &mut spaced,
                &mind(),
                &phenotype(),
                SkillId::Motor,
                100,
                Tick(tick),
                PracticeKind::Retrieval,
            )
            .expect("spaced retrieval is available");
        }
        assert!(
            spaced.memories[&SkillId::Motor].stability > massed.memories[&SkillId::Motor].stability
        );
    }

    #[test]
    fn spaced_retrieval_loses_immediately_but_wins_after_a_long_delay() {
        let mut restudy = remembering_skills();
        let mut retrieval = remembering_skills();
        for tick in [1, 2, 3] {
            practise_skill(
                &mut restudy,
                &mind(),
                &phenotype(),
                SkillId::Motor,
                100,
                Tick(tick),
                PracticeKind::Restudy,
            )
            .expect("restudy is available");
        }
        for tick in [1, 21, 41] {
            practise_skill(
                &mut retrieval,
                &mind(),
                &phenotype(),
                SkillId::Motor,
                100,
                Tick(tick),
                PracticeKind::Retrieval,
            )
            .expect("retrieval is available");
        }
        assert!(restudy.levels[&SkillId::Motor].xp > retrieval.levels[&SkillId::Motor].xp);
        decay_unpractised(&mut restudy, Tick(241));
        decay_unpractised(&mut retrieval, Tick(241));
        assert!(retrieval.levels[&SkillId::Motor].xp > restudy.levels[&SkillId::Motor].xp);
    }

    #[test]
    fn relearning_something_once_known_is_cheaper_than_first_acquisition() {
        let mut novice = remembering_skills();
        let mut relearning = remembering_skills();
        relearning.levels.insert(
            SkillId::Motor,
            SkillState {
                xp: 0,
                learned: true,
            },
        );
        practise_skill(
            &mut novice,
            &mind(),
            &phenotype(),
            SkillId::Motor,
            60,
            Tick(1),
            PracticeKind::Retrieval,
        )
        .expect("novel learning is available");
        practise_skill(
            &mut relearning,
            &mind(),
            &phenotype(),
            SkillId::Motor,
            60,
            Tick(1),
            PracticeKind::Retrieval,
        )
        .expect("relearning is available");
        assert!(relearning.levels[&SkillId::Motor].xp > novice.levels[&SkillId::Motor].xp);
    }
}
