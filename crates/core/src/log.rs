use serde::{Deserialize, Serialize};

use crate::{EventAuthor, EventOutcome, EventPayload, HumanId, Seq, Tick};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct EventRecord {
    pub tick: Tick,
    pub seq: Seq,
    pub author: EventAuthor,
    pub subjects: Vec<HumanId>,
    pub payload: EventPayload,
    pub outcome: EventOutcome,
    pub narration: Option<String>,
}

#[cfg(test)]
mod tests {
    //! Event records preserve every replay input and resolved output through canonical serialization.

    use super::*;
    use crate::{DeterministicKind, EventAuthor, EventOutcome, EventPayload, HumanId, Seq, Tick};

    #[test]
    fn an_event_record_round_trips_through_postcard_unchanged() {
        let record = EventRecord {
            tick: Tick(7),
            seq: Seq(3),
            author: EventAuthor::Engine,
            subjects: vec![HumanId(2), HumanId(5)],
            payload: EventPayload::Deterministic(DeterministicKind::Maturation),
            outcome: EventOutcome::NoOp,
            narration: Some(String::from("time passed")),
        };
        let bytes = postcard::to_allocvec(&record).expect("record serializes");
        assert_eq!(
            postcard::from_bytes::<EventRecord>(&bytes).expect("record deserializes"),
            record
        );
    }
}
