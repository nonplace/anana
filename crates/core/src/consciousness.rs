use serde::{Deserialize, Serialize};

/// A growing mind whose awareness unlocks learning and whose focus scales it.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ecs", derive(bevy_ecs::component::Component))]
pub struct Consciousness {
    pub awareness: u8,
    pub focus: u8,
    pub memory_capacity: u16,
}

#[cfg(test)]
mod tests {
    //! Consciousness stores awareness, focus, and memory capacity without duplicating Recall state.

    use super::*;

    #[test]
    fn consciousness_round_trips_without_a_duplicate_recall_flag() {
        let consciousness = Consciousness {
            awareness: 40,
            focus: 70,
            memory_capacity: 900,
        };
        let bytes = postcard::to_allocvec(&consciousness).expect("consciousness serializes");
        assert_eq!(
            postcard::from_bytes::<Consciousness>(&bytes).expect("consciousness deserializes"),
            consciousness
        );
    }
}
