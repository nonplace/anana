use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{Bond, HumanId, Permille, SocialBonds, Tick};

pub const DEFAULT_SOCIAL_CAPACITY: usize = 150;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
pub enum SocialLayer {
    Support,
    Sympathy,
    Affinity,
    Active,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
pub struct CoalitionId(pub u64);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Coalition {
    pub id: CoalitionId,
    pub members: BTreeSet<HumanId>,
    pub stratified: bool,
    pub mediators: BTreeSet<HumanId>,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum GroupResponse {
    Fission,
    Stratify,
}

#[must_use]
pub fn deference_value(observed_competence: u16, existing_prestige: u32) -> Permille {
    let direct = u32::from(observed_competence.min(1000)).saturating_mul(7);
    let social = existing_prestige.min(1000).saturating_mul(3);
    Permille::clamp1000(i64::from(direct.saturating_add(social) / 10))
}

#[must_use]
pub fn prestige_of(
    subject: HumanId,
    ledgers: &BTreeMap<HumanId, SocialBonds>,
    living: &BTreeSet<HumanId>,
) -> u32 {
    ledgers
        .iter()
        .filter(|(conferrer, _)| living.contains(conferrer))
        .filter_map(|(_, social)| social.deference.get(&subject))
        .fold(0_u32, |sum, value| {
            sum.saturating_add(u32::from(value.0.min(1000)))
        })
}

pub fn trim_to_social_capacity(social: &mut SocialBonds, capacity: usize) {
    let mut ranked = social
        .bonds
        .iter()
        .map(|(id, bond)| (*id, bond.strength))
        .collect::<Vec<_>>();
    ranked.sort_by_key(|(id, strength)| (std::cmp::Reverse(*strength), *id));
    let retained = ranked
        .into_iter()
        .take(capacity)
        .map(|(id, _)| id)
        .collect::<BTreeSet<_>>();
    social.bonds.retain(|id, _| retained.contains(id));
    social.deference.retain(|id, _| retained.contains(id));
    social
        .observed_competence
        .retain(|id, _| retained.contains(id));
}

fn next_layer(layer: SocialLayer) -> Option<SocialLayer> {
    match layer {
        SocialLayer::Support => Some(SocialLayer::Sympathy),
        SocialLayer::Sympathy => Some(SocialLayer::Affinity),
        SocialLayer::Affinity => Some(SocialLayer::Active),
        SocialLayer::Active => None,
    }
}

fn maintenance_ticks(layer: SocialLayer) -> u64 {
    match layer {
        SocialLayer::Support => 5,
        SocialLayer::Sympathy => 15,
        SocialLayer::Affinity => 50,
        SocialLayer::Active => 150,
    }
}

#[must_use]
pub fn relationship_layer(rank: usize, bond: &Bond, now: Tick) -> Option<SocialLayer> {
    let mut layer = match rank {
        0..=4 => SocialLayer::Support,
        5..=14 => SocialLayer::Sympathy,
        15..=49 => SocialLayer::Affinity,
        50..=149 => SocialLayer::Active,
        _ => return None,
    };
    let elapsed = now.0.saturating_sub(bond.last_interaction.0);
    while elapsed > maintenance_ticks(layer) {
        layer = next_layer(layer)?;
    }
    Some(layer)
}

#[must_use]
pub const fn layer_effort(layer: SocialLayer) -> Permille {
    match layer {
        SocialLayer::Support => Permille(400),
        SocialLayer::Sympathy => Permille(200),
        SocialLayer::Affinity => Permille(100),
        SocialLayer::Active => Permille(40),
    }
}

#[must_use]
pub fn derive_coalitions(
    social: &BTreeMap<HumanId, SocialBonds>,
) -> BTreeMap<CoalitionId, Coalition> {
    let mut unvisited = social.keys().copied().collect::<BTreeSet<_>>();
    let mut coalitions = BTreeMap::new();
    while let Some(root) = unvisited.first().copied() {
        unvisited.remove(&root);
        let mut members = BTreeSet::new();
        let mut stack = vec![root];
        while let Some(current) = stack.pop() {
            if !members.insert(current) {
                continue;
            }
            let Some(current_social) = social.get(&current) else {
                continue;
            };
            for (other, bond) in &current_social.bonds {
                let mutual = social
                    .get(other)
                    .and_then(|other_social| other_social.bonds.get(&current));
                if bond.strength.0 >= 700
                    && mutual.is_some_and(|other_bond| other_bond.strength.0 >= 700)
                    && unvisited.remove(other)
                {
                    stack.push(*other);
                }
            }
        }
        if members.len() >= 2 {
            let id = CoalitionId(root.0);
            coalitions.insert(
                id,
                Coalition {
                    id,
                    members,
                    stratified: false,
                    mediators: BTreeSet::new(),
                },
            );
        }
    }
    coalitions
}

#[must_use]
pub fn coalition_cooperation(
    members: &BTreeSet<HumanId>,
    prestige: &BTreeMap<HumanId, u32>,
) -> Permille {
    let total = members.iter().fold(0_u64, |sum, id| {
        sum.saturating_add(u64::from(prestige.get(id).copied().unwrap_or(0)))
    });
    if total == 0 {
        return Permille::ONE;
    }
    let highest = members
        .iter()
        .filter_map(|id| prestige.get(id))
        .copied()
        .max()
        .unwrap_or(0);
    let concentration = u64::from(highest).saturating_mul(1000) / total;
    Permille::clamp1000(1000_i64.saturating_sub(concentration as i64))
}

#[must_use]
pub fn group_response(bond_density: Permille, prestige_concentration: Permille) -> GroupResponse {
    if prestige_concentration.0 >= 500 || bond_density.0 < 500 {
        GroupResponse::Stratify
    } else {
        GroupResponse::Fission
    }
}

#[cfg(test)]
mod tests {
    //! Prestige is proved to be freely conferred and revocable, while bounded networks,
    //! coalitions, and group responses emerge from relationship state rather than labels.

