use anana_core::{Body, HumanId, Instincts, Lineage, Permille, Phenotype, RngDomain, Sex, Tick};
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
    last_birth_tick: Option<Tick>,
}

fn full_siblings(first: &Candidate, second: &Candidate) -> bool {
    first.mother.is_some()
        && first.father.is_some()
        && first.mother == second.mother
        && first.father == second.father
}

pub(crate) fn density_birth_factor(living: u32, carrying_capacity: u32) -> Permille {
    if carrying_capacity == 0 || living >= carrying_capacity {
        return Permille::ZERO;
    }
    let occupied = u64::from(living).saturating_mul(1000) / u64::from(carrying_capacity);
    let open = Permille::clamp1000(1000_i64.saturating_sub(occupied as i64));
    open.and(open)
}

pub(crate) fn mother_has_recovered(
    last_birth_tick: Option<Tick>,
    now: Tick,
    spacing_ticks: u64,
) -> bool {
    last_birth_tick.is_none_or(|last| now.0.saturating_sub(last.0) >= spacing_ticks)
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
    let mut candidates = humans
        .iter()
        .filter(|(_, _, body, _, _)| {
            body.alive && body.fertility > 0 && body.health > body.max_health / 2
        })
        .map(|(id, phenotype, body, instincts, lineage)| Candidate {
            id: *id,
            sex: phenotype.sex,
            fertility: body.fertility.min(100),
            reproduction: instincts.reproduction.min(100),
            mother: lineage.mother,
            father: lineage.father,
            last_birth_tick: lineage.last_birth_tick,
        })
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
        .collect::<Vec<_>>();
    let fathers = candidates
        .iter()
        .filter(|candidate| candidate.sex == Sex::Male)
        .collect::<Vec<_>>();
    let mut planned = 0_u32;
    for mother in mothers {
        let mut eligible_fathers = fathers
            .iter()
            .copied()
            .filter(|father| mother.id != father.id && !full_siblings(mother, father))
            .collect::<Vec<_>>();
        eligible_fathers.sort_by_key(|father| {
            (
                rng.0
                    .draw_u64(RngDomain::Mating, clock.0, mother.id, father.id.0),
                father.id,
            )
        });
        let Some(father) = eligible_fathers.first().copied() else {
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
        let probability = fertility.and(reproduction).and(density);
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
