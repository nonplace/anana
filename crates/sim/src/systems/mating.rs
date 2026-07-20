use std::collections::{BTreeMap, BTreeSet};

use anana_core::{
    Body, Consciousness, HumanId, Instincts, Lineage, MateProfile, Permille, Phenotype, RngDomain,
    Sex, SkillId, Skills, SocialBonds, Tick, are_first_degree_relatives, attraction_score,
    courtship_aversion_factor, mutual_courtship_is_ready,
};
use bevy::prelude::{Query, Res, ResMut};

use crate::{Config, PendingBirth, PendingBirths, SimulationRng, WorldClock};

#[derive(Clone)]
struct Candidate {
    id: HumanId,
    sex: Sex,
    fertility: u8,
    reproduction: u8,
    last_birth_tick: Option<Tick>,
    lineage: Lineage,
    profile: MateProfile,
    social_bonds: SocialBonds,
}

fn age_permille(body: &Body, phenotype: &Phenotype) -> u16 {
    if phenotype.lifespan_ticks == 0 {
        return 1000;
    }
    u64::from(body.age_ticks)
        .saturating_mul(1000)
        .saturating_div(u64::from(phenotype.lifespan_ticks))
        .min(1000) as u16
}

fn mate_profile(
    body: &Body,
    phenotype: &Phenotype,
    instincts: &Instincts,
    consciousness: &Consciousness,
    skills: &Skills,
) -> MateProfile {
    let values = u16::from(instincts.social.min(100))
        .saturating_add(u16::from(instincts.reproduction.min(100)))
        / 2;
    let cognition = u16::from(phenotype.aptitude.min(8))
        .saturating_mul(10)
        .saturating_add(u16::from(skills.level_of(SkillId::Language)).saturating_mul(4))
        .min(100);
    let body_quality = u16::from(phenotype.robustness.min(8))
        .saturating_mul(12)
        .min(100);
    let health_quality = if body.max_health == 0 {
        0
    } else {
        u32::from(body.health)
            .saturating_mul(60)
            .saturating_div(u32::from(body.max_health))
            .min(60) as u16
    };
    let competence = u16::from(skills.level_of(SkillId::SocialBond))
        .saturating_add(u16::from(skills.level_of(SkillId::ToolUse)))
        .saturating_mul(4)
        .min(40);
    MateProfile {
        age_permille: age_permille(body, phenotype),
        values: values.min(100) as u8,
        cognition: cognition as u8,
        body: body_quality as u8,
        temperament: consciousness
            .focus
            .min(100)
            .saturating_add(100_u8.saturating_sub(instincts.fear.min(100)))
            / 2,
        desirability: health_quality.saturating_add(competence).min(100) as u8,
    }
}

fn bond_between<'a>(first: &'a Candidate, second: &Candidate) -> Option<&'a anana_core::Bond> {
    first.social_bonds.bonds.get(&second.id)
}

fn pair_qualifies(first: &Candidate, second: &Candidate) -> bool {
    if are_first_degree_relatives(&first.lineage, &second.lineage) {
        return false;
    }
    let (Some(first_bond), Some(second_bond)) =
        (bond_between(first, second), bond_between(second, first))
    else {
        return false;
    };
    mutual_courtship_is_ready(first_bond, second_bond)
}

fn preference_key(
    chooser: &Candidate,
    candidate: &Candidate,
    rng: &anana_core::Rng,
    tick: Tick,
) -> (u16, u16, u64, std::cmp::Reverse<HumanId>) {
    let bond = bond_between(chooser, candidate).map_or(0, |bond| bond.strength.0.min(1000));
    let choice_noise = rng
        .draw_u64(RngDomain::Mating, tick, chooser.id, candidate.id.0)
        .checked_rem(200)
        .map_or(0, |value| value as u16);
    (
        attraction_score(&chooser.profile, &candidate.profile).saturating_add(choice_noise),
        bond,
        rng.draw_u64(RngDomain::Mating, tick, chooser.id, candidate.id.0),
        std::cmp::Reverse(candidate.id),
    )
}

fn mutual_matches(
    mothers: &[Candidate],
    fathers: &[Candidate],
    rng: &anana_core::Rng,
    tick: Tick,
) -> Vec<(HumanId, HumanId)> {
    let mother_by_id = mothers
        .iter()
        .map(|mother| (mother.id, mother))
        .collect::<BTreeMap<_, _>>();
    let father_by_id = fathers
        .iter()
        .map(|father| (father.id, father))
        .collect::<BTreeMap<_, _>>();
    let mut remaining_mothers = mother_by_id.keys().copied().collect::<BTreeSet<_>>();
    let mut remaining_fathers = father_by_id.keys().copied().collect::<BTreeSet<_>>();
    let mut matches = Vec::new();
    while !remaining_mothers.is_empty() && !remaining_fathers.is_empty() {
        let mut proposals = BTreeMap::<HumanId, Vec<HumanId>>::new();
        for mother_id in &remaining_mothers {
            let Some(mother) = mother_by_id.get(mother_id).copied() else {
                continue;
            };
            let choice = remaining_fathers
                .iter()
                .filter_map(|father_id| father_by_id.get(father_id).copied())
                .filter(|father| pair_qualifies(mother, father))
                .max_by_key(|father| preference_key(mother, father, rng, tick));
            if let Some(father) = choice {
                proposals.entry(father.id).or_default().push(mother.id);
            }
        }
        if proposals.is_empty() {
            break;
        }
        let mut matched_any = false;
        for (father_id, proposing_mothers) in proposals {
            let Some(father) = father_by_id.get(&father_id).copied() else {
                continue;
            };
            let accepted = proposing_mothers
                .iter()
                .filter_map(|mother_id| mother_by_id.get(mother_id).copied())
                .max_by_key(|mother| preference_key(father, mother, rng, tick));
            if let Some(mother) = accepted {
                matches.push((mother.id, father.id));
                remaining_mothers.remove(&mother.id);
                remaining_fathers.remove(&father.id);
                matched_any = true;
            }
        }
        if !matched_any {
            break;
        }
    }
    matches.sort_unstable();
    matches
}

