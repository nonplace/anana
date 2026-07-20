use anana_core::{Body, HumanId, Infection, InfectionPhase, Phenotype, RngDomain};
use bevy::prelude::{Entity, Query, Res};

use crate::{Config, SimulationRng, WorldClock};

fn elapsed_permille(age_ticks: u32, lifespan_ticks: u32) -> u64 {
    if lifespan_ticks == 0 {
        1000
    } else {
        u64::from(age_ticks).saturating_mul(1000) / u64::from(lifespan_ticks)
    }
}

pub(crate) fn fertility_for_age(age_ticks: u32, lifespan_ticks: u32) -> u8 {
    let elapsed = elapsed_permille(age_ticks, lifespan_ticks);
    match elapsed {
        0..=199 => 0,
        200..=349 => elapsed
            .saturating_sub(200)
            .saturating_mul(100)
            .div_ceil(150) as u8,
        350..=499 => 100,
        500..=699 => (700_u64.saturating_sub(elapsed).saturating_mul(100) / 200) as u8,
        _ => 0,
    }
}

pub(crate) fn mortality_for_age(age_ticks: u32, lifespan_ticks: u32) -> u16 {
    let elapsed = elapsed_permille(age_ticks, lifespan_ticks);
    match elapsed {
        0..=49 => 20,
        50..=499 => 2,
        500..=749 => (5_u64.saturating_add(elapsed.saturating_sub(500) * 95 / 250)) as u16,
        750..=999 => (100_u64.saturating_add(elapsed.saturating_sub(750) * 500 / 250)) as u16,
        _ => 1000,
    }
}

pub(crate) fn aging_health(
    config: Res<'_, Config>,
    clock: Res<'_, WorldClock>,
    rng: Res<'_, SimulationRng>,
    mut humans: Query<'_, '_, (Entity, &HumanId, &Phenotype, &mut Body, Option<&Infection>)>,
) {
    let mut ordered = humans
        .iter_mut()
        .map(|(entity, id, _, _, _)| (*id, entity))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(id, _)| *id);
    for (id, entity) in ordered {
        let Ok((_, _, phenotype, mut body, infection)) = humans.get_mut(entity) else {
            continue;
        };
        if !body.alive {
            continue;
        }
        body.age_ticks = body.age_ticks.saturating_add(1);
        body.life_stage = Body::life_stage_for(body.age_ticks, phenotype.lifespan_ticks);
        body.fertility = fertility_for_age(body.age_ticks, phenotype.lifespan_ticks);
        let infection_decay = infection
            .filter(|infection| infection.phase == InfectionPhase::Infectious)
            .map_or(0, |infection| {
                u16::from(infection.severity).max(1).div_ceil(20)
            });
        body.health = body
            .health
            .saturating_sub(infection_decay)
            .min(body.max_health);
        if infection_decay == 0 && clock.0.0.is_multiple_of(10) {
            body.health = body.health.saturating_add(1).min(body.max_health);
        }
        if config.mortality_interval > 0
            && clock.0.0.is_multiple_of(config.mortality_interval)
            && rng.0.gate(
                RngDomain::Mortality,
                clock.0,
                id,
                0,
                anana_core::Permille(mortality_for_age(body.age_ticks, phenotype.lifespan_ticks)),
            )
        {
            body.health = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    //! Fertility and mortality change gradually with age instead of switching at one life-stage cliff.

    use super::*;

    #[test]
    fn fertility_rises_through_adolescence_peaks_and_then_declines() {
        let lifespan = 2_400;
        let samples = [
            fertility_for_age(400, lifespan),
            fertility_for_age(600, lifespan),
            fertility_for_age(900, lifespan),
            fertility_for_age(1_500, lifespan),
            fertility_for_age(1_800, lifespan),
        ];
        assert!(samples[0] < samples[1]);
        assert!(samples[1] < samples[2]);
        assert!(samples[2] > samples[3]);
        assert_eq!(samples[4], 0);
    }

    #[test]
    fn mortality_is_elevated_in_infancy_low_in_youth_and_steep_in_old_age() {
        let lifespan = 2_400;
        let infant = mortality_for_age(20, lifespan);
        let youth = mortality_for_age(600, lifespan);
        let middle = mortality_for_age(1_200, lifespan);
        let old = mortality_for_age(2_000, lifespan);
        assert!(infant > youth);
        assert!(youth <= middle);
        assert!(old > middle.saturating_mul(10));
    }
}
