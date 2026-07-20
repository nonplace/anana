use anana_core::{
    Body, DeadHuman, DeterministicKind, EventAuthor, EventOutcome, EventPayload, HumanId, Lineage,
    Phenotype, Skills,
};
use bevy::prelude::{Commands, Entity, Query, Res, ResMut};

use crate::{DeadRegistry, EventLog, SimulationFaults, SimulationStats, WorldClock};

pub(crate) fn death(
    mut commands: Commands<'_, '_>,
    clock: Res<'_, WorldClock>,
    mut log: ResMut<'_, EventLog>,
    mut dead: ResMut<'_, DeadRegistry>,
    mut faults: ResMut<'_, SimulationFaults>,
    mut stats: ResMut<'_, SimulationStats>,
    humans: Query<'_, '_, (Entity, &HumanId, &Phenotype, &Body, &Lineage, &Skills)>,
) {
    let mut dying = humans
        .iter()
        .filter(|(_, _, phenotype, body, _, _)| {
            body.alive && (body.health == 0 || body.age_ticks >= phenotype.lifespan_ticks)
        })
        .map(|(entity, id, _, _, _, _)| (*id, entity))
        .collect::<Vec<_>>();
    dying.sort_by_key(|(id, _)| *id);
    for (id, entity) in dying {
        let Ok((_, _, _, _, lineage, skills)) = humans.get(entity) else {
            continue;
        };
        dead.0.insert(
            id,
            DeadHuman {
                id,
                lineage: lineage.clone(),
                generation: lineage.generation,
                birth_tick: lineage.birth_tick,
                death_tick: clock.0,
                skills: skills.clone(),
            },
        );
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
