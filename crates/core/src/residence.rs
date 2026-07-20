use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug, Default,
)]
pub struct ResidenceId(pub u64);

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ecs", derive(bevy_ecs::component::Component))]
pub struct Residence {
    pub id: ResidenceId,
}

#[cfg(test)]
mod tests {
    //! Co-residence is an explicit canonical fact rather than an inferred random proximity.

    use super::*;

    #[test]
    fn residence_identifiers_keep_groups_in_stable_domain_order() {
        let mut groups = [ResidenceId(3), ResidenceId(1), ResidenceId(2)];
        groups.sort();
        assert_eq!(groups, [ResidenceId(1), ResidenceId(2), ResidenceId(3)]);
    }
}
