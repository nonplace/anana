use anana_core::{Body, HumanId, Infection, InfectionPhase, LifeStage, Phenotype};
use bevy::prelude::{Entity, Query};

fn elapsed_permille(age_ticks: u32, lifespan_ticks: u32) -> u64 {
    if lifespan_ticks == 0 {
        1000
    } else {
        u64::from(age_ticks).saturating_mul(1000) / u64::from(lifespan_ticks)
    }
}

fn fertility_for(body: &Body, phenotype: &Phenotype) -> u8 {
    match body.life_stage {
        LifeStage::Infant | LifeStage::Child | LifeStage::Elder => 0,
        LifeStage::Adolescent => {
            let elapsed = elapsed_permille(body.age_ticks, phenotype.lifespan_ticks);
            let progress = elapsed.saturating_sub(200).min(149);
            ((progress.saturating_mul(100)) / 150).min(100) as u8
        }
        LifeStage::Adult => 100,
    }
}

pub(crate) fn aging_health(
    mut humans: Query<'_, '_, (Entity, &HumanId, &Phenotype, &mut Body, Option<&Infection>)>,
) {
    let mut ordered = humans
        .iter_mut()
        .map(|(entity, id, _, _, _)| (*id, entity))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(id, _)| *id);
    for (_, entity) in ordered {
        let Ok((_, _, phenotype, mut body, infection)) = humans.get_mut(entity) else {
            continue;
        };
        if !body.alive {
            continue;
        }
        body.age_ticks = body.age_ticks.saturating_add(1);
        body.life_stage = Body::life_stage_for(body.age_ticks, phenotype.lifespan_ticks);
        body.fertility = fertility_for(&body, phenotype);
        let elder_decay = u16::from(body.life_stage == LifeStage::Elder);
        let infection_decay = infection
            .filter(|infection| infection.phase == InfectionPhase::Infectious)
            .map_or(0, |infection| {
                u16::from(infection.severity).max(1).div_ceil(20)
            });
        body.health = body
            .health
            .saturating_sub(elder_decay.saturating_add(infection_decay))
            .min(body.max_health);
    }
}
