use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    ChanceTemplate, DeterministicKind, Genome, GoshKind, HumanId, Permille, SkillId, VirusId,
};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum EventAuthor {
    Engine,
    Ai,
    God,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum EventPayload {
    Chance {
        template: ChanceTemplate,
        base_prob: Permille,
        skill_modifier: Option<SkillId>,
        modifier_strength: Permille,
    },
    Deterministic(DeterministicKind),
    Gosh(GoshKind),
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum EventOutcome {
    Occurred(BTreeMap<HumanId, EffectSummary>),
    NoOp,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
pub struct EffectSummary {
    pub health_delta: i32,
    pub fertility_delta: i16,
    pub age_ticks_delta: u32,
    pub skill_xp: BTreeMap<SkillId, u32>,
    pub immunities_granted: BTreeSet<VirusId>,
    pub infection: Option<VirusId>,
    pub seeded_genome: Option<Genome>,
}

impl EffectSummary {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}
