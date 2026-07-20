use serde::{Deserialize, Serialize};

use crate::{HumanId, Tick};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ecs", derive(bevy_ecs::component::Component))]
pub struct Lineage {
    pub id: HumanId,
    pub mother: Option<HumanId>,
    pub father: Option<HumanId>,
    pub generation: u32,
    pub birth_tick: Tick,
    pub children: Vec<HumanId>,
    pub last_birth_tick: Option<Tick>,
}

impl Lineage {
    #[must_use]
    pub fn new(
        id: HumanId,
        mother: Option<HumanId>,
        father: Option<HumanId>,
        generation: u32,
        birth_tick: Tick,
    ) -> Self {
        Self {
            id,
            mother,
            father,
            generation,
            birth_tick,
            children: Vec::new(),
            last_birth_tick: None,
        }
    }
}

#[cfg(test)]
mod tests {
    //! Lineage preserves parentage, generation, and canonical child birth order.

    use super::*;
    use crate::{HumanId, Tick};

    #[test]
    fn a_newborn_starts_with_parentage_and_no_children() {
        let lineage = Lineage::new(HumanId(3), Some(HumanId(1)), Some(HumanId(2)), 2, Tick(40));
        assert_eq!(lineage.id, HumanId(3));
        assert_eq!(
            (lineage.mother, lineage.father),
            (Some(HumanId(1)), Some(HumanId(2)))
        );
        assert_eq!((lineage.generation, lineage.birth_tick), (2, Tick(40)));
        assert!(lineage.children.is_empty());
        assert_eq!(lineage.last_birth_tick, None);
    }

    #[test]
    fn children_remain_in_the_order_they_were_born() {
        let mut lineage = Lineage::new(HumanId(1), None, None, 0, Tick(0));
        lineage.children.push(HumanId(4));
        lineage.children.push(HumanId(7));
        assert_eq!(lineage.children, vec![HumanId(4), HumanId(7)]);
    }
}
