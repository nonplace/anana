use crate::{Consciousness, CoreError, Phenotype, SkillId, Skills};

#[must_use]
pub const fn min_awareness(id: SkillId) -> u8 {
    match id {
        SkillId::Recall => 5,
        SkillId::Motor => 10,
        SkillId::Language | SkillId::Foraging => 20,
        SkillId::SocialBond => 25,
        SkillId::ToolUse => 35,
        SkillId::Farming => 45,
        SkillId::Medicine => 55,
        SkillId::Planning => 65,
    }
}

pub fn full_learning_gain(
    consciousness: &Consciousness,
    phenotype: &Phenotype,
    id: SkillId,
    base_xp: u32,
) -> Result<u32, CoreError> {
    if consciousness.awareness < min_awareness(id) {
        return Err(CoreError::SkillLocked(id));
    }
    let learning_rate = u64::from(phenotype.learning_rate.0.min(1000));
    let focus = u64::from(consciousness.focus.min(100));
    let aptitude_factor =
        1000_u64.saturating_add(u64::from(phenotype.aptitude.min(8)).saturating_mul(50));
    let gain = u64::from(base_xp)
        .saturating_mul(learning_rate)
        .saturating_mul(focus)
        .saturating_mul(aptitude_factor)
        / 100_000_000;
    Ok(gain.min(u64::from(u32::MAX)) as u32)
}

pub fn learning_gain(
    skills: &Skills,
    consciousness: &Consciousness,
    phenotype: &Phenotype,
    id: SkillId,
    base_xp: u32,
) -> Result<u32, CoreError> {
    let full = full_learning_gain(consciousness, phenotype, id, base_xp)?;
    if id != SkillId::Recall && !skills.recall_learned() {
        Ok(full / 2)
    } else {
        Ok(full)
    }
}

pub fn apply_learning(
    skills: &mut Skills,
    consciousness: &Consciousness,
    phenotype: &Phenotype,
    id: SkillId,
    base_xp: u32,
) -> Result<(), CoreError> {
    let gain = learning_gain(skills, consciousness, phenotype, id, base_xp)?;
    apply_calculated_learning(skills, consciousness, id, gain)
}