    use std::collections::{BTreeMap, BTreeSet};

    use crate::{Bond, HumanId, Permille, SocialBonds, Tick};

    use super::*;

    fn ledger(entries: &[(u64, u16)]) -> SocialBonds {
        SocialBonds {
            deference: entries
                .iter()
                .map(|(id, value)| (HumanId(*id), Permille(*value)))
                .collect(),
            ..SocialBonds::default()
        }
    }

    #[test]
    fn prestige_is_the_sum_of_living_peoples_revocable_deference() {
        let ledgers = BTreeMap::from([
            (HumanId(1), ledger(&[(9, 300)])),
            (HumanId(2), ledger(&[(9, 400)])),
        ]);
        let everyone = BTreeSet::from([HumanId(1), HumanId(2), HumanId(9)]);
        let after_one_dies = BTreeSet::from([HumanId(2), HumanId(9)]);
        assert_eq!(prestige_of(HumanId(9), &ledgers, &everyone), 700);
        assert_eq!(prestige_of(HumanId(9), &ledgers, &after_one_dies), 400);
    }

    #[test]
    fn equally_competent_people_differ_when_only_one_was_observed() {
        let observed = deference_value(800, 0);
        let obscure = deference_value(0, 0);
        assert!(observed > obscure);
    }

    #[test]
    fn existing_deference_compounds_an_early_prestige_lead() {
        let first_round = deference_value(600, 0);
        let second_round = deference_value(600, u32::from(first_round.0));
        assert!(second_round > first_round);
    }

    #[test]
    fn prestige_derivation_cannot_coerce_or_mutate_a_non_conferrer() {
        let untouched = ledger(&[]);
        let before = untouched.clone();
        let ledgers = BTreeMap::from([(HumanId(1), ledger(&[(9, 1000)])), (HumanId(2), untouched)]);
        let living = BTreeSet::from([HumanId(1), HumanId(2), HumanId(9)]);
        assert_eq!(prestige_of(HumanId(9), &ledgers, &living), 1000);
        assert_eq!(ledgers[&HumanId(2)], before);
    }

    #[test]
    fn active_relationships_never_exceed_capacity_and_break_ties_by_identifier() {
        let mut social = SocialBonds::default();
        for id in (1..=170).rev() {
            social.bonds.insert(
                HumanId(id),
                Bond {
                    strength: Permille(500),
                    last_interaction: Tick(10),
                    last_decay_tick: Tick(10),
                    positive_interactions: 3,
                    defections: 0,
                },
            );
        }
        trim_to_social_capacity(&mut social, 150);
        assert_eq!(social.bonds.len(), 150);
        assert!(social.bonds.contains_key(&HumanId(1)));
        assert!(!social.bonds.contains_key(&HumanId(170)));
    }

    #[test]
    fn a_relationship_demotes_itself_when_contact_no_longer_maintains_its_layer() {
        let bond = Bond {
            strength: Permille::ONE,
            last_interaction: Tick(1),
            last_decay_tick: Tick(1),
            positive_interactions: 20,
            defections: 0,
        };
        assert_eq!(
            relationship_layer(0, &bond, Tick(2)),
            Some(SocialLayer::Support)
        );
        assert_eq!(
            relationship_layer(0, &bond, Tick(20)),
            Some(SocialLayer::Affinity)
        );
        assert_eq!(relationship_layer(0, &bond, Tick(200)), None);
    }

    #[test]
    fn social_effort_is_concentrated_in_the_innermost_layers() {
        assert_eq!(layer_effort(SocialLayer::Support), Permille(400));
        assert!(layer_effort(SocialLayer::Support) > layer_effort(SocialLayer::Sympathy));
        assert!(layer_effort(SocialLayer::Sympathy) > layer_effort(SocialLayer::Active));
    }

    #[test]
    fn mutual_strong_bonds_form_a_canonical_coalition() {
        let strong = Bond {
            strength: Permille(900),
            ..Bond::default()
        };
        let mut first = SocialBonds::default();
        first.bonds.insert(HumanId(2), strong.clone());
        let mut second = SocialBonds::default();
        second.bonds.insert(HumanId(1), strong);
        let coalitions = derive_coalitions(&BTreeMap::from([
            (HumanId(1), first),
            (HumanId(2), second),
            (HumanId(3), SocialBonds::default()),
        ]));
        assert_eq!(coalitions.len(), 1);
        assert_eq!(
            coalitions[&CoalitionId(1)].members,
            BTreeSet::from([HumanId(1), HumanId(2)])
        );
    }

    #[test]
    fn cooperation_falls_as_prestige_concentrates_inside_a_coalition() {
        let members = BTreeSet::from([HumanId(1), HumanId(2), HumanId(3)]);
        let flat = BTreeMap::from([(HumanId(1), 100), (HumanId(2), 100), (HumanId(3), 100)]);
        let steep = BTreeMap::from([(HumanId(1), 290), (HumanId(2), 5), (HumanId(3), 5)]);
        assert!(coalition_cooperation(&members, &flat) > coalition_cooperation(&members, &steep));
    }

    #[test]
    fn dense_flat_groups_fission_while_concentrated_groups_stratify() {
        assert_eq!(
            group_response(Permille(800), Permille(100)),
            GroupResponse::Fission
        );
        assert_eq!(
            group_response(Permille(400), Permille(800)),
            GroupResponse::Stratify
        );
    }
}
