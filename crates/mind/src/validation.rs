use std::collections::BTreeSet;

use anana_core::{ChanceTemplate, DeterministicKind, EventPayload, HumanId, Permille};

use crate::{AiEffect, AiEventBatch, AiEventKind, AiField, MindError, WorldContext};

pub const MAX_EVENTS_PER_BATCH: usize = 8;
pub const MAX_HEALTH_DELTA: i64 = 10;
pub const MAX_FERTILITY_DELTA: i64 = 20;
pub const MAX_AGE_TICKS_DELTA: i64 = 1_000;
pub const MAX_SKILL_XP: i64 = 5_000;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ValidatedEvent {
    pub subjects: Vec<HumanId>,
    pub payload: EventPayload,
}

#[must_use]
pub fn normalize_effect(effect: &AiEffect) -> AiEffect {
    let bound = match effect.field {
        AiField::Health => MAX_HEALTH_DELTA,
        AiField::Fertility => MAX_FERTILITY_DELTA,
        AiField::AgeTicks => MAX_AGE_TICKS_DELTA,
        AiField::SkillXp => MAX_SKILL_XP,
    };
    AiEffect {
        target: effect.target,
        op: effect.op,
        field: effect.field,
        value: effect.value.clamp(bound.saturating_neg(), bound),
    }
}

fn chance_template(effect: &AiEffect) -> ChanceTemplate {
    match effect.field {
        AiField::Health if effect.value > 0 => ChanceTemplate::Windfall,
        AiField::Health if effect.value < -5 => ChanceTemplate::Accident,
        AiField::Health | AiField::Fertility => ChanceTemplate::Conflict,
        AiField::SkillXp | AiField::AgeTicks => ChanceTemplate::Discovery,
    }
}

fn payload_for(proposal: &crate::AiEventProposal, effect: &AiEffect) -> EventPayload {
    match proposal.kind {
        AiEventKind::Chance => EventPayload::Chance {
            template: chance_template(effect),
            base_prob: Permille::clamp1000(proposal.base_prob.unwrap_or(0)),
            skill_modifier: proposal.skill_modifier,
            modifier_strength: Permille::clamp1000(proposal.modifier_strength.unwrap_or(0)),
        },
        AiEventKind::Deterministic => EventPayload::Deterministic(match effect.field {
            AiField::AgeTicks => DeterministicKind::Maturation,
            AiField::Health | AiField::Fertility | AiField::SkillXp => {
                DeterministicKind::HealthTick
            }
        }),
    }
}

pub fn validate(
    batch: &AiEventBatch,
    context: &WorldContext,
) -> Result<Vec<ValidatedEvent>, MindError> {
    let known = context
        .humans
        .iter()
        .map(|human| human.id)
        .collect::<BTreeSet<_>>();
    let mut validated = Vec::new();
    for proposal in batch.events.iter().take(MAX_EVENTS_PER_BATCH) {
        let subject = HumanId(proposal.subject_id);
        if !known.contains(&subject) {
            continue;
        }
        let effects = proposal
            .effects
            .iter()
            .filter(|effect| known.contains(&HumanId(effect.target)))
            .map(normalize_effect)
            .collect::<Vec<_>>();
        let Some(first_effect) = effects.first() else {
            continue;
        };
        let mut subjects = effects
            .iter()
            .map(|effect| HumanId(effect.target))
            .collect::<BTreeSet<_>>();
        subjects.insert(subject);
        validated.push(ValidatedEvent {
            subjects: subjects.into_iter().collect(),
            payload: payload_for(proposal, first_effect),
        });
    }
    validated.sort_by_key(|event| event.subjects.first().copied().unwrap_or(HumanId(u64::MAX)));
    Ok(validated)
}

#[cfg(test)]
mod tests {
    //! Validation deterministically rejects unknown humans, clamps authored integers, and bounds each batch.

    use anana_core::{ChanceTemplate, EventPayload, HumanId, LifeStage, Sex, SkillId, Tick};

    use crate::{
        AiEffect, AiEventBatch, AiEventKind, AiEventProposal, AiField, AiOp, HumanBrief,
        TraitSummary, WorldContext,
    };

    use super::*;

    fn context() -> WorldContext {
        let traits = TraitSummary {
            eye_color: anana_core::EyeColor::Brown,
            handedness: anana_core::Handedness::Right,
            disease_status: anana_core::DiseaseStatus::Clear,
            robustness: 4,
            aptitude: 4,
        };
        WorldContext {
            tick: Tick(7),
            humans: vec![
                HumanBrief {
                    id: HumanId(1),
                    sex: Sex::Female,
                    life_stage: LifeStage::Adult,
                    age_ticks: 8_000,
                    health: 80,
                    max_health: 100,
                    notable_traits: traits.clone(),
                    top_skills: vec![(SkillId::Planning, 2)],
                    infected: None,
                },
                HumanBrief {
                    id: HumanId(2),
                    sex: Sex::Male,
                    life_stage: LifeStage::Adult,
                    age_ticks: 9_000,
                    health: 90,
                    max_health: 100,
                    notable_traits: traits,
                    top_skills: Vec::new(),
                    infected: None,
                },
            ],
            viruses: Vec::new(),
            recent: Vec::new(),
        }
    }

