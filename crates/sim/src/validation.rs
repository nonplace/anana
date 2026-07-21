use std::collections::{BTreeMap, BTreeSet};

use anana_core::{HumanId, Lineage, SkillId, WorldSnapshot};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ValidationSample {
    pub tick: anana_core::Tick,
    pub living: u64,
    pub total_lived: u64,
    pub adopters: u64,
    pub living_adopters: u64,
    pub deepest_generation: u32,
    pub surviving_founder_lineages: u32,
    pub maximum_relationships: u64,
    pub people_at_capacity: u64,
    pub active_relationships: u64,
    pub inner_layer_relationships: u64,
    pub inner_layer_effort: u64,
    pub outer_layer_effort: u64,
}

fn has_adopted(skills: &anana_core::Skills, skill: SkillId) -> bool {
    skills.level_of(skill) >= 1
}

fn surviving_founder_lineages(snapshot: &WorldSnapshot) -> u32 {
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
        .collect::<BTreeMap<HumanId, Lineage>>();
    let mut founders = BTreeSet::new();
    for living in snapshot.humans.keys() {
        let mut visited = BTreeSet::new();
        let mut pending = vec![*living];
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            let Some(lineage) = lineages.get(&id) else {
                continue;
            };
            if lineage.generation == 0 {
                founders.insert(id);
            } else {
                pending.extend(lineage.mother);
                pending.extend(lineage.father);
            }
        }
    }
    u32::try_from(founders.len()).unwrap_or(u32::MAX)
}

/// Summarises only canonical public state for independent validation and reporting.
#[must_use]
pub fn validation_sample(snapshot: &WorldSnapshot, skill: SkillId) -> ValidationSample {
    let living_adopters = snapshot
        .humans
        .values()
        .filter(|human| has_adopted(&human.skills, skill))
        .count() as u64;
    let dead_adopters = snapshot
        .dead
        .values()
        .filter(|human| has_adopted(&human.skills, skill))
        .count() as u64;
    let mut relationship_counts = Vec::new();
    let mut active_relationships = 0_u64;
    let mut inner_layer_relationships = 0_u64;
    let mut inner_effort = 0_u64;
    let mut outer_effort = 0_u64;
    let mut inner_count = 0_u64;
    let mut outer_count = 0_u64;
    for human in snapshot.humans.values() {
        let mut bonds = human.social_bonds.bonds.iter().collect::<Vec<_>>();
        bonds.sort_by_key(|(id, bond)| (std::cmp::Reverse(bond.strength), **id));
        relationship_counts.push(bonds.len() as u64);
        active_relationships = active_relationships.saturating_add(bonds.len() as u64);
        inner_layer_relationships =
            inner_layer_relationships.saturating_add(bonds.len().min(5) as u64);
        for (rank, (_, bond)) in bonds.into_iter().enumerate() {
            if rank < 5 {
                inner_count = inner_count.saturating_add(1);
                inner_effort = inner_effort.saturating_add(u64::from(bond.positive_interactions));
            } else {
                outer_count = outer_count.saturating_add(1);
                outer_effort = outer_effort.saturating_add(u64::from(bond.positive_interactions));
            }
        }
    }
    ValidationSample {
        tick: snapshot.tick,
        living: snapshot.humans.len() as u64,
        total_lived: snapshot.humans.len().saturating_add(snapshot.dead.len()) as u64,
        adopters: living_adopters.saturating_add(dead_adopters),
        living_adopters,
        deepest_generation: snapshot
            .humans
            .values()
            .map(|human| human.lineage.generation)
            .chain(snapshot.dead.values().map(|human| human.generation))
            .max()
            .unwrap_or(0),
        surviving_founder_lineages: surviving_founder_lineages(snapshot),
        maximum_relationships: relationship_counts.iter().copied().max().unwrap_or(0),
        people_at_capacity: relationship_counts
            .iter()
            .filter(|count| **count == 150)
            .count() as u64,
        active_relationships,
        inner_layer_relationships,
        inner_layer_effort: inner_effort / inner_count.max(1),
        outer_layer_effort: outer_effort / outer_count.max(1),
    }
}
