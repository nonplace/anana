use anana_core::{
    Body, DeterministicKind, EventAuthor, EventOutcome, EventPayload, HumanId, Phenotype,
};
use bevy::prelude::{Commands, Entity, Query, Res, ResMut};

use crate::{EventLog, SimulationFaults, SimulationStats, WorldClock};

pub(crate) fn death(
    mut commands: Commands<'_, '_>,
    clock: Res<'_, WorldClock>,
    mut log: ResMut<'_, EventLog>,
    mut faults: ResMut<'_, SimulationFaults>,
    mut stats: ResMut<'_, SimulationStats>,
    humans: Query<'_, '_, (Entity, &HumanId, &Phenotype, &Body)>,
) {
    let mut dying = humans
        .iter()
        .filter(|(_, _, phenotype, body)| {
            body.alive && (body.health == 0 || body.age_ticks >= phenotype.lifespan_ticks)
        })
        .map(|(entity, id, _, _)| (*id, entity))
        .collect::<Vec<_>>();
    dying.sort_by_key(|(id, _)| *id);
    for (id, entity) in dying {
        if let Err(error) = log.append(
            clock.0,
            EventAuthor::Engine,
            vec![id],
            EventPayload::Deterministic(DeterministicKind::HealthTick),
            EventOutcome::NoOp,
        ) {
            faults.0.push(error);
        }
        stats.deaths = stats.deaths.saturating_add(1);
        commands.entity(entity).despawn();
    }
}
