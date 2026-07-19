use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::{HumanId, Tick};

const DOMAIN_SEPARATOR: &[u8] = b"anana-keyed-rng-v1";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub struct Permille(pub u16);

impl Permille {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1000);

    #[must_use]
    pub fn clamp1000(value: i64) -> Self {
        Self(value.clamp(0, 1000) as u16)
    }

    #[must_use]
    pub fn and(self, other: Self) -> Self {
        let left = u32::from(self.0.min(1000));
        let right = u32::from(other.0.min(1000));
        Self(((left * right) / 1000) as u16)
    }

    #[must_use]
    pub fn complement(self) -> Self {
        Self(1000_u16.saturating_sub(self.0.min(1000)))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[repr(u8)]
pub enum RngDomain {
    Meiosis = 0,
    Penetrance = 1,
    SexDetermination = 2,
    SkillGain = 3,
    Mating = 4,
    Infection = 5,
    Mutation = 6,
    Event = 7,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Rng {
    pub master_seed: u64,
}

impl Rng {
    #[must_use]
    pub const fn new(master_seed: u64) -> Self {
        Self { master_seed }
    }

    #[must_use]
    pub fn draw_u64(&self, domain: RngDomain, tick: Tick, subject: HumanId, salt: u64) -> u64 {
        let mut hasher = blake3::Hasher::new();
        hasher.update(DOMAIN_SEPARATOR);
        hasher.update(&self.master_seed.to_le_bytes());
        hasher.update(&[domain as u8]);
        hasher.update(&tick.0.to_le_bytes());
        hasher.update(&subject.0.to_le_bytes());
        hasher.update(&salt.to_le_bytes());
        let mut stream = ChaCha8Rng::from_seed(*hasher.finalize().as_bytes());
        stream.next_u64()
    }

    #[must_use]
    pub fn coin(&self, domain: RngDomain, tick: Tick, subject: HumanId, salt: u64) -> bool {
        self.draw_u64(domain, tick, subject, salt) & 1 == 1
    }

    #[must_use]
    pub fn gate(
        &self,
        domain: RngDomain,
        tick: Tick,
        subject: HumanId,
        salt: u64,
        probability: Permille,
    ) -> bool {
        let probability = u64::from(probability.0.min(1000));
        self.draw_u64(domain, tick, subject, salt) % 1000 < probability
    }
}

#[cfg(test)]
mod tests {
    //! Keyed draws are reproducible, order-independent, and pinned to one exact random stream.

    use std::collections::BTreeSet;

    use super::*;
    const SEED: u64 = 0xA11C_E5EED;
    const TICK: Tick = Tick(37);
    const SUBJECT: HumanId = HumanId(42);
    const SALT: u64 = 9;

    #[test]
    fn a_draw_is_reproducible_and_unaffected_by_unrelated_draws() {
        let rng = Rng::new(SEED);
        let first = rng.draw_u64(RngDomain::Meiosis, TICK, SUBJECT, SALT);
        let _unrelated = rng.draw_u64(RngDomain::Event, Tick(999), HumanId(7), 88);
        let repeated = rng.draw_u64(RngDomain::Meiosis, TICK, SUBJECT, SALT);
        assert_eq!(first, repeated);
    }

    #[test]
    fn changing_any_key_component_changes_the_draw() {
        let baseline = Rng::new(SEED).draw_u64(RngDomain::Meiosis, TICK, SUBJECT, SALT);
        let variants = BTreeSet::from([
            Rng::new(SEED + 1).draw_u64(RngDomain::Meiosis, TICK, SUBJECT, SALT),
            Rng::new(SEED).draw_u64(RngDomain::Mutation, TICK, SUBJECT, SALT),
            Rng::new(SEED).draw_u64(RngDomain::Meiosis, Tick(TICK.0 + 1), SUBJECT, SALT),
            Rng::new(SEED).draw_u64(RngDomain::Meiosis, TICK, HumanId(SUBJECT.0 + 1), SALT),
            Rng::new(SEED).draw_u64(RngDomain::Meiosis, TICK, SUBJECT, SALT + 1),
        ]);
        assert_eq!(variants.len(), 5);
        assert!(variants.iter().all(|draw| *draw != baseline));
    }

    #[test]
    fn coin_and_gate_are_views_of_the_same_keyed_draw() {
        let rng = Rng::new(SEED);
        let draw = rng.draw_u64(RngDomain::SkillGain, TICK, SUBJECT, SALT);
        assert_eq!(
            rng.coin(RngDomain::SkillGain, TICK, SUBJECT, SALT),
            draw & 1 == 1
        );
        assert_eq!(
            rng.gate(RngDomain::SkillGain, TICK, SUBJECT, SALT, Permille(437)),
            draw % 1000 < 437
        );
    }

    #[test]
    fn zero_probability_never_fires_and_one_always_fires() {
        let rng = Rng::new(SEED);
        for salt in 0..100 {
            assert!(!rng.gate(RngDomain::Event, TICK, SUBJECT, salt, Permille::ZERO));
            assert!(rng.gate(RngDomain::Event, TICK, SUBJECT, salt, Permille::ONE));
        }
    }

    #[test]
    fn fixed_point_combinators_clamp_hostile_values() {
        assert_eq!(Permille::clamp1000(-1), Permille::ZERO);
        assert_eq!(Permille::clamp1000(1001), Permille::ONE);
        assert_eq!(Permille(500).and(Permille(400)), Permille(200));
        assert_eq!(Permille(u16::MAX).and(Permille(u16::MAX)), Permille::ONE);
        assert_eq!(Permille(250).complement(), Permille(750));
        assert_eq!(Permille(u16::MAX).complement(), Permille::ZERO);
    }

    #[test]
    fn the_random_stream_matches_the_pinned_golden_draw() {
        let draw = Rng::new(SEED).draw_u64(RngDomain::Meiosis, TICK, SUBJECT, SALT);
        assert_eq!(draw, 12_695_985_738_939_563_246);
    }
}
