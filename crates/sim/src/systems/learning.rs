use anana_core::{
    Body, Consciousness, GroupResponse, HumanId, Instincts, LifeStage, Lineage, ObservationFactors,
    Permille, Phenotype, PracticeKind, Residence, RngDomain, Sex, SkillId, Skills, SocialBonds,
    are_first_degree_relatives, coalition_cooperation, courtship_aversion_factor, decay_bond,
    decay_unpractised, deference_value, derive_coalitions, group_response, layer_effort,
    min_awareness, observational_gain, practise_skill, prestige_of,
    record_positive_interaction_scaled, relationship_layer, teaching_gain, trim_to_social_capacity,
    unfamiliar_attention,
};
use bevy::prelude::{Entity, Query, Res, ResMut};

use crate::{Coalitions, Config, NextResidenceId, SimulationFaults, SimulationRng, WorldClock};

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
    phenotype: Phenotype,
    lineage: Lineage,
    social_bonds: SocialBonds,
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
        &'static Lineage,
        &'static mut Consciousness,
        &'static mut Skills,
        &'static mut Residence,
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
    model: &LearnerSnapshot,
    model_competence: u16,
    model_prestige: u32,
    skill: SkillId,
) -> ObservationFactors {
    let observer_competence = u16::from(observer.skills.level_of(skill)).saturating_mul(20);
    let base_attention = u16::from(observer.consciousness.focus.min(100))
        .saturating_mul(5)
        .saturating_add(u16::from(observer.consciousness.awareness.min(100)).saturating_mul(3))
        .saturating_add(model_competence.saturating_mul(2))
        .saturating_add(model_prestige.min(1000) as u16 / 5)
        .min(1000);
    let attachment = observer
        .social_bonds
        .bonds
        .get(&model.id)
        .map_or(Permille::ZERO, |bond| bond.strength);
    let attention = unfamiliar_attention(
        Permille(base_attention),
        are_first_degree_relatives(&observer.lineage, &model.lineage),
        attachment,
        observer.phenotype.novelty_tolerance,
    );
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
        attention,
        retention: Permille(retention),
        reproduction: reproduction_factor(observer.body.life_stage, observer_competence),
        motivation: Permille(motivation),
    }
}

fn demonstrated_competence(skills: &Skills) -> u16 {
    ORDERED_SKILLS
        .iter()
        .map(|skill| u16::from(skills.level_of(*skill)).saturating_mul(200))
        .max()
        .unwrap_or(0)
}

