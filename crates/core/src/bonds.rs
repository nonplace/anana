use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{HumanId, Lineage, Permille, Tick};

pub const COURTSHIP_THRESHOLD: Permille = Permille(700);
const DIRECT_REARING_CUE: Permille = Permille(900);
const DURATION_CUE_CAPACITY: u32 = 572;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Bond {
    pub strength: Permille,
    pub last_interaction: Tick,
    pub last_decay_tick: Tick,
    pub positive_interactions: u32,
    pub defections: u32,
}

impl Default for Bond {
    fn default() -> Self {
        Self {
            strength: Permille::ZERO,
            last_interaction: Tick(0),
            last_decay_tick: Tick(0),
            positive_interactions: 0,
            defections: 0,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
pub struct RearingAversion {
    pub direct_cue: bool,
    pub duration_weight: u32,
}

impl RearingAversion {
    #[must_use]
    pub fn with_direct_cue() -> Self {
        Self {
            direct_cue: true,
            duration_weight: 0,
        }
    }

    pub fn observe_co_residence(&mut self, age_ticks: u32) {
        if self.direct_cue {
            return;
        }
        let weight = match age_ticks {
            0..=77 => 4,
            78..=155 => 2,
            156..=259 => 1,
            _ => 0,
        };
        self.duration_weight = self
            .duration_weight
            .saturating_add(weight)
            .min(DURATION_CUE_CAPACITY);
    }

    #[must_use]
    pub fn strength(&self) -> Permille {
        if self.direct_cue {
            return DIRECT_REARING_CUE;
        }
        Permille::clamp1000(
            i64::from(self.duration_weight).saturating_mul(900) / i64::from(DURATION_CUE_CAPACITY),
        )
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
#[cfg_attr(feature = "ecs", derive(bevy_ecs::component::Component))]
pub struct SocialBonds {
    pub bonds: BTreeMap<HumanId, Bond>,
    pub rearing_aversions: BTreeMap<HumanId, RearingAversion>,
    pub observed_competence: BTreeMap<HumanId, u16>,
    pub deference: BTreeMap<HumanId, Permille>,
}

fn integer_sqrt(value: u32) -> u32 {
    if value < 2 {
        return value;
    }
    let mut low = 1_u32;
    let mut high = value.min(u16::MAX.into());
    while low < high {
        let middle = low.saturating_add(high).saturating_add(1) / 2;
        if middle <= value / middle {
            low = middle;
        } else {
            high = middle.saturating_sub(1);
        }
    }
    low
}

pub fn record_positive_interaction(bond: &mut Bond, now: Tick, model_prestige: Permille) -> u16 {
    record_positive_interaction_scaled(bond, now, model_prestige, Permille::ONE)
}

pub fn record_positive_interaction_scaled(
    bond: &mut Bond,
    now: Tick,
    model_prestige: Permille,
    attraction_factor: Permille,
) -> u16 {
    let familiarity = integer_sqrt(bond.positive_interactions)
        .saturating_add(1)
        .max(1);
    let base = 80_u32.saturating_add(160_u32 / familiarity);
    let prestige = u32::from(model_prestige.0.min(1000)) / 25;
    let unscaled = base.saturating_add(prestige).clamp(12, 200);
    let gain = unscaled
        .saturating_mul(u32::from(attraction_factor.0.min(1000)))
        .saturating_div(1000)
        .min(200) as u16;
    bond.strength = Permille::clamp1000(i64::from(bond.strength.0) + i64::from(gain));
    bond.last_interaction = now;
    bond.last_decay_tick = now;
    bond.positive_interactions = bond.positive_interactions.saturating_add(1);
    gain
}

pub fn decay_bond(bond: &mut Bond, now: Tick) -> u16 {
    let total_elapsed =
        u32::try_from(now.0.saturating_sub(bond.last_interaction.0)).unwrap_or(u32::MAX);
    let previous_elapsed = u32::try_from(
        bond.last_decay_tick
            .0
            .saturating_sub(bond.last_interaction.0),
    )
    .unwrap_or(u32::MAX);
    let loss = integer_sqrt(total_elapsed)
        .saturating_sub(integer_sqrt(previous_elapsed))
        .saturating_mul(25)
        .min(1000) as u16;
    bond.strength = Permille(bond.strength.0.saturating_sub(loss));
    bond.last_decay_tick = now;
    loss
}

pub fn record_defection(bond: &mut Bond, now: Tick) -> u16 {
    const DEFECTION_COST: u16 = 240;
    bond.strength = Permille(bond.strength.0.saturating_sub(DEFECTION_COST));
    bond.last_interaction = now;
    bond.last_decay_tick = now;
    bond.defections = bond.defections.saturating_add(1);
    DEFECTION_COST
}

#[must_use]
pub fn bond_is_courtship_ready(bond: &Bond) -> bool {
    bond.strength >= COURTSHIP_THRESHOLD && bond.defections < 3
}

#[must_use]
pub fn mutual_bond_strength(first: &Bond, second: &Bond) -> Permille {
    first.strength.min(second.strength)
}

#[must_use]
pub fn mutual_courtship_is_ready(first: &Bond, second: &Bond) -> bool {
    bond_is_courtship_ready(first)
        && bond_is_courtship_ready(second)
        && mutual_bond_strength(first, second) >= COURTSHIP_THRESHOLD
}

#[must_use]
pub fn are_first_degree_relatives(first: &Lineage, second: &Lineage) -> bool {
    let parent_child = first.mother == Some(second.id)
        || first.father == Some(second.id)
        || second.mother == Some(first.id)
        || second.father == Some(first.id);
    let shared_mother = first.mother.is_some() && first.mother == second.mother;
    let shared_father = first.father.is_some() && first.father == second.father;
    parent_child || shared_mother || shared_father
}

#[must_use]
pub fn courtship_aversion_factor(first: Permille, second: Permille) -> Permille {
    let combined = first.max(second).0.min(1000);
    Permille(1000_u16.saturating_sub(combined).max(25))
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct MateProfile {
    pub age_permille: u16,
    pub values: u8,
    pub cognition: u8,
    pub body: u8,
    pub temperament: u8,
    pub desirability: u8,
}

fn similarity(left: u16, right: u16, maximum: u16) -> u32 {
    u32::from(maximum.saturating_sub(left.abs_diff(right).min(maximum))).saturating_mul(1000)
        / u32::from(maximum.max(1))
}

#[must_use]
pub fn attraction_score(chooser: &MateProfile, candidate: &MateProfile) -> u16 {
    let age = similarity(
        chooser.age_permille.min(1000),
        candidate.age_permille.min(1000),
        1000,
    )
    .saturating_mul(300);
    let values = similarity(
        u16::from(chooser.values.min(100)),
        u16::from(candidate.values.min(100)),
        100,
    )
    .saturating_mul(250);
    let cognition = similarity(
        u16::from(chooser.cognition.min(100)),
        u16::from(candidate.cognition.min(100)),
        100,
    )
    .saturating_mul(200);
    let body = similarity(
        u16::from(chooser.body.min(100)),
        u16::from(candidate.body.min(100)),
        100,
    )
    .saturating_mul(130);
    let temperament = similarity(
        u16::from(chooser.temperament.min(100)),
        u16::from(candidate.temperament.min(100)),
        100,
    )
    .saturating_mul(70);
    let quality = u32::from(candidate.desirability.min(100))
        .saturating_mul(10)
        .saturating_mul(50);
    age.saturating_add(values)
        .saturating_add(cognition)
        .saturating_add(body)
        .saturating_add(temperament)
        .saturating_add(quality)
        .saturating_div(1000)
        .min(1000) as u16
}

#[cfg(test)]
mod tests {
    //! Bonds prove that attachment grows through repeated cooperation, remembers betrayal,
    //! fades through disuse, and distinguishes lineage from childhood co-residence.

    use crate::{HumanId, Lineage, Permille, Tick};

    use super::*;

    #[test]
    fn one_friendly_encounter_cannot_create_a_courtship_bond() {
        let mut bond = Bond::default();
        record_positive_interaction(&mut bond, Tick(1), Permille::ZERO);
        assert!(!bond_is_courtship_ready(&bond));
    }

    #[test]
    fn repeated_positive_encounters_eventually_create_a_courtship_bond() {
        let mut bond = Bond::default();
        for tick in 1..=24 {
            record_positive_interaction(&mut bond, Tick(tick), Permille::ZERO);
        }
        assert!(bond_is_courtship_ready(&bond));
    }

    #[test]
    fn attachment_gains_diminish_as_familiarity_accumulates() {
        let mut bond = Bond::default();
        let first = record_positive_interaction(&mut bond, Tick(1), Permille::ZERO);
        let second = record_positive_interaction(&mut bond, Tick(2), Permille::ZERO);
        let hundredth = (3..=100)
            .map(|tick| record_positive_interaction(&mut bond, Tick(tick), Permille::ZERO))
            .last()
            .expect("the range has positive encounters");
        assert!(first > second);
        assert!(second > hundredth);
    }

    #[test]
    fn an_unmaintained_bond_falls_back_below_the_courtship_threshold() {
        let mut bond = Bond {
            strength: Permille::ONE,
            last_interaction: Tick(1),
            last_decay_tick: Tick(1),
            positive_interactions: 30,
            defections: 0,
        };
        decay_bond(&mut bond, Tick(500));
        assert!(!bond_is_courtship_ready(&bond));
    }

    #[test]
    fn one_betrayal_costs_more_than_one_cooperation_gains() {
        let mut bond = Bond::default();
        let gain = record_positive_interaction(&mut bond, Tick(1), Permille::ZERO);
        let loss = record_defection(&mut bond, Tick(2));
        assert!(loss > gain);
        assert_eq!(bond.defections, 1);
    }

    #[test]
    fn one_betrayal_can_be_repaired_but_repeated_betrayal_overwhelms_cooperation() {
        let mut repaired = Bond::default();
        record_defection(&mut repaired, Tick(1));
        for tick in 2..=30 {
            record_positive_interaction(&mut repaired, Tick(tick), Permille::ZERO);
        }
        let mut repeated = Bond::default();
        for tick in 1..=6 {
            record_defection(&mut repeated, Tick(tick));
        }
        for tick in 7..=35 {
            record_positive_interaction(&mut repeated, Tick(tick), Permille::ZERO);
        }
        assert!(bond_is_courtship_ready(&repaired));
        assert!(!bond_is_courtship_ready(&repeated));
    }

    #[test]
    fn courtship_uses_the_weaker_of_two_directional_bonds() {
        let strong = Bond {
            strength: Permille(800),
            ..Bond::default()
        };
        let weak = Bond {
            strength: Permille(400),
            ..Bond::default()
        };
        assert_eq!(mutual_bond_strength(&strong, &weak), Permille(400));
        assert!(!mutual_courtship_is_ready(&strong, &weak));
    }

    #[test]
    fn parents_children_and_half_siblings_are_first_degree_relatives() {
        let parent = Lineage::new(HumanId(1), None, None, 0, Tick(0));
        let child = Lineage::new(HumanId(2), Some(HumanId(1)), Some(HumanId(9)), 1, Tick(1));
        let half_sibling = Lineage::new(HumanId(3), Some(HumanId(1)), Some(HumanId(8)), 1, Tick(2));
        assert!(are_first_degree_relatives(&parent, &child));
        assert!(are_first_degree_relatives(&child, &half_sibling));
    }

    #[test]
    fn separated_siblings_have_no_rearing_aversion_despite_the_lineage_block() {
        let first = Lineage::new(HumanId(2), Some(HumanId(1)), Some(HumanId(9)), 1, Tick(1));
        let second = Lineage::new(HumanId(3), Some(HumanId(1)), Some(HumanId(8)), 1, Tick(2));
        assert!(are_first_degree_relatives(&first, &second));
        assert_eq!(RearingAversion::default().strength(), Permille::ZERO);
    }

    #[test]
    fn the_older_childs_direct_cue_differs_from_the_younger_childs_duration_cue() {
        let older = RearingAversion::with_direct_cue();
        let mut younger = RearingAversion::default();
        for age in 0..120 {
            younger.observe_co_residence(age);
        }
        assert!(older.direct_cue);
        assert!(!younger.direct_cue);
        assert_ne!(older.strength(), younger.strength());
    }

    #[test]
    fn a_direct_newborn_cue_replaces_rather_than_adds_to_duration() {
        let mut aversion = RearingAversion::with_direct_cue();
        let before = aversion.strength();
        for age in 0..260 {
            aversion.observe_co_residence(age);
        }
        assert_eq!(aversion.strength(), before);
    }

    #[test]
    fn early_childhood_co_residence_builds_more_aversion_than_late_childhood() {
        let mut early = RearingAversion::default();
        let mut late = RearingAversion::default();
        for age in 0..78 {
            early.observe_co_residence(age);
        }
        for age in 182..260 {
            late.observe_co_residence(age);
        }
        assert!(early.strength() > late.strength());
    }

    #[test]
    fn co_rearing_strongly_suppresses_courtship_without_making_it_impossible() {
        let factor = courtship_aversion_factor(Permille(900), Permille(700));
        assert!(factor > Permille::ZERO);
        assert!(factor < Permille(200));
    }

    #[test]
    fn age_similarity_shapes_attraction_more_than_values_cognition_body_or_temperament() {
        let chooser = MateProfile {
            age_permille: 500,
            values: 50,
            cognition: 50,
            body: 50,
            temperament: 50,
            desirability: 50,
        };
        let baseline = attraction_score(&chooser, &chooser);
        let age_cost = baseline.saturating_sub(attraction_score(
            &chooser,
            &MateProfile {
                age_permille: 0,
                ..chooser
            },
        ));
        let values_cost = baseline.saturating_sub(attraction_score(
            &chooser,
            &MateProfile {
                values: 0,
                ..chooser
            },
        ));
        let cognition_cost = baseline.saturating_sub(attraction_score(
            &chooser,
            &MateProfile {
                cognition: 0,
                ..chooser
            },
        ));
        let body_cost = baseline.saturating_sub(attraction_score(
            &chooser,
            &MateProfile { body: 0, ..chooser },
        ));
        let temperament_cost = baseline.saturating_sub(attraction_score(
            &chooser,
            &MateProfile {
                temperament: 0,
                ..chooser
            },
        ));
        assert!(age_cost > values_cost);
        assert!(values_cost > cognition_cost);
        assert!(cognition_cost > body_cost);
        assert!(body_cost > temperament_cost);
    }

    #[test]
    fn everyone_values_quality_without_seeking_their_own_desirability() {
        let chooser = MateProfile {
            age_permille: 500,
            values: 50,
            cognition: 50,
            body: 50,
            temperament: 50,
            desirability: 10,
        };
        let similar = MateProfile {
            desirability: 10,
            ..chooser
        };
        let desirable = MateProfile {
            desirability: 90,
            ..chooser
        };
        assert!(attraction_score(&chooser, &desirable) > attraction_score(&chooser, &similar));
    }
}
