use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{DeadHuman, EventRecord, God, GodId, HumanId, HumanState, Tick, Virus, VirusId};

const EVENT_LOG_DOMAIN: &[u8] = b"anana-event-log-v2";
const WORLD_DOMAIN: &[u8] = b"anana-world-v2";

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct WorldSnapshot {
    pub seed: u64,
    pub tick: Tick,
    pub next_human_id: HumanId,
    pub next_residence_id: crate::ResidenceId,
    pub humans: BTreeMap<HumanId, HumanState>,
    pub dead: BTreeMap<HumanId, DeadHuman>,
    pub viruses: BTreeMap<VirusId, Virus>,
    pub gods: BTreeMap<GodId, God>,
    pub event_log: Vec<EventRecord>,
}

#[must_use]
pub fn event_log_hash(records: &[EventRecord]) -> [u8; 32] {
    extend_event_log_hash(*blake3::hash(EVENT_LOG_DOMAIN).as_bytes(), records)
}

#[must_use]
pub fn extend_event_log_hash(mut previous: [u8; 32], records: &[EventRecord]) -> [u8; 32] {
    for record in records {
        let Ok(bytes) = postcard::to_allocvec(record) else {
            return [0; 32];
        };
        let mut hasher = blake3::Hasher::new();
        hasher.update(EVENT_LOG_DOMAIN);
        hasher.update(&previous);
        hasher.update(&bytes);
        previous = *hasher.finalize().as_bytes();
    }
    previous
}

#[derive(Serialize)]
struct CanonicalWorld<'a> {
    seed: u64,
    tick: Tick,
    next_human_id: HumanId,
    next_residence_id: crate::ResidenceId,
    humans: &'a BTreeMap<HumanId, HumanState>,
    dead: &'a BTreeMap<HumanId, DeadHuman>,
    viruses: &'a BTreeMap<VirusId, Virus>,
    gods: &'a BTreeMap<GodId, God>,
    event_log_hash: [u8; 32],
}

#[must_use]
pub fn world_hash_with_event_log_hash(
    snapshot: &WorldSnapshot,
    event_log_hash: [u8; 32],
) -> [u8; 32] {
    let canonical = CanonicalWorld {
        seed: snapshot.seed,
        tick: snapshot.tick,
        next_human_id: snapshot.next_human_id,
        next_residence_id: snapshot.next_residence_id,
        humans: &snapshot.humans,
        dead: &snapshot.dead,
        viruses: &snapshot.viruses,
        gods: &snapshot.gods,
        event_log_hash,
    };
    match postcard::to_allocvec(&canonical) {
        Ok(bytes) => {
            let mut hasher = blake3::Hasher::new();
            hasher.update(WORLD_DOMAIN);
            hasher.update(&bytes);
            *hasher.finalize().as_bytes()
        }
        Err(_) => [0; 32],
    }
}

#[must_use]
pub fn world_hash(snapshot: &WorldSnapshot) -> [u8; 32] {
    world_hash_with_event_log_hash(snapshot, event_log_hash(&snapshot.event_log))
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
            next_residence_id: crate::ResidenceId(2),
            humans: BTreeMap::from([(HumanId(1), first), (HumanId(2), second)]),
            dead: BTreeMap::new(),
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
    fn the_world_hash_composes_canonical_state_and_log_and_is_stable_across_a_clone() {
        let snapshot = snapshot();
        assert_eq!(
            world_hash(&snapshot),
            world_hash_with_event_log_hash(&snapshot, event_log_hash(&snapshot.event_log))
        );
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
        changed.next_residence_id.0 = changed.next_residence_id.0.saturating_add(1);
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
            .humans
            .get_mut(&HumanId(1))
            .expect("fixture exists")
            .residence
            .id = crate::ResidenceId(8);
        variants.push(changed);
        let mut changed = original.clone();
        changed
            .humans
            .get_mut(&HumanId(1))
            .expect("fixture exists")
            .skills
            .memories
            .insert(
                crate::SkillId::Motor,
                crate::SkillMemory {
                    stability: 10,
                    ..crate::SkillMemory::default()
                },
            );
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

        let mut changed = original.clone();
        changed.dead.insert(
            HumanId(9),
            crate::DeadHuman {
                id: HumanId(9),
                lineage: crate::Lineage::new(HumanId(9), None, None, 0, Tick(0)),
                generation: 0,
                birth_tick: Tick(0),
                death_tick: Tick(12),
                skills: crate::Skills::default(),
            },
        );
        variants.push(changed);

        assert!(
            variants
                .iter()
                .all(|variant| world_hash(variant) != expected)
        );
    }

    #[test]
    fn extending_the_event_digest_matches_hashing_the_complete_log() {
        let snapshot = snapshot();
        let first = event_log_hash(&snapshot.event_log[..0]);
        let extended = extend_event_log_hash(first, &snapshot.event_log);
        assert_eq!(extended, event_log_hash(&snapshot.event_log));
        assert_eq!(
            world_hash_with_event_log_hash(&snapshot, extended),
            world_hash(&snapshot)
        );
    }

    #[test]
    fn the_fully_populated_world_matches_the_pinned_golden_hash() {
        assert_eq!(
            world_hash(&snapshot()),
            [
                241, 241, 11, 194, 109, 59, 110, 172, 252, 239, 220, 237, 61, 82, 100, 127, 98,
                231, 108, 38, 218, 10, 168, 188, 5, 164, 180, 214, 122, 28, 4, 108,
            ]
        );
    }
}
