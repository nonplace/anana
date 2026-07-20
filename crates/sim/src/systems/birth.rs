use std::collections::BTreeMap;

use anana_core::{
    Body, Consciousness, DeterministicKind, EffectSummary, EventAuthor, EventOutcome, EventPayload,
    Genome, HumanId, Instincts, Lineage, Phenotype, RearingAversion, Residence, Skills,
    SocialBonds, conceive, express,
};
use bevy::prelude::{Commands, Entity, Query, Res, ResMut};

use crate::{
    EventLog, NextHumanId, PendingBirths, SimulationFaults, SimulationRng, SimulationStats,
    WorldClock,
};

type BirthResources<'w, 's> = (
    Commands<'w, 's>,
    Res<'w, WorldClock>,
    Res<'w, SimulationRng>,
    ResMut<'w, PendingBirths>,
    ResMut<'w, NextHumanId>,
    ResMut<'w, EventLog>,
    ResMut<'w, SimulationFaults>,
    ResMut<'w, SimulationStats>,
);

type ParentQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static HumanId,
        &'static Genome,
        &'static Instincts,
        &'static mut Lineage,
        &'static Residence,
        &'static mut SocialBonds,
    ),
>;

#[derive(Clone)]
struct ParentSnapshot {
    entity: Entity,
    genome: Genome,
    instincts: Instincts,
    generation: u32,
    residence: Residence,
}

pub(crate) struct Newborn {
    pub id: HumanId,
    pub genome: Genome,
    pub phenotype: Phenotype,
    pub instincts: Instincts,
    pub consciousness: Consciousness,
    pub skills: Skills,
    pub lineage: Lineage,
    pub residence: Residence,
    pub social_bonds: SocialBonds,
}

pub(crate) fn spawn_newborn(commands: &mut Commands<'_, '_>, newborn: Newborn) {
    let body = Body::at_birth(&newborn.phenotype);
    commands.spawn((
        newborn.id,
        newborn.genome,
        newborn.phenotype,
        newborn.instincts,
        newborn.consciousness,
        body,
        newborn.skills,
        newborn.lineage,
        newborn.residence,
        newborn.social_bonds,
    ));
}

fn midpoint(left: u8, right: u8) -> u8 {
    u16::from(left)
        .saturating_add(u16::from(right))
        .div_ceil(2)
        .min(100) as u8
}

fn inherited_instincts(mother: &Instincts, father: &Instincts) -> Instincts {
    Instincts {
        survival: midpoint(mother.survival, father.survival),
        reproduction: midpoint(mother.reproduction, father.reproduction),
        hunger: midpoint(mother.hunger, father.hunger),
        fear: midpoint(mother.fear, father.fear),
        social: midpoint(mother.social, father.social),
    }
}

pub(crate) fn birth(resources: BirthResources<'_, '_>, mut humans: ParentQuery<'_, '_>) {
    let (mut commands, clock, rng, mut pending, mut next_id, mut log, mut faults, mut stats) =
        resources;
    let parents = humans
        .iter_mut()
        .map(|(entity, id, genome, instincts, lineage, residence, _)| {
            (
                *id,
                ParentSnapshot {
                    entity,
                    genome: genome.clone(),
                    instincts: instincts.clone(),
                    generation: lineage.generation,
                    residence: *residence,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let births = std::mem::take(&mut pending.0);
    for pending_birth in births {
        let (Some(mother), Some(father)) = (
            parents.get(&pending_birth.mother).cloned(),
            parents.get(&pending_birth.father).cloned(),
        ) else {
            continue;
        };
        let child_id = match next_id.allocate() {
            Ok(id) => id,
            Err(error) => {
                faults.0.push(error);
                continue;
            }
        };
        let genome = conceive(&mother.genome, &father.genome, &rng.0, clock.0, child_id);
        let phenotype = express(&genome, &rng.0, clock.0, child_id);
        let instincts = inherited_instincts(&mother.instincts, &father.instincts);
        let generation = mother.generation.max(father.generation).saturating_add(1);
        let lineage = Lineage::new(
            child_id,
            Some(pending_birth.mother),
            Some(pending_birth.father),
            generation,
            clock.0,
        );
        if let Ok((_, _, _, _, mut lineage, _, _)) = humans.get_mut(mother.entity) {
            lineage.children.push(child_id);
            lineage.last_birth_tick = Some(clock.0);
        }
        if let Ok((_, _, _, _, mut lineage, _, _)) = humans.get_mut(father.entity) {
            lineage.children.push(child_id);
        }
        for (_, existing_id, _, _, _, residence, mut social_bonds) in &mut humans {
            if *existing_id != child_id && *residence == mother.residence {
                social_bonds
                    .rearing_aversions
                    .insert(child_id, RearingAversion::with_direct_cue());
            }
        }
        let newborn = Newborn {
            id: child_id,
            genome: genome.clone(),
            phenotype,
            instincts,
            consciousness: Consciousness {
                awareness: 1,
                focus: 10,
                memory_capacity: 20,
            },
            skills: Skills::default(),
            lineage,
            residence: mother.residence,
            social_bonds: SocialBonds::default(),
        };
        spawn_newborn(&mut commands, newborn);
        let outcome = EventOutcome::Occurred(BTreeMap::from([(
            child_id,
            EffectSummary {
                seeded_genome: Some(genome),
                ..EffectSummary::default()
            },
        )]));
        if let Err(error) = log.append(
            clock.0,
            EventAuthor::Engine,
            vec![pending_birth.mother, pending_birth.father, child_id],
            EventPayload::Deterministic(DeterministicKind::Maturation),
            outcome,
        ) {
            faults.0.push(error);
        }
        stats.births = stats.births.saturating_add(1);
    }
}
