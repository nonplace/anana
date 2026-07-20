use std::collections::{BTreeMap, BTreeSet};

use anana_core::{
    Body, ChanceTemplate, EffectSummary, EventAuthor, EventOutcome, EventPayload, HumanId,
    Infection, InfectionPhase, Instincts, Permille, Phenotype, RngDomain, SkillId, Skills,
    p_infect,
};
use bevy::prelude::{Entity, World};

use crate::{EventLog, SimulationFaults, SimulationRng, SimulationStats, Viruses, WorldClock};

#[derive(Clone)]
struct ContactSnapshot {
    entity: Entity,
    id: HumanId,
    robustness: u8,
    fear: u8,
    social: u8,
    body: Body,
    medicine_level: u8,
    infection: Option<Infection>,
}

fn append_infection_record(
    world: &mut World,
    tick: anana_core::Tick,
    source: HumanId,
    target: HumanId,
    virus: &anana_core::Virus,
    probability: Permille,
) {
    let outcome = EventOutcome::Occurred(BTreeMap::from([(
        target,
        EffectSummary {
            infection: Some(virus.id),
            ..EffectSummary::default()
        },
    )]));
    let payload = EventPayload::Chance {
        template: ChanceTemplate::Conflict,
        base_prob: probability,
        skill_modifier: Some(SkillId::Medicine),
        modifier_strength: Permille::ZERO,
    };
    let result = world.resource_mut::<EventLog>().append(
        tick,
        EventAuthor::Engine,
        vec![source, target],
        payload,
        outcome,
    );
    if let Err(error) = result {
        world.resource_mut::<SimulationFaults>().0.push(error);
    }
}

pub(crate) fn virus_spread(world: &mut World) {
    let tick = world.resource::<WorldClock>().0;
    let rng = world.resource::<SimulationRng>().0;
    let viruses = world.resource::<Viruses>().0.clone();

    let mut infection_query = world.query::<(Entity, &HumanId, &Infection)>();
    let mut infected = infection_query
        .iter(world)
        .map(|(entity, id, infection)| (*id, entity, infection.clone()))
        .collect::<Vec<_>>();
    infected.sort_by_key(|(id, _, _)| *id);
    let mut recovered = BTreeSet::new();
    for (id, entity, previous) in infected {
        let Some(virus) = viruses.get(&previous.strain) else {
            continue;
        };
        let mut should_recover = false;
        let strain = previous.strain;
        if let Some(mut infection) = world.entity_mut(entity).get_mut::<Infection>() {
            infection.ticks = infection.ticks.saturating_add(1);
            if infection.phase == InfectionPhase::Incubating
                && infection.ticks >= virus.incubation_ticks
            {
                infection.phase = InfectionPhase::Infectious;
            }
            let illness_end = virus
                .incubation_ticks
                .saturating_add(40)
                .saturating_add(u32::from(infection.severity));
            should_recover =
                infection.phase == InfectionPhase::Infectious && infection.ticks >= illness_end;
            if should_recover {
                infection.phase = InfectionPhase::Recovered;
            }
        }
        if should_recover {
            if let Some(mut body) = world.entity_mut(entity).get_mut::<Body>() {
                body.immunities.insert(strain);
            }
            world.entity_mut(entity).remove::<Infection>();
            recovered.insert(id);
        }
    }

    let mut contact_query = world.query::<(
        Entity,
        &HumanId,
        &Phenotype,
        &Instincts,
        &Body,
        &Skills,
        Option<&Infection>,
    )>();
    let mut snapshots = contact_query
        .iter(world)
        .map(
            |(entity, id, phenotype, instincts, body, skills, infection)| ContactSnapshot {
                entity,
                id: *id,
                robustness: phenotype.robustness.min(8),
                fear: instincts.fear.min(100),
                social: instincts.social.min(100),
                body: body.clone(),
                medicine_level: skills.level_of(SkillId::Medicine),
                infection: infection.cloned(),
            },
        )
        .collect::<Vec<_>>();
    snapshots.sort_by_key(|human| human.id);
    let sources = snapshots
        .iter()
        .filter(|human| {
            !recovered.contains(&human.id)
                && human
                    .infection
                    .as_ref()
                    .is_some_and(|infection| infection.phase == InfectionPhase::Infectious)
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut infected_this_tick = BTreeSet::new();
    for source in sources {
        let Some(infection) = source.infection.as_ref() else {
            continue;
        };
        let Some(virus) = viruses.get(&infection.strain) else {
            continue;
        };
        for target in &snapshots {
            if target.id == source.id
                || !target.body.alive
                || target.body.immunities.contains(&virus.id)
                || target.infection.is_some()
                || infected_this_tick.contains(&target.id)
            {
                continue;
            }
            let resistance = Permille(u16::from(target.robustness).saturating_mul(100).min(1000));
            let fear = Permille(u16::from(target.fear).saturating_mul(10).min(1000));
            let contact = Permille(
                u16::from(source.social)
                    .saturating_add(u16::from(target.social))
                    .saturating_mul(5)
                    .min(1000),
            );
            let medicine = Permille(
                u16::from(target.medicine_level)
                    .saturating_mul(200)
                    .min(1000),
            );
            let probability = p_infect(virus, resistance, fear, contact, medicine);
            let salt = source.id.0 ^ u64::from(virus.id.0);
            if !rng.gate(RngDomain::Infection, tick, target.id, salt, probability) {
                continue;
            }
            world.entity_mut(target.entity).insert(Infection {
                strain: virus.id,
                ticks: 0,
                severity: virus.virulence,
                phase: InfectionPhase::Incubating,
            });
            infected_this_tick.insert(target.id);
            append_infection_record(world, tick, source.id, target.id, virus, probability);
            let mut stats = world.resource_mut::<SimulationStats>();
            stats.infections = stats.infections.saturating_add(1);
        }
    }
}