    fn proposal(subject: u64, target: u64) -> AiEventProposal {
        AiEventProposal {
            subject_id: subject,
            kind: AiEventKind::Chance,
            title: String::from("A sudden accident"),
            description: String::from("The day turns unexpectedly"),
            base_prob: Some(250),
            skill_modifier: Some(SkillId::Planning),
            modifier_strength: Some(50),
            effects: vec![AiEffect {
                target,
                op: AiOp::Add,
                field: AiField::Health,
                value: -7,
            }],
            seed_salt: 3,
        }
    }

    #[test]
    fn a_proposal_naming_an_unknown_human_is_dropped() {
        let validated = validate(
            &AiEventBatch {
                events: vec![proposal(99, 1)],
            },
            &context(),
        )
        .expect("an invalid proposal is dropped without rejecting the batch");
        assert!(validated.is_empty());
    }

    #[test]
    fn an_effect_naming_an_unknown_target_is_dropped_without_losing_valid_targets() {
        let mut candidate = proposal(1, 1);
        candidate.effects.push(AiEffect {
            target: 99,
            op: AiOp::Add,
            field: AiField::Health,
            value: -5,
        });
        let validated = validate(
            &AiEventBatch {
                events: vec![candidate],
            },
            &context(),
        )
        .expect("the valid part of the proposal remains usable");
        assert_eq!(validated.len(), 1);
        assert_eq!(validated[0].subjects, vec![HumanId(1)]);
    }

    #[test]
    fn authored_probabilities_clamp_to_both_permille_bounds() {
        let mut high = proposal(1, 1);
        high.base_prob = Some(50_000);
        high.modifier_strength = Some(50_000);
        let mut low = proposal(2, 2);
        low.base_prob = Some(-1);
        let validated = validate(
            &AiEventBatch {
                events: vec![high, low],
            },
            &context(),
        )
        .expect("bounded probabilities remain usable");
        let probabilities = validated
            .iter()
            .map(|event| match event.payload {
                EventPayload::Chance { base_prob, .. } => base_prob.0,
                _ => 0,
            })
            .collect::<Vec<_>>();
        assert_eq!(probabilities, vec![1_000, 0]);
        assert!(matches!(
            validated[0].payload,
            EventPayload::Chance {
                modifier_strength: anana_core::Permille(1_000),
                ..
            }
        ));
    }

    #[test]
    fn a_proposal_without_any_usable_effect_is_dropped_as_a_no_op() {
        let mut candidate = proposal(1, 99);
        candidate.effects.clear();
        let validated = validate(
            &AiEventBatch {
                events: vec![candidate],
            },
            &context(),
        )
        .expect("an empty proposal does not reject its batch");
        assert!(validated.is_empty());
    }

    #[test]
    fn an_absurd_effect_magnitude_clamps_to_its_declared_field_bound() {
        let effect = normalize_effect(&AiEffect {
            target: 1,
            op: AiOp::Add,
            field: AiField::Health,
            value: 50_000,
        });
        assert_eq!(effect.value, MAX_HEALTH_DELTA);
    }

    #[test]
    fn an_oversized_batch_keeps_only_its_first_bounded_number_of_proposals() {
        let batch = AiEventBatch {
            events: (0..MAX_EVENTS_PER_BATCH + 3)
                .map(|index| proposal(if index.is_multiple_of(2) { 1 } else { 2 }, 1))
                .collect(),
        };
        let validated = validate(&batch, &context()).expect("truncation is a normal outcome");
        assert_eq!(validated.len(), MAX_EVENTS_PER_BATCH);
    }

    #[test]
    fn an_empty_batch_is_valid_and_yields_no_events() {
        assert_eq!(
            validate(&AiEventBatch::default(), &context()),
            Ok(Vec::new())
        );
    }

    #[test]
    fn validated_events_are_sorted_by_human_identifier_not_model_array_order() {
        let validated = validate(
            &AiEventBatch {
                events: vec![proposal(2, 2), proposal(1, 1)],
            },
            &context(),
        )
        .expect("both proposals are valid");
        assert_eq!(
            validated
                .iter()
                .map(|event| event.subjects[0])
                .collect::<Vec<_>>(),
            vec![HumanId(1), HumanId(2)]
        );
        assert!(validated.iter().all(|event| matches!(
            event.payload,
            EventPayload::Chance {
                template: ChanceTemplate::Accident,
                ..
            }
        )));
    }
}