/// Applies a gain already scaled by the learning model while preserving the awareness and Recall gates.
pub fn apply_calculated_learning(
    skills: &mut Skills,
    consciousness: &Consciousness,
    id: SkillId,
    gain: u32,
) -> Result<(), CoreError> {
    if consciousness.awareness < min_awareness(id) {
        return Err(CoreError::SkillLocked(id));
    }
    let recall_was_learned = skills.recall_learned();
    let state = skills.levels.entry(id).or_default();
    if id != SkillId::Recall && !recall_was_learned {
        state.xp = state.xp.saturating_mul(9) / 10;
    }
    state.xp = state.xp.saturating_add(gain);
    state.learned |= (id == SkillId::Recall || recall_was_learned) && state.xp >= 100;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Recall turns fading experience into durable learning while awareness and focus bound the mind.

    use super::*;
    use crate::{
        Consciousness, CoreError, DiseaseStatus, EyeColor, Handedness, Permille, Phenotype, Sex,
        SkillId, SkillState, Skills,
    };

    fn consciousness(awareness: u8, focus: u8) -> Consciousness {
        Consciousness {
            awareness,
            focus,
            memory_capacity: 1_000,
        }
    }

    fn phenotype(aptitude: u8, learning_rate: u16) -> Phenotype {
        Phenotype {
            sex: Sex::Female,
            eye_color: EyeColor::Brown,
            handedness: Handedness::Right,
            disease_x: DiseaseStatus::Clear,
            robustness: 4,
            aptitude,
            base_max_health: 100,
            learning_rate: Permille(learning_rate),
            lifespan_ticks: 22_000,
        }
    }

    #[test]
    fn recall_has_a_strictly_lower_awareness_gate_than_every_other_skill() {
        let recall_gate = min_awareness(SkillId::Recall);
        assert_eq!(recall_gate, 5);
        for skill in [
            SkillId::Motor,
            SkillId::Language,
            SkillId::Foraging,
            SkillId::ToolUse,
            SkillId::SocialBond,
            SkillId::Farming,
            SkillId::Medicine,
            SkillId::Planning,
        ] {
            assert!(recall_gate < min_awareness(skill));
        }
    }

    #[test]
    fn a_locked_learning_attempt_leaves_the_skill_map_untouched() {
        let mut skills = Skills::default();
        skills.levels.insert(
            SkillId::Motor,
            SkillState {
                xp: 44,
                learned: false,
            },
        );
        let before = skills.clone();
        assert_eq!(
            apply_learning(
                &mut skills,
                &consciousness(9, 100),
                &phenotype(0, 1000),
                SkillId::Motor,
                100,
            ),
            Err(CoreError::SkillLocked(SkillId::Motor))
        );
        assert_eq!(skills, before);
    }

    #[test]
    fn non_recall_experience_decays_until_recall_is_learned() {
        let mut skills = Skills::default();
        let mind = consciousness(100, 100);
        let body = phenotype(0, 1000);
        apply_learning(&mut skills, &mind, &body, SkillId::Motor, 100).expect("motor is unlocked");
        assert_eq!(skills.levels[&SkillId::Motor].xp, 50);
        apply_learning(&mut skills, &mind, &body, SkillId::Motor, 100).expect("motor is unlocked");
        assert_eq!(skills.levels[&SkillId::Motor].xp, 95);
        apply_learning(&mut skills, &mind, &body, SkillId::Motor, 100).expect("motor is unlocked");
        assert_eq!(skills.levels[&SkillId::Motor].xp, 135);
    }

    #[test]
    fn no_non_recall_skill_can_latch_before_recall() {
        let mut skills = Skills::default();
        apply_learning(
            &mut skills,
            &consciousness(100, 100),
            &phenotype(0, 1000),
            SkillId::Motor,
            10_000,
        )
        .expect("motor is unlocked");
        assert!(!skills.levels[&SkillId::Motor].learned);
    }

    #[test]
    fn learning_recall_stops_decay_and_allows_experience_to_compound() {
        let mut skills = Skills::default();
        let mind = consciousness(100, 100);
        let body = phenotype(0, 1000);
        apply_learning(&mut skills, &mind, &body, SkillId::Motor, 200).expect("motor is unlocked");
        assert_eq!(skills.levels[&SkillId::Motor].xp, 100);
        apply_learning(&mut skills, &mind, &body, SkillId::Recall, 100)
            .expect("recall is unlocked");
        assert!(skills.recall_learned());
        apply_learning(&mut skills, &mind, &body, SkillId::Motor, 100).expect("motor is unlocked");
        assert_eq!(skills.levels[&SkillId::Motor].xp, 200);
        assert!(skills.levels[&SkillId::Motor].learned);
        apply_learning(&mut skills, &mind, &body, SkillId::Motor, 100).expect("motor is unlocked");
        assert_eq!(skills.levels[&SkillId::Motor].xp, 300);
    }

    #[test]
    fn aptitude_and_focus_scale_full_learning_by_exact_integer_math() {
        assert_eq!(
            full_learning_gain(
                &consciousness(100, 50),
                &phenotype(4, 800),
                SkillId::Motor,
                1000,
            ),
            Ok(480)
        );
    }

    #[test]
    fn a_precalculated_event_gain_still_obeys_awareness_and_recall() {
        let mut skills = Skills::default();
        let before = skills.clone();
        assert_eq!(
            apply_calculated_learning(&mut skills, &consciousness(9, 100), SkillId::Motor, 100),
            Err(CoreError::SkillLocked(SkillId::Motor))
        );
        assert_eq!(skills, before);
        apply_calculated_learning(&mut skills, &consciousness(100, 100), SkillId::Motor, 100)
            .expect("motor is available");
        assert_eq!(skills.levels[&SkillId::Motor].xp, 100);
        assert!(!skills.levels[&SkillId::Motor].learned);
    }
}
