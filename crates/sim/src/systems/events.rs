use std::collections::BTreeMap;

use anana_core::{
    Body, ChanceTemplate, Consciousness, EventAuthor, EventOutcome, EventPayload, Genome, GodId,
    HumanId, HumanState, Infection, InfectionPhase, Instincts, Lineage, Permille, Phenotype,
    Positions, Residence, Skills, SocialBonds, WorldView, encode_experience_magnitude,
    exercised_skill, learning_gain, record_defection, record_positive_interaction, resolve,
};
use bevy::prelude::{Commands, Entity, Query, Res, ResMut};

use crate::{
    EventIntake, EventLog, Gods, NextHumanId, NextResidenceId, SimulationFaults, SimulationRng,
    SimulationStats, Viruses, WorldClock,
};

use super::birth::{Newborn, spawn_newborn};

type EventHumanQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static HumanId,
        &'static Genome,
        &'static Phenotype,
        &'static Instincts,
        &'static Consciousness,
        &'static mut Body,
        &'static mut Skills,
        &'static mut Lineage,
        &'static Residence,
        &'static mut SocialBonds,
        &'static Positions,
        Option<&'static Infection>,
    ),
>;

type EventParams<'w, 's> = (
    Commands<'w, 's>,
    Res<'w, WorldClock>,
    Res<'w, SimulationRng>,
    Res<'w, EventIntake>,
    Res<'w, Viruses>,
    ResMut<'w, EventLog>,
    ResMut<'w, NextHumanId>,
    ResMut<'w, NextResidenceId>,
    ResMut<'w, Gods>,
    ResMut<'w, SimulationFaults>,
    ResMut<'w, SimulationStats>,
    EventHumanQuery<'w, 's>,
);

fn snapshot_humans(humans: &mut EventHumanQuery<'_, '_>) -> BTreeMap<HumanId, HumanState> {
    humans
        .iter_mut()
        .map(
            |(
                _,
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
                positions,
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
                        positions: positions.clone(),
                        infection: infection.cloned(),
                    },
                )
            },
        )
        .collect()
}

struct ApplyContext<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    rng: anana_core::Rng,
    tick: anana_core::Tick,
    viruses: &'a Viruses,
    next_id: &'a mut NextHumanId,
    next_residence: &'a mut NextResidenceId,
    faults: &'a mut SimulationFaults,
    stats: &'a mut SimulationStats,
}

fn apply_outcome(
    outcome: &EventOutcome,
    humans: &mut EventHumanQuery<'_, '_>,
    context: &mut ApplyContext<'_, '_, '_>,
) {
    let EventOutcome::Occurred(effects) = outcome else {
        return;
    };
    let entities = humans
        .iter_mut()
        .map(|(entity, id, _, _, _, _, _, _, _, _, _, _, _)| (*id, entity))
        .collect::<BTreeMap<_, _>>();
    for (id, effect) in effects {
        if let Some(entity) = entities.get(id).copied() {
            let Ok((_, _, _, _, _, consciousness, mut body, mut skills, _, _, _, _, _)) =
                humans.get_mut(entity)
            else {
                continue;
            };
            let health = i64::from(body.health).saturating_add(i64::from(effect.health_delta));
            body.health = health.clamp(0, i64::from(body.max_health)) as u16;
            let fertility =
                i64::from(body.fertility).saturating_add(i64::from(effect.fertility_delta));
            body.fertility = fertility.clamp(0, 100) as u8;
            body.age_ticks = body.age_ticks.saturating_add(effect.age_ticks_delta);
            for (skill, xp) in &effect.skill_xp {
                let _result =
                    anana_core::apply_calculated_learning(&mut skills, consciousness, *skill, *xp);
            }
            body.immunities
                .extend(effect.immunities_granted.iter().copied());
            if let Some(virus_id) = effect.infection
                && !body.immunities.contains(&virus_id)
            {
                let severity = context
                    .viruses
                    .0
                    .get(&virus_id)
                    .map_or(0, |virus| virus.virulence);
                context.commands.entity(entity).insert(Infection {
                    strain: virus_id,
                    ticks: 0,
                    severity,
                    phase: InfectionPhase::Incubating,
                });
            }
        } else if let Some(genome) = effect.seeded_genome.clone() {
            let allocated = match context.next_id.allocate() {
                Ok(allocated) => allocated,
                Err(error) => {
                    context.faults.0.push(error);
                    continue;
                }
            };
            let residence_id = match context.next_residence.allocate() {
                Ok(residence) => residence,
                Err(error) => {
                    context.faults.0.push(error);
                    continue;
                }
            };
            let phenotype = anana_core::express(&genome, &context.rng, context.tick, allocated);
            spawn_newborn(
                context.commands,
                Newborn {
                    id: allocated,
                    genome,
                    phenotype,
                    instincts: Instincts {
                        survival: 50,
                        reproduction: 50,
                        hunger: 50,
                        fear: 50,
                        social: 50,
                    },
                    consciousness: Consciousness {
                        awareness: 1,
                        focus: 10,
                        memory_capacity: 20,
                    },
                    skills: Skills::default(),
                    lineage: Lineage::new(allocated, None, None, 0, context.tick),
                    residence: Residence { id: residence_id },
                    social_bonds: SocialBonds::default(),
                    positions: Positions::default(),
                },
            );
            context.stats.births = context.stats.births.saturating_add(1);
        }
    }
}

