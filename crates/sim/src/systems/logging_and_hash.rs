use std::collections::BTreeMap;

use anana_core::{
    Body, Consciousness, Genome, HumanId, HumanState, Infection, Instincts, Lineage, Phenotype,
    Skills, WorldSnapshot, world_hash,
};
use bevy::prelude::{Query, Res, ResMut};

use crate::{
    EventLog, Gods, HashHistory, NextHumanId, SimulationRng, SimulationStats, Viruses, WorldClock,
};

type HashResources<'w> = (
    Res<'w, WorldClock>,
    Res<'w, SimulationRng>,
    Res<'w, NextHumanId>,
    Res<'w, Viruses>,
    Res<'w, Gods>,
    Res<'w, EventLog>,
    ResMut<'w, HashHistory>,
    ResMut<'w, SimulationStats>,
);

type SnapshotQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static HumanId,
        &'static Genome,
        &'static Phenotype,
        &'static Instincts,
        &'static Consciousness,
        &'static Body,
        &'static Skills,
        &'static Lineage,
        Option<&'static Infection>,
    ),
>;

pub(crate) fn logging_and_hash(resources: HashResources<'_>, humans: SnapshotQuery<'_, '_>) {
    let (clock, rng, next_id, viruses, gods, log, mut hashes, mut stats) = resources;
    let canonical = humans
        .iter()
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
        .collect::<BTreeMap<_, _>>();
    stats.living = canonical.len() as u64;
    let snapshot = WorldSnapshot {
        seed: rng.0.master_seed,
        tick: clock.0,
        next_human_id: next_id.0,
        humans: canonical,
        viruses: viruses.0.clone(),
        gods: gods.0.clone(),
        event_log: log.records().to_vec(),
    };
    hashes.0.push(world_hash(&snapshot));
}
