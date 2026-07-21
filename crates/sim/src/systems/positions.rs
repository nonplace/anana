use std::collections::BTreeMap;

use anana_core::{
    AttachedPosition, Body, ChanceTemplate, Consciousness, EventOutcome, EventPayload, HumanId,
    Permille, PositionSignal, Positions, RngDomain, Skills, SocialBonds, receive_position,
};
use bevy::prelude::{Entity, Query, Res};

use crate::{Config, EventLog, SimulationRng, WorldClock};

#[derive(Clone)]
struct PositionSnapshot {
    id: HumanId,
    alive: bool,
    retention: Permille,
    bonds: SocialBonds,
    positions: Positions,
}

fn event_value(payload: &EventPayload) -> Option<i16> {
    match payload {
        EventPayload::Chance { template, .. } => Some(match template {
            ChanceTemplate::Accident | ChanceTemplate::Conflict => -800,
            ChanceTemplate::Discovery | ChanceTemplate::Windfall => 800,
        }),
        EventPayload::Deterministic(_) | EventPayload::Gosh(_) => None,
    }
}

fn attached_positions(
    observer: &PositionSnapshot,
    people: &BTreeMap<HumanId, PositionSnapshot>,
    slot: u8,
) -> Vec<AttachedPosition> {
    observer
        .bonds
        .bonds
        .iter()
        .filter_map(|(id, bond)| {
            let neighbour = people.get(id)?;
            let position = neighbour.positions.slots.get(usize::from(slot))?;
            (position.conviction != Permille::ZERO).then_some(AttachedPosition {
                value: position.value,
                attachment: bond.strength,
            })
        })
        .collect()
}

fn expressed_signal(
    observer: &PositionSnapshot,
    people: &BTreeMap<HumanId, PositionSnapshot>,
    slot: u8,
) -> Option<PositionSignal> {
    observer
        .bonds
        .bonds
        .iter()
        .filter_map(|(id, bond)| {
            let model = people.get(id)?;
            let position = model.positions.slots.get(usize::from(slot))?;
            (position.conviction != Permille::ZERO).then_some((
                bond.strength,
                std::cmp::Reverse(*id),
                PositionSignal {
                    slot,
                    value: position.value,
                    retention: observer.retention,
                },
            ))
        })
        .max_by_key(|(strength, id, _)| (*strength, *id))
        .map(|(_, _, signal)| signal)
}

pub(crate) fn positions(
    clock: Res<'_, WorldClock>,
    rng: Res<'_, SimulationRng>,
    config: Res<'_, Config>,
    log: Res<'_, EventLog>,
    mut humans: Query<
        '_,
        '_,
        (
            Entity,
            &HumanId,
            &Body,
            &Consciousness,
            &Skills,
            &SocialBonds,
            &mut Positions,
        ),
    >,
) {
    if !clock.0.0.is_multiple_of(5) {
        return;
    }
    let mut snapshots = humans
        .iter_mut()
        .map(|(_, id, body, consciousness, skills, bonds, positions)| {
            let retention = if skills.recall_learned() {
                Permille(consciousness.memory_capacity.min(1_000))
            } else {
                Permille::ZERO
            };
            PositionSnapshot {
                id: *id,
                alive: body.alive,
                retention,
                bonds: bonds.clone(),
                positions: positions.clone(),
            }
        })
        .collect::<Vec<_>>();
    snapshots.sort_by_key(|human| human.id);
    let people = snapshots
        .iter()
        .cloned()
        .map(|human| (human.id, human))
        .collect::<BTreeMap<_, _>>();
    let entities = humans
        .iter_mut()
        .map(|(entity, id, _, _, _, _, _)| (*id, entity))
        .collect::<BTreeMap<_, _>>();

    for observer in snapshots.iter().filter(|human| human.alive) {
        let Some(entity) = entities.get(&observer.id).copied() else {
            continue;
        };
        let Ok((_, _, _, _, _, _, mut current)) = humans.get_mut(entity) else {
            continue;
        };
        current.record_relationship_count(observer.bonds.bonds.len());
        if observer.retention == Permille::ZERO {
            continue;
        }
        let default_slot = u8::try_from((clock.0.0 / 5) % 8).unwrap_or(0);
        let mut signals = log
            .records()
            .iter()
            .filter(|record| {
                record.tick == clock.0
                    && record.subjects.contains(&observer.id)
                    && matches!(record.outcome, EventOutcome::Occurred(_))
            })
            .filter_map(|record| {
                event_value(&record.payload).map(|value| PositionSignal {
                    slot: u8::try_from(record.seq.0 % 8).unwrap_or(0),
                    value,
                    retention: observer.retention,
                })
            })
            .collect::<Vec<_>>();
        if let Some(signal) = expressed_signal(observer, &people, default_slot) {
            signals.push(signal);
        } else if signals.is_empty() && clock.0.0.is_multiple_of(20) {
            let raw = rng.0.draw_u64(
                RngDomain::Position,
                clock.0,
                HumanId(0),
                u64::from(default_slot),
            ) % 2_001;
            let value = i16::try_from(raw).map_or(0, |value| value.saturating_sub(1_000));
            signals.push(PositionSignal {
                slot: default_slot,
                value,
                retention: observer.retention,
            });
        }
        for signal in signals {
            let attached = attached_positions(observer, &people, signal.slot);
            receive_position(
                &mut current,
                signal,
                &attached,
                config.coalition_cost_enabled,
            );
        }
    }
}
