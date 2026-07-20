use anana_core::{
    Body, HumanId, Instincts, LifeStage, Lineage, Permille, Phenotype, RngDomain, Sex,
};
use bevy::prelude::{Query, Res, ResMut};

use crate::{Config, PendingBirth, PendingBirths, SimulationRng, WorldClock};

#[derive(Clone)]
struct Candidate {
    id: HumanId,
    sex: Sex,
    fertility: u8,
    reproduction: u8,
    mother: Option<HumanId>,
    father: Option<HumanId>,
}

fn full_siblings(first: &Candidate, second: &Candidate) -> bool {
    first.mother.is_some()
        && first.father.is_some()
        && first.mother == second.mother
        && first.father == second.father
}

pub(crate) fn mating(
    config: Res<'_, Config>,
    clock: Res<'_, WorldClock>,
    rng: Res<'_, SimulationRng>,
    mut births: ResMut<'_, PendingBirths>,
    humans: Query<'_, '_, (&HumanId, &Phenotype, &Body, &Instincts, &Lineage)>,
) {
    if config.mating_interval == 0 || !clock.0.0.is_multiple_of(config.mating_interval) {
        return;
    }
    let living = humans
        .iter()
        .filter(|(_, _, body, _, _)| body.alive)
        .count() as u32;
    if living >= config.max_population {
        return;
    }
    let mut candidates = humans
        .iter()
        .filter(|(_, _, body, _, _)| {
            body.alive
                && body.life_stage == LifeStage::Adult
                && body.fertility > 0
                && body.health > body.max_health / 2
        })
        .map(|(id, phenotype, body, instincts, lineage)| Candidate {
            id: *id,
            sex: phenotype.sex,
            fertility: body.fertility.min(100),
            reproduction: instincts.reproduction.min(100),
            mother: lineage.mother,
            father: lineage.father,
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| candidate.id);
    let females = candidates
        .iter()
        .filter(|candidate| candidate.sex == Sex::Female)
        .collect::<Vec<_>>();
    let males = candidates
        .iter()
        .filter(|candidate| candidate.sex == Sex::Male)
        .collect::<Vec<_>>();
    let mut planned = 0_u32;
    for (mother, father) in females.into_iter().zip(males) {
        if mother.id == father.id
            || full_siblings(mother, father)
            || living.saturating_add(planned) >= config.max_population
        {
            continue;
        }
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
        let probability = fertility.and(reproduction);
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
