use std::collections::BTreeMap;

use anana_core::{
    Body, Consciousness, Genome, HumanId, HumanState, Infection, Instincts, Lineage, Phenotype,
    Skills, WorldSnapshot, world_hash,
};
use bevy::prelude::World;

use crate::{
    EventLog, Gods, HashHistory, NextHumanId, SimulationRng, SimulationStats, Viruses, WorldClock,
};

pub(crate) fn build_snapshot(world: &mut World) -> WorldSnapshot {
    let canonical = {
        let mut query = world.query::<(
            &HumanId,
            &Genome,
            &Phenotype,
            &Instincts,
            &Consciousness,
            &Body,
            &Skills,
            &Lineage,
            Option<&Infection>,
        )>();
        query
            .iter(world)
            .map(
                |(
                    id,
                    genome,
                    phenotype,
                    instincts,
                    consciousness,
                    body,
                    skills,
                    lineage,
                    infection,
                )| {
                    (
                        *id,
                        HumanState {
                            id: *id,
                            genome: genome.clone(),
                            phenotype: phenotype.clone(),
                            instincts: instincts.clone(),
                            consciousness: consciousness.clone(),
                            body: body.clone(),
                            skills: skills.clone(),
                            lineage: lineage.clone(),
                            infection: infection.cloned(),
                        },
                    )
                },
            )
            .collect::<BTreeMap<_, _>>()
    };
    WorldSnapshot {
        seed: world.resource::<SimulationRng>().0.master_seed,
        tick: world.resource::<WorldClock>().0,
        next_human_id: world.resource::<NextHumanId>().0,
        humans: canonical,
        viruses: world.resource::<Viruses>().0.clone(),
        gods: world.resource::<Gods>().0.clone(),
        event_log: world.resource::<EventLog>().records().to_vec(),
    }
}

pub(crate) fn logging_and_hash(world: &mut World) {
    let snapshot = build_snapshot(world);
    world.resource_mut::<SimulationStats>().living = snapshot.humans.len() as u64;
    world
        .resource_mut::<HashHistory>()
        .0
        .push(world_hash(&snapshot));
}