fn add_lived_experience(
    outcome: &mut EventOutcome,
    template: ChanceTemplate,
    humans: &BTreeMap<HumanId, HumanState>,
) {
    let EventOutcome::Occurred(effects) = outcome else {
        return;
    };
    let skill = exercised_skill(template);
    let is_bad = matches!(
        template,
        ChanceTemplate::Accident | ChanceTemplate::Conflict
    );
    for (id, effect) in effects {
        let Some(human) = humans.get(id) else {
            continue;
        };
        let encoded = encode_experience_magnitude(20, is_bad, human.phenotype.threat_salience);
        if let Ok(gain) = learning_gain(
            &human.skills,
            &human.consciousness,
            &human.phenotype,
            skill,
            encoded,
        ) && gain > 0
        {
            effect.skill_xp.insert(skill, gain);
        }
    }
}

fn record_shared_relationship(
    outcome: &EventOutcome,
    template: ChanceTemplate,
    subjects: &[HumanId],
    humans: &mut EventHumanQuery<'_, '_>,
    tick: anana_core::Tick,
) {
    if !matches!(outcome, EventOutcome::Occurred(_)) {
        return;
    }
    let entities = humans
        .iter_mut()
        .map(|(entity, id, _, _, _, _, _, _, _, _, _, _, _)| (*id, entity))
        .collect::<BTreeMap<_, _>>();
    for observer in subjects {
        let Some(entity) = entities.get(observer).copied() else {
            continue;
        };
        let Ok((_, _, _, _, _, _, _, _, _, _, mut bonds, _, _)) = humans.get_mut(entity) else {
            continue;
        };
        for other in subjects.iter().copied().filter(|other| other != observer) {
            let bond = bonds.bonds.entry(other).or_default();
            if template == ChanceTemplate::Conflict && Some(&other) == subjects.first() {
                record_defection(bond, tick);
            } else {
                record_positive_interaction(bond, tick, Permille::ZERO);
            }
        }
    }
}

fn co_resident_participants(
    humans: &BTreeMap<HumanId, HumanState>,
    subject: HumanId,
    limit: usize,
) -> Vec<HumanId> {
    let Some(anchor) = humans.get(&subject) else {
        return Vec::new();
    };
    let group = humans
        .values()
        .filter(|human| human.body.alive && human.residence == anchor.residence)
        .map(|human| human.id)
        .collect::<Vec<_>>();
    let Some(start) = group.iter().position(|id| *id == subject) else {
        return Vec::new();
    };
    (0..limit.min(group.len()))
        .filter_map(|offset| {
            start
                .checked_add(offset)
                .and_then(|index| index.checked_rem(group.len()))
                .and_then(|index| group.get(index))
                .copied()
        })
        .collect()
}

pub(crate) fn events(params: EventParams<'_, '_>) {
    let (
        mut commands,
        clock,
        rng,
        intake,
        viruses,
        mut log,
        mut next_id,
        mut next_residence,
        mut gods,
        mut faults,
        mut stats,
        mut humans,
    ) = params;
    let pending_events = match intake.drain() {
        Ok(events) => events,
        Err(error) => {
            faults.0.push(error);
            Vec::new()
        }
    };
    for pending in pending_events {
        let seq = match log.next_seq() {
            Ok(seq) => seq,
            Err(error) => {
                faults.0.push(error);
                continue;
            }
        };
        let canonical = snapshot_humans(&mut humans);
        let view = WorldView {
            humans: &canonical,
            subjects: &pending.subjects,
            next_human_id: next_id.0,
        };
        let outcome = resolve(&pending.payload, &view, &rng.0, pending.tick, seq);
        let mut context = ApplyContext {
            commands: &mut commands,
            rng: rng.0,
            tick: pending.tick,
            viruses: &viruses,
            next_id: &mut next_id,
            next_residence: &mut next_residence,
            faults: &mut faults,
            stats: &mut stats,
        };
        apply_outcome(&outcome, &mut humans, &mut context);
        if let Err(error) = log.append(
            pending.tick,
            pending.author,
            pending.subjects,
            pending.payload,
            outcome,
        ) {
            faults.0.push(error);
        }
        if pending.author == EventAuthor::God
            && let Some(god) = gods.0.get_mut(&GodId(1))
        {
            god.goshes_spoken = god.goshes_spoken.saturating_add(1);
        }
    }

    if !clock.0.0.is_multiple_of(10) {
        return;
    }
    let canonical = snapshot_humans(&mut humans);
    let subject_ids = canonical
        .iter()
        .filter_map(|(id, human)| human.body.alive.then_some(id))
        .copied()
        .collect::<Vec<_>>();
    for subject in subject_ids {
        let seq = match log.next_seq() {
            Ok(seq) => seq,
            Err(error) => {
                faults.0.push(error);
                continue;
            }
        };
        let subjects = co_resident_participants(&canonical, subject, 3);
        let view = WorldView {
            humans: &canonical,
            subjects: &subjects,
            next_human_id: next_id.0,
        };
        let template = match (clock.0.0 / 10) % 4 {
            0 => ChanceTemplate::Accident,
            1 => ChanceTemplate::Discovery,
            2 => ChanceTemplate::Conflict,
            _ => ChanceTemplate::Windfall,
        };
        let payload = EventPayload::Chance {
            template,
            base_prob: Permille(20),
            skill_modifier: Some(anana_core::SkillId::Planning),
            modifier_strength: Permille(10),
        };
        let mut outcome = resolve(&payload, &view, &rng.0, clock.0, seq);
        add_lived_experience(&mut outcome, template, &canonical);
        let mut context = ApplyContext {
            commands: &mut commands,
            rng: rng.0,
            tick: clock.0,
            viruses: &viruses,
            next_id: &mut next_id,
            next_residence: &mut next_residence,
            faults: &mut faults,
            stats: &mut stats,
        };
        apply_outcome(&outcome, &mut humans, &mut context);
        record_shared_relationship(&outcome, template, &subjects, &mut humans, clock.0);
        if let Err(error) = log.append(clock.0, EventAuthor::Engine, subjects, payload, outcome) {
            faults.0.push(error);
        }
    }
}