pub(crate) fn density_birth_factor(living: u32, carrying_capacity: u32) -> Permille {
    if carrying_capacity == 0 || living >= carrying_capacity {
        return Permille::ZERO;
    }
    let occupied = u64::from(living).saturating_mul(1000) / u64::from(carrying_capacity);
    let open = Permille::clamp1000(1000_i64.saturating_sub(occupied as i64));
    open.and(Permille(800))
}

pub(crate) fn mother_has_recovered(
    last_birth_tick: Option<Tick>,
    now: Tick,
    spacing_ticks: u64,
) -> bool {
    last_birth_tick.is_none_or(|last| now.0.saturating_sub(last.0) >= spacing_ticks)
}

type MatingQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static HumanId,
        &'static Phenotype,
        &'static Body,
        &'static Instincts,
        &'static Consciousness,
        &'static Skills,
        &'static Lineage,
        &'static SocialBonds,
    ),
>;

pub(crate) fn mating(
    config: Res<'_, Config>,
    clock: Res<'_, WorldClock>,
    rng: Res<'_, SimulationRng>,
    mut births: ResMut<'_, PendingBirths>,
    humans: MatingQuery<'_, '_>,
) {
    if config.mating_interval == 0 || !clock.0.0.is_multiple_of(config.mating_interval) {
        return;
    }
    let living = humans
        .iter()
        .filter(|(_, _, body, _, _, _, _, _)| body.alive)
        .count() as u32;
    let mut candidates = humans
        .iter()
        .filter(|(_, _, body, _, _, _, _, _)| {
            body.alive && body.fertility > 0 && body.health > body.max_health / 2
        })
        .map(
            |(id, phenotype, body, instincts, consciousness, skills, lineage, social_bonds)| {
                Candidate {
                    id: *id,
                    sex: phenotype.sex,
                    fertility: body.fertility.min(100),
                    reproduction: instincts.reproduction.min(100),
                    last_birth_tick: lineage.last_birth_tick,
                    lineage: lineage.clone(),
                    profile: mate_profile(body, phenotype, instincts, consciousness, skills),
                    social_bonds: social_bonds.clone(),
                }
            },
        )
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| candidate.id);
    let mothers = candidates
        .iter()
        .filter(|candidate| {
            candidate.sex == Sex::Female
                && mother_has_recovered(
                    candidate.last_birth_tick,
                    clock.0,
                    config.birth_spacing_ticks,
                )
        })
        .cloned()
        .collect::<Vec<_>>();
    let fathers = candidates
        .iter()
        .filter(|candidate| candidate.sex == Sex::Male)
        .cloned()
        .collect::<Vec<_>>();
    let mother_by_id = mothers
        .iter()
        .map(|mother| (mother.id, mother))
        .collect::<BTreeMap<_, _>>();
    let father_by_id = fathers
        .iter()
        .map(|father| (father.id, father))
        .collect::<BTreeMap<_, _>>();
    let mut planned = 0_u32;
    for (mother_id, father_id) in mutual_matches(&mothers, &fathers, &rng.0, clock.0) {
        let (Some(mother), Some(father)) =
            (mother_by_id.get(&mother_id), father_by_id.get(&father_id))
        else {
            continue;
        };
        let fertility = Permille(
            u16::from(mother.fertility)
                .saturating_add(u16::from(father.fertility))
                .saturating_mul(5)
                .min(1000),
        );
        let reproduction = Permille(
            u16::from(mother.reproduction)
                .saturating_add(u16::from(father.reproduction))
                .saturating_mul(5)
                .min(1000),
        );
        let density =
            density_birth_factor(living.saturating_add(planned), config.carrying_capacity);
        let mother_aversion = mother
            .social_bonds
            .rearing_aversions
            .get(&father.id)
            .map_or(Permille::ZERO, anana_core::RearingAversion::strength);
        let father_aversion = father
            .social_bonds
            .rearing_aversions
            .get(&mother.id)
            .map_or(Permille::ZERO, anana_core::RearingAversion::strength);
        let probability = fertility
            .and(reproduction)
            .and(density)
            .and(courtship_aversion_factor(mother_aversion, father_aversion));
        if rng.0.gate(
            RngDomain::Mating,
            clock.0,
            mother.id,
            father.id.0,
            probability,
        ) {
            births.0.push(PendingBirth {
                mother: mother.id,
                father: father.id,
            });
            planned = planned.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    //! Conception is smoothly density-dependent and a mother recovers between births.

    use super::*;

    #[test]
    fn birth_pressure_falls_smoothly_as_the_population_approaches_capacity() {
        let open = density_birth_factor(50, 300);
        let middle = density_birth_factor(150, 300);
        let crowded = density_birth_factor(290, 300);
        assert!(open > middle);
        assert!(middle > crowded);
        assert!(crowded > Permille::ZERO);
        assert_eq!(density_birth_factor(300, 300), Permille::ZERO);
    }

    #[test]
    fn a_recent_birth_enforces_a_recovery_interval_without_ending_fertility() {
        assert!(!mother_has_recovered(
            Some(anana_core::Tick(100)),
            anana_core::Tick(120),
            40
        ));
        assert!(mother_has_recovered(
            Some(anana_core::Tick(100)),
            anana_core::Tick(140),
            40
        ));
        assert!(mother_has_recovered(None, anana_core::Tick(1), 40));
    }
}
