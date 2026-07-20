use std::collections::{BTreeMap, BTreeSet};

use anana_core::{
    Body, Consciousness, Genome, HumanId, HumanState, Infection, Instincts, Lineage, Phenotype,
    Residence, Skills, SocialBonds, WorldSnapshot, extend_event_log_hash,
    world_hash_with_event_log_hash,
};
use bevy::prelude::World;

use crate::{
    Coalitions, DeadRegistry, EventDigest, EventLog, Gods, HashHistory, NextHumanId,
    NextResidenceId, PopulationHistory, PopulationPoint, SimulationRng, SimulationStats, Viruses,
    WorldClock,
};

fn living_humans(world: &mut World) -> BTreeMap<HumanId, HumanState> {
    let mut query = world.query::<(
        &HumanId,
        &Genome,
        &Phenotype,
        &Instincts,
        &Consciousness,
        &Body,
        &Skills,
        &Lineage,
        &Residence,
        &SocialBonds,
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
                residence,
                social_bonds,
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
                        residence: *residence,
                        social_bonds: social_bonds.clone(),
                        infection: infection.cloned(),
                    },
                )
            },
        )
        .collect()
}

fn snapshot_with_log(world: &mut World, include_log: bool) -> WorldSnapshot {
    let humans = living_humans(world);
    WorldSnapshot {
        seed: world.resource::<SimulationRng>().0.master_seed,
        tick: world.resource::<WorldClock>().0,
        next_human_id: world.resource::<NextHumanId>().0,
        next_residence_id: world.resource::<NextResidenceId>().0,
        humans,
        dead: world.resource::<DeadRegistry>().0.clone(),
        viruses: world.resource::<Viruses>().0.clone(),
        gods: world.resource::<Gods>().0.clone(),
        coalitions: world.resource::<Coalitions>().0.clone(),
        event_log: if include_log {
            world.resource::<EventLog>().records().to_vec()
        } else {
            Vec::new()
        },
    }
}

pub(crate) fn build_snapshot(world: &mut World) -> WorldSnapshot {
    snapshot_with_log(world, true)
}

fn surviving_founders(snapshot: &WorldSnapshot) -> u32 {
    let lineages = snapshot
        .humans
        .iter()
        .map(|(id, human)| (*id, human.lineage.clone()))
        .chain(
            snapshot
                .dead
                .iter()
                .map(|(id, human)| (*id, human.lineage.clone())),
        )
        .collect::<BTreeMap<_, _>>();
    let mut founders = BTreeSet::new();
    for living in snapshot.humans.keys() {
        let mut stack = vec![*living];
        let mut visited = BTreeSet::new();
        while let Some(id) = stack.pop() {
            if !visited.insert(id) {
                continue;
            }
            let Some(lineage) = lineages.get(&id) else {
                continue;
            };
            if lineage.generation == 0 || (lineage.mother.is_none() && lineage.father.is_none()) {
                founders.insert(id);
            } else {
                if let Some(mother) = lineage.mother {
                    stack.push(mother);
                }
                if let Some(father) = lineage.father {
                    stack.push(father);
                }
            }
        }
    }
    u32::try_from(founders.len()).map_or(u32::MAX, |count| count)
}

pub(crate) fn logging_and_hash(world: &mut World) {
    let record_count = world.resource::<EventLog>().records().len();
    let (previous_hash, previous_count) = {
        let digest = world.resource::<EventDigest>();
        (digest.hash, digest.records_hashed)
    };
    let next_hash = world
        .resource::<EventLog>()
        .records()
        .get(previous_count..)
        .map_or([0; 32], |records| {
            extend_event_log_hash(previous_hash, records)
        });
    {
        let mut digest = world.resource_mut::<EventDigest>();
        digest.hash = next_hash;
        digest.records_hashed = record_count;
    }

    let snapshot = snapshot_with_log(world, false);
    let living = snapshot.humans.len() as u64;
    let deepest_generation = snapshot
        .humans
        .values()
        .map(|human| human.lineage.generation)
        .chain(snapshot.dead.values().map(|human| human.generation))
        .max()
        .unwrap_or(0);
    let surviving_founder_lineages = surviving_founders(&snapshot);
    let point = {
        let mut stats = world.resource_mut::<SimulationStats>();
        stats.living = living;
        stats.deepest_generation = deepest_generation;
        stats.surviving_founder_lineages = surviving_founder_lineages;
        PopulationPoint {
            tick: snapshot.tick,
            living,
            births: stats.births,
            deaths: stats.deaths,
            deepest_generation,
            surviving_founder_lineages,
        }
    };
    world.resource_mut::<PopulationHistory>().0.push(point);
    world
        .resource_mut::<HashHistory>()
        .0
        .push(world_hash_with_event_log_hash(&snapshot, next_hash));
}