fn update_coalitions_and_groups(
    capacity: usize,
    prestige: &std::collections::BTreeMap<HumanId, u32>,
    entities: &std::collections::BTreeMap<HumanId, Entity>,
    humans: &mut LearningQuery<'_, '_>,
    next_residence: &mut NextResidenceId,
    faults: &mut SimulationFaults,
    coalitions_resource: &mut Coalitions,
) {
    let state = humans
        .iter_mut()
        .map(|(_, id, _, _, _, _, _, _, residence, social)| (*id, (*residence, social.clone())))
        .collect::<std::collections::BTreeMap<_, _>>();
    let social = state
        .iter()
        .map(|(id, (_, bonds))| (*id, bonds.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut coalitions = derive_coalitions(&social);
    let mut groups = std::collections::BTreeMap::<anana_core::ResidenceId, Vec<HumanId>>::new();
    for (id, (residence, _)) in &state {
        groups.entry(residence.id).or_default().push(*id);
    }
    for members in groups.values_mut() {
        members.sort_unstable();
        if members.len() <= capacity.max(1) {
            continue;
        }
        let member_set = members
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let possible = members
            .len()
            .saturating_mul(members.len().saturating_sub(1));
        let connections = members
            .iter()
            .filter_map(|id| social.get(id))
            .flat_map(|bonds| bonds.bonds.keys())
            .filter(|other| member_set.contains(other))
            .count();
        let density = Permille::clamp1000(
            connections
                .saturating_mul(1000)
                .checked_div(possible)
                .and_then(|value| i64::try_from(value).ok())
                .unwrap_or(0),
        );
        let total_prestige = members.iter().fold(0_u64, |sum, id| {
            sum.saturating_add(u64::from(prestige.get(id).copied().unwrap_or(0)))
        });
        let highest_prestige = members
            .iter()
            .filter_map(|id| prestige.get(id))
            .copied()
            .max()
            .unwrap_or(0);
        let concentration = Permille::clamp1000(
            u64::from(highest_prestige)
                .saturating_mul(1000)
                .checked_div(total_prestige)
                .and_then(|value| i64::try_from(value).ok())
                .unwrap_or(0),
        );
        match group_response(density, concentration) {
            GroupResponse::Fission => {
                let residence = match next_residence.allocate() {
                    Ok(residence) => residence,
                    Err(error) => {
                        faults.0.push(error);
                        continue;
                    }
                };
                let mut ranked = members
                    .iter()
                    .map(|id| {
                        let connection_strength = social.get(id).map_or(0_u32, |bonds| {
                            bonds
                                .bonds
                                .iter()
                                .filter(|(other, _)| member_set.contains(other))
                                .fold(0_u32, |sum, (_, bond)| {
                                    sum.saturating_add(u32::from(bond.strength.0.min(1000)))
                                })
                        });
                        (*id, connection_strength)
                    })
                    .collect::<Vec<_>>();
                ranked.sort_by_key(|(id, strength)| (*strength, *id));
                let moving = ranked.len() / 2;
                for (id, _) in ranked.into_iter().take(moving.max(1)) {
                    let Some(entity) = entities.get(&id).copied() else {
                        continue;
                    };
                    if let Ok((_, _, _, _, _, _, _, _, mut current_residence, _)) =
                        humans.get_mut(entity)
                    {
                        current_residence.id = residence;
                    }
                }
            }
            GroupResponse::Stratify => {
                let id = anana_core::CoalitionId(members.first().map_or(0, |id| id.0));
                let coalition = coalitions
                    .entry(id)
                    .or_insert_with(|| anana_core::Coalition {
                        id,
                        members: member_set,
                        stratified: false,
                        mediators: std::collections::BTreeSet::new(),
                    });
                coalition.stratified = true;
                let mut ranked = members.clone();
                ranked.sort_by_key(|id| {
                    (
                        std::cmp::Reverse(prestige.get(id).copied().unwrap_or(0)),
                        *id,
                    )
                });
                coalition.mediators = ranked.into_iter().take(5).collect();
            }
        }
    }
    coalitions_resource.0 = coalitions;
}

pub(crate) fn learning(
    config: Res<'_, Config>,
    clock: Res<'_, WorldClock>,
    rng: Res<'_, SimulationRng>,
    mut next_residence: ResMut<'_, NextResidenceId>,
    mut faults: ResMut<'_, SimulationFaults>,
    mut coalitions_resource: ResMut<'_, Coalitions>,
    mut humans: LearningQuery<'_, '_>,
) {
    let mut snapshots = humans
        .iter_mut()
        .map(
            |(
                _,
                id,
                body,
                instincts,
                phenotype,
                lineage,
                consciousness,
                skills,
                residence,
                social_bonds,
            )| {
                LearnerSnapshot {
                    id: *id,
                    body: body.clone(),
                    instincts: instincts.clone(),
                    consciousness: consciousness.clone(),
                    skills: skills.clone(),
                    residence: *residence,
                    sex: phenotype.sex,
                    phenotype: phenotype.clone(),
                    lineage: lineage.clone(),
                    social_bonds: social_bonds.clone(),
                }
            },
        )
        .collect::<Vec<_>>();
    snapshots.sort_by_key(|human| human.id);
    let living = snapshots
        .iter()
        .filter(|human| human.body.alive)
        .map(|human| human.id)
        .collect::<std::collections::BTreeSet<_>>();
    let ledgers = snapshots
        .iter()
        .map(|human| (human.id, human.social_bonds.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let prestige = if clock.0.0.is_multiple_of(5) {
        snapshots
            .iter()
            .map(|human| (human.id, prestige_of(human.id, &ledgers, &living)))
            .collect::<std::collections::BTreeMap<_, _>>()
    } else {
        std::collections::BTreeMap::new()
    };
    let coalitions_before = coalitions_resource.0.clone();
    let mut coalition_membership = std::collections::BTreeMap::new();
    for (id, coalition) in &coalitions_before {
        let cooperation = coalition_cooperation(&coalition.members, &prestige);
        for member in &coalition.members {
            coalition_membership.insert(*member, (*id, cooperation));
        }
    }
    let relationship_efforts = if clock.0.0.is_multiple_of(5) {
        snapshots
            .iter()
            .map(|observer| {
                let mut ranked = observer.social_bonds.bonds.iter().collect::<Vec<_>>();
                ranked.sort_by_key(|(id, bond)| (std::cmp::Reverse(bond.strength), **id));
                let efforts = ranked
                    .into_iter()
                    .enumerate()
                    .map(|(rank, (id, bond))| {
                        (
                            *id,
                            relationship_layer(rank, bond, clock.0)
                                .map_or(Permille(20), layer_effort),
                        )
                    })
                    .collect::<std::collections::BTreeMap<_, _>>();
                (observer.id, efforts)
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    } else {
        std::collections::BTreeMap::new()
    };
    let entities = humans
        .iter_mut()
        .map(|(entity, id, _, _, _, _, _, _, _, _)| (*id, entity))
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
            _,
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
                let mut factor = courtship_aversion_factor(aversion, Permille::ZERO);
                if let (Some((observer_coalition, cooperation)), Some((other_coalition, _))) = (
                    coalition_membership.get(&observer.id),
                    coalition_membership.get(&other.id),
                ) && observer_coalition == other_coalition
                {
                    factor = factor.and(*cooperation);
                }
                let bond = social_bonds.bonds.entry(other.id).or_default();
                let model_prestige = Permille::clamp1000(i64::from(
                    prestige.get(&other.id).copied().unwrap_or(0).min(1000),
                ));
                record_positive_interaction_scaled(bond, clock.0, model_prestige, factor);
                social_bonds
                    .observed_competence
                    .entry(other.id)
                    .and_modify(|value| {
                        *value = (*value).max(demonstrated_competence(&other.skills));
                    })
                    .or_insert_with(|| demonstrated_competence(&other.skills));
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
                            && observer.social_bonds.bonds.contains_key(&model.id)
                            && model.skills.level_of(skill) > observer.skills.level_of(skill)
                    })
                    .max_by_key(|model| {
                        (
                            model.skills.level_of(skill),
                            prestige.get(&model.id).copied().unwrap_or(0),
                            std::cmp::Reverse(model.id),
                        )
                    });
                if let Some(model) = model {
                    let model_competence =
                        u16::from(model.skills.level_of(skill)).saturating_mul(20);
                    let raw_observed = observational_gain(
                        20,
                        model_competence,
                        observer_competence,
                        observation_factors(
                            observer,
                            model,
                            model_competence,
                            prestige.get(&model.id).copied().unwrap_or(0),
                            skill,
                        ),
                    );
                    let observed = raw_observed
                        .saturating_mul(u32::from(
                            relationship_efforts
                                .get(&observer.id)
                                .and_then(|efforts| efforts.get(&model.id))
                                .copied()
                                .unwrap_or(Permille(20))
                                .0,
                        ))
                        .saturating_div(400);
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
                            && observer.social_bonds.bonds.contains_key(&teacher.id)
                    })
                    .map(|teacher| {
                        let teacher_competence =
                            u16::from(teacher.skills.level_of(skill)).saturating_mul(20);
                        let gain = teaching_gain(learner_competence, teacher_competence, 30);
                        let effort = u32::from(
                            relationship_efforts
                                .get(&observer.id)
                                .and_then(|efforts| efforts.get(&teacher.id))
                                .copied()
                                .unwrap_or(Permille(20))
                                .0,
                        );
                        let coalition_bonus = if coalition_membership
                            .get(&observer.id)
                            .zip(coalition_membership.get(&teacher.id))
                            .is_some_and(|(first, second)| first.0 == second.0)
                        {
                            1250_u32
                        } else {
                            1000
                        };
                        (
                            gain.saturating_mul(effort)
                                .saturating_mul(coalition_bonus)
                                .saturating_div(400_000),
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
        if clock.0.0.is_multiple_of(10) {
            let mut choices = social_bonds
                .observed_competence
                .iter()
                .filter(|(id, _)| social_bonds.bonds.contains_key(id))
                .map(|(id, competence)| {
                    (
                        *id,
                        deference_value(*competence, prestige.get(id).copied().unwrap_or(0)),
                    )
                })
                .collect::<Vec<_>>();
            choices.sort_by_key(|(id, value)| (std::cmp::Reverse(*value), *id));
            social_bonds.deference.clear();
            social_bonds.deference.extend(choices.into_iter().take(5));
        }
        if clock.0.0.is_multiple_of(5) {
            trim_to_social_capacity(&mut social_bonds, config.social_capacity);
        }
    }
    if clock.0.0.is_multiple_of(10) {
        update_coalitions_and_groups(
            config.social_capacity,
            &prestige,
            &entities,
            &mut humans,
            &mut next_residence,
            &mut faults,
            &mut coalitions_resource,
        );
    }
}
