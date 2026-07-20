use anana_core::{
    Body, Consciousness, HumanId, Instincts, LifeStage, Permille, Phenotype, RngDomain, SkillId,
    Skills, apply_learning, min_awareness,
};
use bevy::prelude::{Entity, Query, Res};

use crate::{SimulationRng, WorldClock};

const ORDERED_SKILLS: [SkillId; 9] = [
    SkillId::Recall,
    SkillId::Motor,
    SkillId::Language,
    SkillId::Foraging,
    SkillId::ToolUse,
    SkillId::SocialBond,
    SkillId::Farming,
    SkillId::Medicine,
    SkillId::Planning,
];

fn developmental_caps(stage: LifeStage) -> (u8, u8) {
    match stage {
        LifeStage::Infant => (5, 20),
        LifeStage::Child => (25, 45),
        LifeStage::Adolescent => (60, 70),
        LifeStage::Adult => (90, 90),
        LifeStage::Elder => (100, 80),
    }
}

fn affinity(skill: SkillId, instincts: &Instincts) -> u8 {
    match skill {
        SkillId::Recall | SkillId::Language | SkillId::SocialBond => instincts.social,
        SkillId::Motor | SkillId::ToolUse | SkillId::Planning => instincts.survival,
        SkillId::Foraging | SkillId::Farming => instincts.hunger,
        SkillId::Medicine => 100_u8.saturating_sub(instincts.fear.min(100)),
    }
}

pub(crate) fn learning(
    clock: Res<'_, WorldClock>,
    rng: Res<'_, SimulationRng>,
    mut humans: Query<
        '_,
        '_,
        (
            Entity,
            &HumanId,
            &Body,
            &Instincts,
            &Phenotype,
            &mut Consciousness,
            &mut Skills,
        ),
    >,
) {
    let mut ordered = humans
        .iter_mut()
        .map(|(entity, id, _, _, _, _, _)| (*id, entity))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(id, _)| *id);
    for (id, entity) in ordered {
        let Ok((_, _, body, instincts, phenotype, mut consciousness, mut skills)) =
            humans.get_mut(entity)
        else {
            continue;
        };
        if !body.alive {
            continue;
        }
        let (awareness_cap, focus_cap) = developmental_caps(body.life_stage);
        if consciousness.awareness < awareness_cap {
            consciousness.awareness = consciousness.awareness.saturating_add(1).min(awareness_cap);
        }
        if consciousness.focus < focus_cap {
            consciousness.focus = consciousness.focus.saturating_add(1).min(focus_cap);
        } else if consciousness.focus > focus_cap {
            consciousness.focus = consciousness.focus.saturating_sub(1).max(focus_cap);
        }
        for (index, skill) in ORDERED_SKILLS.iter().copied().enumerate() {
            if consciousness.awareness < min_awareness(skill) {
                continue;
            }
            let probability = Permille::clamp1000(
                i64::from(affinity(skill, instincts).min(100)).saturating_mul(5)
                    + i64::from(consciousness.focus.min(100)).saturating_mul(5),
            );
            if rng.0.gate(
                RngDomain::SkillGain,
                clock.0,
                id,
                (index as u64).saturating_add(1),
                probability,
            ) {
                let _result = apply_learning(&mut skills, &consciousness, phenotype, skill, 20);
            }
        }
    }
}
