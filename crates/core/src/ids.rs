use serde::{Deserialize, Serialize};

macro_rules! domain_id {
    ($name:ident, $inner:ty) => {
        #[derive(
            Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug,
        )]
        pub struct $name(pub $inner);
    };
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ecs", derive(bevy_ecs::component::Component))]
pub struct HumanId(pub u64);

domain_id!(Tick, u64);
domain_id!(Seq, u32);
domain_id!(VirusId, u32);
domain_id!(GodId, u32);

#[cfg(test)]
mod tests {
    //! Stable domain identifiers keep canonical ordering independent of engine internals.

    use super::*;

    #[test]
    fn human_identifiers_sort_in_monotonic_domain_order() {
        let mut ids = [HumanId(9), HumanId(2), HumanId(5)];
        ids.sort();
        assert_eq!(ids, [HumanId(2), HumanId(5), HumanId(9)]);
    }

    #[test]
    fn every_identifier_round_trips_through_the_canonical_serializer() {
        let ids = (HumanId(11), Tick(12), Seq(13), VirusId(14), GodId(15));
        let bytes = postcard::to_allocvec(&ids).expect("identifiers serialize");
        let decoded = postcard::from_bytes(&bytes).expect("identifiers deserialize");
        assert_eq!(ids, decoded);
    }
}
