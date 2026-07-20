use anana_core::{
    Body, Consciousness, HumanId, Instincts, LifeStage, ObservationFactors, Permille, Phenotype,
    PracticeKind, Residence, RngDomain, Sex, SkillId, Skills, SocialBonds,
    courtship_aversion_factor, decay_bond, decay_unpractised, min_awareness, observational_gain,
    practise_skill, record_positive_interaction_scaled, teaching_gain,
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

#[derive(Clone)]
struct LearnerSnapshot {
    id: HumanId,
    body: Body,
    instincts: Instincts,
    consciousness: Consciousness,
    skills: Skills,
    residence: Residence,
    sex: Sex,
}

type LearningQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static HumanId,
        &'static Body,
        &'static Instincts,
        &'static Phenotype,
        &'static mut Consciousness,
        &'static mut Skills,
        &'static Residence,
        &'static mut SocialBonds,
    ),
>;

fn reproduction_factor(stage: LifeStage, competence: u16) -> Permille {
    let stage_capacity = match stage {
        LifeStage::Infant => 50_u16,
        LifeStage::Child => 300,
        LifeStage::Adolescent => 700,
        LifeStage::Adult => 1000,
        LifeStage::Elder => 800,
    };
    Permille(
        stage_capacity
            .saturating_add(competence.saturating_mul(2))
            .min(1000),
    )
}

fn observation_factors(
    observer: &LearnerSnapshot,
    model_competence: u16,
    skill: SkillId,
) -> ObservationFactors {
    let observer_competence = u16::from(observer.skills.level_of(skill)).saturating_mul(20);
    let attention = u16::from(observer.consciousness.focus.min(100))
        .saturating_mul(5)
        .saturating_add(u16::from(observer.consciousness.awareness.min(100)).saturating_mul(3))
        .saturating_add(model_competence.saturating_mul(2))
        .min(1000);
    let can_retain = skill == SkillId::Recall || observer.skills.recall_learned();
    let retention = if can_retain {
        observer.consciousness.memory_capacity.min(1000)
    } else {
        0
    };
    let motivation = u16::from(affinity(skill, &observer.instincts).min(100))
        .saturating_mul(7)
        .saturating_add(observer_competence.saturating_mul(3))
        .min(1000);
    ObservationFactors {
        attention: Permille(attention),
        retention: Permille(retention),
        reproduction: reproduction_factor(observer.body.life_stage, observer_competence),
        motivation: Permille(motivation),
    }
}

pub(crate) fn learning(
    clock: Res<'_, WorldClock>,
    rng: Res<'_, SimulationRng>,
    mut humans: LearningQuery<'_, '_>,
) {
    let mut snapshots = humans
        .iter_mut()
        .map(
            |(_, id, body, instincts, phenotype, consciousness, skills, residence, _)| {
                LearnerSnapshot {
                    id: *id,
                    body: body.clone(),
                    instincts: instincts.clone(),
                    consciousness: consciousness.clone(),
                    skills: skills.clone(),
                    residence: *residence,
                    sex: phenotype.sex,
                }
            },
        )
        .collect::<Vec<_>>();
    snapshots.sort_by_key(|human| human.id);
    let entities = humans
        .iter_mut()
        .map(|(entity, id, _, _, _, _, _, _, _)| (*id, entity))
        .collect::<std::collections::BTreeMap<_, _>>();
    for observer in &snapshots {
        let Some(entity) = entities.get(&observer.id).copied() else {
            continue;
        };
        let Ok((
            _,
            _,
            body,
            instincts,
            phenotype,
            mut consciousness,
            mut skills,
            _,
            mut social_bonds,
        )) = humans.get_mut(entity)
        else {
            continue;
        };
        if !body.alive {
            continue;
        }
        for bond in social_bonds.bonds.values_mut() {
            decay_bond(bond, clock.0);
        }
        let co_residents = snapshots
            .iter()
            .filter(|other| {
                other.id != observer.id && other.residence == observer.residence && other.body.alive
            })
            .collect::<Vec<_>>();
        if body.age_ticks < 260 {
            for other in &co_residents {
                social_bonds
                    .rearing_aversions
                    .entry(other.id)
                    .or_default()
                    .observe_co_residence(body.age_ticks);
            }
        }
        if clock.0.0.is_multiple_of(5) {
            let mut social_contacts = co_residents.clone();
            if body.fertility > 0 {
                social_contacts.extend(snapshots.iter().filter(|other| {
                    other.id != observer.id
                        && other.residence != observer.residence
                        && other.sex != observer.sex
                        && other.body.alive
                        && other.body.fertility > 0
                        && other.body.age_ticks.abs_diff(body.age_ticks) <= 800
                }));
                social_contacts.sort_by_key(|other| other.id);
                social_contacts.dedup_by_key(|other| other.id);
            }
            for other in social_contacts {
                let aversion = social_bonds
                    .rearing_aversions
                    .get(&other.id)
                    .map_or(Permille::ZERO, anana_core::RearingAversion::strength);
                let factor = courtship_aversion_factor(aversion, Permille::ZERO);
                let bond = social_bonds.bonds.entry(other.id).or_default();
                record_positive_interaction_scaled(bond, clock.0, Permille::ZERO, factor);
            }
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
        decay_unpractised(&mut skills, clock.0);
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
                observer.id,
                (index as u64).saturating_add(1),
                probability,
            ) {
                let _result = practise_skill(
                    &mut skills,
                    &consciousness,
                    phenotype,
                    skill,
                    20,
                    clock.0,
                    PracticeKind::Restudy,
                );
            }
            if clock.0.0.is_multiple_of(5) {
                let observer_competence =
                    u16::from(observer.skills.level_of(skill)).saturating_mul(20);
                let model = snapshots
                    .iter()
                    .filter(|model| {
                        model.id != observer.id
                            && model.residence == observer.residence
                            && model.body.alive
                            && model.skills.level_of(skill) > observer.skills.level_of(skill)
                    })
                    .max_by_key(|model| {
                        (model.skills.level_of(skill), std::cmp::Reverse(model.id))
                    });
                if let Some(model) = model {
                    let model_competence =
                        u16::from(model.skills.level_of(skill)).saturating_mul(20);
                    let observed = observational_gain(
                        20,
                        model_competence,
                        observer_competence,
                        observation_factors(observer, model_competence, skill),
                    );
                    if observed > 0 {
                        let _result = practise_skill(
                            &mut skills,
                            &consciousness,
                            phenotype,
                            skill,
                            observed,
                            clock.0,
                            PracticeKind::Restudy,
                        );
                    }
                }
            }
            if clock.0.0.is_multiple_of(10) {
                let learner_competence =
                    u16::from(observer.skills.level_of(skill)).saturating_mul(20);
                let lesson = snapshots
                    .iter()
                    .filter(|teacher| {
                        teacher.id != observer.id
                            && teacher.residence == observer.residence
                            && teacher.body.alive
                    })
                    .map(|teacher| {
                        let teacher_competence =
                            u16::from(teacher.skills.level_of(skill)).saturating_mul(20);
                        (
                            teaching_gain(learner_competence, teacher_competence, 30),
                            std::cmp::Reverse(teacher.id),
                        )
                    })
                    .max();
                if let Some((gain, _)) = lesson
                    && gain > 0
                {
                    let _result = practise_skill(
                        &mut skills,
                        &consciousness,
                        phenotype,
                        skill,
                        gain,
                        clock.0,
                        PracticeKind::Retrieval,
                    );
                }
            }
        }
    }
}
