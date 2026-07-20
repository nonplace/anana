use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{EventRecord, God, GodId, HumanId, HumanState, Tick, Virus, VirusId};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct WorldSnapshot {
    pub seed: u64,
    pub tick: Tick,
    pub next_human_id: HumanId,
    pub humans: BTreeMap<HumanId, HumanState>,
    pub viruses: BTreeMap<VirusId, Virus>,
    pub gods: BTreeMap<GodId, God>,
    pub event_log: Vec<EventRecord>,
}

#[must_use]
pub fn world_hash(snapshot: &WorldSnapshot) -> [u8; 32] {
    match postcard::to_allocvec(snapshot) {
        Ok(bytes) => *blake3::hash(&bytes).as_bytes(),
        Err(_) => [0; 32],
    }
}

#[cfg(test)]
mod tests {
    //! Canonical snapshots hash every world section independent of map insertion order and pin serialization drift.

    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        DeterministicKind, EventAuthor, EventOutcome, EventPayload, EventRecord, God, GodId,
        HumanId, Seq, Tick, Virus, VirusId, fixture_human,
    };

    fn snapshot() -> WorldSnapshot {
        let mut first = fixture_human(HumanId(1));
        first.body.age_ticks = 123;
        first.body.health = 71;
        first.instincts.social = 83;
        first.lineage.generation = 2;
        let second = fixture_human(HumanId(2));
        WorldSnapshot {
            seed: 42,
            tick: Tick(17),
            next_human_id: HumanId(3),
            humans: BTreeMap::from([(HumanId(1), first), (HumanId(2), second)]),
            viruses: BTreeMap::from([(
                VirusId(1),
                Virus {
                    id: VirusId(1),
                    spreadscore: 44,
                    virulence: 12,
                    incubation_ticks: 8,
                    mutation_rate: crate::Permille(2),
                },
            )]),
            gods: BTreeMap::from([(
                GodId(1),
                God {
                    id: GodId(1),
                    goshes_spoken: 3,
                },
            )]),
            event_log: vec![EventRecord {
                tick: Tick(16),
                seq: Seq(4),
                author: EventAuthor::Engine,
                subjects: vec![HumanId(1)],
                payload: EventPayload::Deterministic(DeterministicKind::Maturation),
                outcome: EventOutcome::NoOp,
                narration: Some(String::from("the world turned")),
            }],
        }
    }

    #[test]
    fn the_world_hash_is_blake3_over_the_exact_postcard_bytes_and_stable_across_a_clone() {
        let snapshot = snapshot();
        let bytes = postcard::to_allocvec(&snapshot).expect("snapshot serializes");
        assert_eq!(world_hash(&snapshot), *blake3::hash(&bytes).as_bytes());
        assert_eq!(world_hash(&snapshot), world_hash(&snapshot.clone()));
    }

    #[test]
    fn btree_map_insertion_order_cannot_change_the_world_hash() {
        let first = snapshot();
        let mut second = first.clone();
        second.humans.clear();
        second
            .humans
            .insert(HumanId(2), first.humans[&HumanId(2)].clone());
        second
            .humans
            .insert(HumanId(1), first.humans[&HumanId(1)].clone());
        assert_eq!(world_hash(&first), world_hash(&second));
    }

    #[test]
    fn perturbing_any_canonical_world_section_changes_the_hash() {
        let original = snapshot();
        let expected = world_hash(&original);
        let mut variants = Vec::new();

        let mut changed = original.clone();
        changed.seed = changed.seed.saturating_add(1);
        variants.push(changed);
        let mut changed = original.clone();
        changed.tick.0 = changed.tick.0.saturating_add(1);
        variants.push(changed);
        let mut changed = original.clone();
        changed.next_human_id.0 = changed.next_human_id.0.saturating_add(1);
        variants.push(changed);
        let mut changed = original.clone();
        changed
            .humans
            .get_mut(&HumanId(1))
            .expect("fixture exists")
            .body
            .health = 70;
        variants.push(changed);
        let mut changed = original.clone();
        changed
            .humans
            .get_mut(&HumanId(1))
            .expect("fixture exists")
            .instincts
            .fear = 99;
        variants.push(changed);
        let mut changed = original.clone();
        changed
            .humans
            .get_mut(&HumanId(1))
            .expect("fixture exists")
            .lineage
            .generation = 9;
        variants.push(changed);
        let mut changed = original.clone();
        changed
            .viruses
            .get_mut(&VirusId(1))
            .expect("fixture exists")
            .spreadscore = 45;
        variants.push(changed);
        let mut changed = original.clone();
        changed
            .gods
            .get_mut(&GodId(1))
            .expect("fixture exists")
            .goshes_spoken = 4;
        variants.push(changed);
        let mut changed = original.clone();
        changed.event_log[0].narration = Some(String::from("a different telling"));
        variants.push(changed);

        assert!(
            variants
                .iter()
                .all(|variant| world_hash(variant) != expected)
        );
    }

    #[test]
    fn the_fully_populated_world_matches_the_pinned_golden_hash() {
        assert_eq!(
            world_hash(&snapshot()),
            [
                249, 26, 90, 102, 98, 22, 214, 126, 195, 136, 66, 218, 217, 221, 66, 50, 90, 6,
                144, 142, 95, 187, 30, 25, 38, 82, 214, 245, 23, 66, 124, 118,
            ]
        );
    }
}
