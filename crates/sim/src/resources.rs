use std::{collections::BTreeMap, mem, sync::Mutex};

use anana_core::{
    EventAuthor, EventOutcome, EventPayload, EventRecord, God, GodId, GoshKind, GoshTarget,
    HumanId, Rng, Seq, Tick, Virus, VirusId,
};
use bevy::prelude::Resource;
use thiserror::Error;

#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum SimError {
    #[error("the event intake lock was poisoned")]
    EventIntakePoisoned,
    #[error("the human identifier allocator is exhausted")]
    HumanIdExhausted,
    #[error("the event sequence is exhausted")]
    EventSequenceExhausted,
}

#[derive(Clone, PartialEq, Eq, Debug, Resource)]
pub struct Config {
    pub initial_population: u32,
    pub max_population: u32,
    pub mating_interval: u64,
    pub requested_threads: usize,
    pub initial_virus: Virus,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            initial_population: 5,
            max_population: 64,
            mating_interval: 10,
            requested_threads: 1,
            initial_virus: Virus {
                id: VirusId(1),
                spreadscore: 38,
                virulence: 12,
                incubation_ticks: 8,
                mutation_rate: anana_core::Permille(3),
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Resource)]
pub struct WorldClock(pub Tick);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Resource)]
pub struct SimulationRng(pub Rng);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Resource)]
pub struct NextHumanId(pub HumanId);

impl NextHumanId {
    pub fn allocate(&mut self) -> Result<HumanId, SimError> {
        let next = self.0.0.checked_add(1).ok_or(SimError::HumanIdExhausted)?;
        let allocated = self.0;
        self.0 = HumanId(next);
        Ok(allocated)
    }
}

#[derive(Debug, Default, Resource)]
pub struct EventLog {
    records: Vec<EventRecord>,
    next_seq: u32,
}

impl EventLog {
    #[must_use]
    pub fn records(&self) -> &[EventRecord] {
        &self.records
    }

    pub fn next_seq(&self) -> Result<Seq, SimError> {
        if self.next_seq == u32::MAX {
            Err(SimError::EventSequenceExhausted)
        } else {
            Ok(Seq(self.next_seq))
        }
    }

    pub fn append(
        &mut self,
        tick: Tick,
        author: EventAuthor,
        subjects: Vec<HumanId>,
        payload: EventPayload,
        outcome: EventOutcome,
    ) -> Result<Seq, SimError> {
        let seq = self.next_seq()?;
        let next = self
            .next_seq
            .checked_add(1)
            .ok_or(SimError::EventSequenceExhausted)?;
        self.records.push(EventRecord {
            tick,
            seq,
            author,
            subjects,
            payload,
            outcome,
            narration: None,
        });
        self.next_seq = next;
        Ok(seq)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PendingEvent {
    pub author: EventAuthor,
    pub tick: Tick,
    pub subjects: Vec<HumanId>,
    pub payload: EventPayload,
}

#[derive(Debug, Default, Resource)]
pub struct EventIntake {
    events: Mutex<Vec<PendingEvent>>,
}

impl EventIntake {
    pub fn cast_gosh(&self, now: Tick, gosh: GoshKind) -> Result<(), SimError> {
        let subjects = match &gosh {
            GoshKind::Bless { subject, .. } | GoshKind::Teach { subject, .. } => vec![*subject],
            GoshKind::Afflict {
                target: GoshTarget::One(subject) | GoshTarget::Lineage(subject),
                ..
            } => vec![*subject],
            GoshKind::Afflict {
                target: GoshTarget::All,
                ..
            }
            | GoshKind::Seed { .. } => Vec::new(),
        };
        self.push(PendingEvent {
            author: EventAuthor::God,
            tick: now,
            subjects,
            payload: EventPayload::Gosh(gosh),
        })
    }

    pub fn push(&self, event: PendingEvent) -> Result<(), SimError> {
        let mut events = self
            .events
            .lock()
            .map_err(|_| SimError::EventIntakePoisoned)?;
        events.push(event);
        Ok(())
    }

    pub fn drain(&self) -> Result<Vec<PendingEvent>, SimError> {
        let mut events = self
            .events
            .lock()
            .map_err(|_| SimError::EventIntakePoisoned)?;
        Ok(mem::take(&mut *events))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PendingBirth {
    pub mother: HumanId,
    pub father: HumanId,
}

#[derive(Debug, Default, Resource)]
pub struct PendingBirths(pub Vec<PendingBirth>);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Resource)]
pub struct SimulationStats {
    pub births: u64,
    pub deaths: u64,
    pub infections: u64,
    pub living: u64,
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Resource)]
pub struct SimulationFaults(pub Vec<SimError>);

#[derive(Clone, PartialEq, Eq, Debug, Default, Resource)]
pub struct HashHistory(pub Vec<[u8; 32]>);

#[derive(Clone, PartialEq, Eq, Debug, Default, Resource)]
pub struct Viruses(pub BTreeMap<VirusId, Virus>);

#[derive(Clone, PartialEq, Eq, Debug, Default, Resource)]
pub struct Gods(pub BTreeMap<GodId, God>);

#[cfg(test)]
mod tests {
    //! Simulation resources preserve monotonic allocation, append-only history, and captured input order.

    use anana_core::{
        Boon, DeterministicKind, EventAuthor, EventOutcome, EventPayload, GoshKind, HumanId, Seq,
        Tick,
    };

    use super::*;

    #[test]
    fn human_identifiers_allocate_monotonically_and_report_exhaustion() {
        let mut ids = NextHumanId(HumanId(7));
        assert_eq!(ids.allocate(), Ok(HumanId(7)));
        assert_eq!(ids.allocate(), Ok(HumanId(8)));
        let mut exhausted = NextHumanId(HumanId(u64::MAX));
        assert_eq!(exhausted.allocate(), Err(SimError::HumanIdExhausted));
    }

    #[test]
    fn event_records_receive_strictly_increasing_sequences_only_when_appended() {
        let mut log = EventLog::default();
        let payload = EventPayload::Deterministic(DeterministicKind::Maturation);
        assert_eq!(log.next_seq(), Ok(Seq(0)));
        assert_eq!(
            log.append(
                Tick(1),
                EventAuthor::Engine,
                vec![HumanId(1)],
                payload.clone(),
                EventOutcome::NoOp,
            ),
            Ok(Seq(0))
        );
        assert_eq!(log.next_seq(), Ok(Seq(1)));
        assert_eq!(
            log.append(
                Tick(1),
                EventAuthor::Engine,
                vec![HumanId(2)],
                payload,
                EventOutcome::NoOp,
            ),
            Ok(Seq(1))
        );
        assert_eq!(
            log.records()
                .iter()
                .map(|record| record.seq)
                .collect::<Vec<_>>(),
            vec![Seq(0), Seq(1)]
        );
    }

    #[test]
    fn pending_events_drain_in_capture_order_and_goshes_are_authored_by_god() {
        let intake = EventIntake::default();
        intake
            .push(PendingEvent {
                author: EventAuthor::Ai,
                tick: Tick(4),
                subjects: vec![HumanId(2)],
                payload: EventPayload::Deterministic(DeterministicKind::HealthTick),
            })
            .expect("intake is available");
        intake
            .cast_gosh(
                Tick(4),
                GoshKind::Bless {
                    subject: HumanId(1),
                    boon: Boon::Heal(5),
                },
            )
            .expect("intake is available");
        let drained = intake.drain().expect("intake is available");
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].author, EventAuthor::Ai);
        assert_eq!(drained[1].author, EventAuthor::God);
        assert_eq!(drained[1].subjects, vec![HumanId(1)]);
        assert!(intake.drain().expect("intake is available").is_empty());
    }

    #[test]
    fn a_poisoned_event_intake_returns_an_error_instead_of_panicking() {
        let intake = std::sync::Arc::new(EventIntake::default());
        let worker_intake = std::sync::Arc::clone(&intake);
        let worker = std::thread::spawn(move || {
            let _guard = worker_intake.events.lock().expect("test acquires lock");
            panic!("poison the lock for this test");
        });
        assert!(worker.join().is_err());
        assert_eq!(intake.drain(), Err(SimError::EventIntakePoisoned));
    }
}
