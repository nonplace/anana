mod chance;
#[path = "event.rs"]
mod data;
mod deterministic;

pub use chance::*;
pub use data::*;
pub use deterministic::*;

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    HumanId, HumanState, Permille, Rng, RngDomain, Seq, SkillId, Tick, WorldView, learning_gain,
    resolve_gosh,
};

#[derive(Clone, Copy)]
struct ChanceSpec {
    template: ChanceTemplate,
    base_prob: Permille,
    skill_modifier: Option<SkillId>,
    modifier_strength: Permille,
}

#[derive(Clone, Copy)]
struct ResolutionKey<'a> {
    rng: &'a Rng,
    tick: Tick,
    seq: Seq,
}

fn existing_subjects(view: &WorldView<'_>) -> BTreeSet<HumanId> {
    view.subjects
        .iter()
        .copied()
        .filter(|id| view.humans.contains_key(id))
        .collect()
}

fn outcome_from_effects(effects: BTreeMap<HumanId, EffectSummary>) -> EventOutcome {
    if effects.is_empty() {
        EventOutcome::NoOp
    } else {
        EventOutcome::Occurred(effects)
    }
}

fn chance_effect(
    template: ChanceTemplate,
    skill_modifier: Option<SkillId>,
    human: &HumanState,
) -> Option<EffectSummary> {
    let mut effect = EffectSummary::default();
    match template {
        ChanceTemplate::Accident => {
            effect.health_delta = -i32::from(human.body.health.min(10));
        }
        ChanceTemplate::Conflict => {
            effect.health_delta = -i32::from(human.body.health.min(5));
        }
        ChanceTemplate::Windfall => {
            let missing = human.body.max_health.saturating_sub(human.body.health);
            effect.health_delta = i32::from(missing.min(5));
        }
        ChanceTemplate::Discovery => {
            let skill = skill_modifier?;
            let gain = learning_gain(
                &human.skills,
                &human.consciousness,
                &human.phenotype,
                skill,
                100,
            )
            .ok()?;
            if gain > 0 {
                effect.skill_xp.insert(skill, gain);
            }
        }
    }
    (!effect.is_empty()).then_some(effect)
}

fn resolve_chance(spec: ChanceSpec, view: &WorldView<'_>, key: ResolutionKey<'_>) -> EventOutcome {
    let subjects = existing_subjects(view);
    let Some(draw_subject) = subjects.first().copied() else {
        return EventOutcome::NoOp;
    };
    let level = spec
        .skill_modifier
        .and_then(|skill| {
            view.humans
                .get(&draw_subject)
                .map(|human| human.skills.level_of(skill))
        })
        .unwrap_or(0);
    let shifted = i64::from(spec.base_prob.0.min(1000)).saturating_add(
        i64::from(spec.modifier_strength.0.min(1000)).saturating_mul(i64::from(level)),
    );
    let probability = Permille::clamp1000(shifted);
    if !key.rng.gate(
        RngDomain::Event,
        key.tick,
        draw_subject,
        u64::from(key.seq.0),
        probability,
    ) {
        return EventOutcome::NoOp;
    }
    let effects = subjects
        .into_iter()
        .filter_map(|id| {
            view.humans
                .get(&id)
                .and_then(|human| chance_effect(spec.template, spec.skill_modifier, human))
                .map(|effect| (id, effect))
        })
        .collect();
    outcome_from_effects(effects)
}

fn resolve_deterministic(kind: DeterministicKind, view: &WorldView<'_>) -> EventOutcome {
    let effects = existing_subjects(view)
        .into_iter()
        .filter_map(|id| {
            let human = view.humans.get(&id)?;
            let effect = match kind {
                DeterministicKind::HealthTick if human.body.health > 0 => EffectSummary {
                    health_delta: -1,
                    ..EffectSummary::default()
                },
                DeterministicKind::Maturation => EffectSummary {
                    age_ticks_delta: 1,
                    ..EffectSummary::default()
                },
                DeterministicKind::HealthTick => EffectSummary::default(),
            };
            (!effect.is_empty()).then_some((id, effect))
        })
        .collect();
    outcome_from_effects(effects)
}

#[must_use]
pub fn resolve(
    payload: &EventPayload,
    view: &WorldView<'_>,
    rng: &Rng,
    tick: Tick,
    seq: Seq,
) -> EventOutcome {
    match payload {
        EventPayload::Chance {
            template,
            base_prob,
            skill_modifier,
            modifier_strength,
        } => resolve_chance(
            ChanceSpec {
                template: *template,
                base_prob: *base_prob,
                skill_modifier: *skill_modifier,
                modifier_strength: *modifier_strength,
            },
            view,
            ResolutionKey { rng, tick, seq },
        ),
        EventPayload::Deterministic(kind) => resolve_deterministic(*kind, view),
        EventPayload::Gosh(kind) => {
            resolve_gosh(kind, view, tick, seq).unwrap_or(EventOutcome::NoOp)
        }
    }
}

#[cfg(test)]
mod tests {
    //! Event resolution is replayable, canonically ordered, awareness-gated, and deterministic when required.

    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        Boon, GoshKind, HumanId, Permille, Rng, RngDomain, Seq, SkillId, SkillState, Tick,
        WorldView, fixture_human,
    };

    fn humans() -> BTreeMap<HumanId, crate::HumanState> {
        BTreeMap::from([
            (HumanId(1), fixture_human(HumanId(1))),
            (HumanId(2), fixture_human(HumanId(2))),
        ])
    }

    #[test]
    fn a_chance_event_replays_identically_after_unrelated_draws() {
        let humans = humans();
        let subjects = [HumanId(2), HumanId(1), HumanId(2)];
        let view = WorldView {
            humans: &humans,
            subjects: &subjects,
            next_human_id: HumanId(3),
        };
        let payload = EventPayload::Chance {
            template: ChanceTemplate::Accident,
            base_prob: Permille(500),
            skill_modifier: None,
            modifier_strength: Permille::ZERO,
        };
        let rng = Rng::new(7);
        let first = resolve(&payload, &view, &rng, Tick(9), Seq(4));
        let _unrelated = rng.draw_u64(RngDomain::Mutation, Tick(100), HumanId(99), 88);
        assert_eq!(first, resolve(&payload, &view, &rng, Tick(9), Seq(4)));
    }

    #[test]
    fn a_skill_modifier_shifts_and_clamps_chance_probability() {
        let mut humans = humans();
        humans
            .get_mut(&HumanId(1))
            .expect("fixture exists")
            .skills
            .levels
            .insert(
                SkillId::Planning,
                SkillState {
                    xp: 100,
                    learned: false,
                },
            );
        let subjects = [HumanId(1)];
        let view = WorldView {
            humans: &humans,
            subjects: &subjects,
            next_human_id: HumanId(3),
        };
        let impossible = EventPayload::Chance {
            template: ChanceTemplate::Accident,
            base_prob: Permille::ZERO,
            skill_modifier: Some(SkillId::Medicine),
            modifier_strength: Permille::ONE,
        };
        assert_eq!(
            resolve(&impossible, &view, &Rng::new(1), Tick(1), Seq(1)),
            EventOutcome::NoOp
        );

        let guaranteed = EventPayload::Chance {
            template: ChanceTemplate::Accident,
            base_prob: Permille(900),
            skill_modifier: Some(SkillId::Planning),
            modifier_strength: Permille(100),
        };
        assert!(matches!(
            resolve(&guaranteed, &view, &Rng::new(1), Tick(1), Seq(1)),
            EventOutcome::Occurred(_)
        ));
    }

    #[test]
    fn discovery_emits_no_experience_for_an_awareness_locked_skill() {
        let mut humans = humans();
        humans
            .get_mut(&HumanId(1))
            .expect("fixture exists")
            .consciousness
            .awareness = 9;
        let subjects = [HumanId(1)];
        let view = WorldView {
            humans: &humans,
            subjects: &subjects,
            next_human_id: HumanId(3),
        };
        let payload = EventPayload::Chance {
            template: ChanceTemplate::Discovery,
            base_prob: Permille::ONE,
            skill_modifier: Some(SkillId::Motor),
            modifier_strength: Permille::ZERO,
        };
        assert_eq!(
            resolve(&payload, &view, &Rng::new(1), Tick(1), Seq(1)),
            EventOutcome::NoOp
        );
    }

    #[test]
    fn discovery_experience_is_attenuated_before_recall() {
        let humans = humans();
        let subjects = [HumanId(1)];
        let view = WorldView {
            humans: &humans,
            subjects: &subjects,
            next_human_id: HumanId(3),
        };
        let payload = EventPayload::Chance {
            template: ChanceTemplate::Discovery,
            base_prob: Permille::ONE,
            skill_modifier: Some(SkillId::Motor),
            modifier_strength: Permille::ZERO,
        };
        let EventOutcome::Occurred(effects) =
            resolve(&payload, &view, &Rng::new(1), Tick(1), Seq(1))
        else {
            panic!("guaranteed discovery should occur");
        };
        assert_eq!(effects[&HumanId(1)].skill_xp[&SkillId::Motor], 25);
    }

    #[test]
    fn deterministic_events_are_pure_functions_of_the_view() {
        let humans = humans();
        let subjects = [HumanId(1), HumanId(2)];
        let view = WorldView {
            humans: &humans,
            subjects: &subjects,
            next_human_id: HumanId(3),
        };
        let health = EventPayload::Deterministic(DeterministicKind::HealthTick);
        let maturation = EventPayload::Deterministic(DeterministicKind::Maturation);
        assert_eq!(
            resolve(&health, &view, &Rng::new(1), Tick(1), Seq(1)),
            resolve(&health, &view, &Rng::new(u64::MAX), Tick(1), Seq(1))
        );
        let EventOutcome::Occurred(effects) =
            resolve(&maturation, &view, &Rng::new(1), Tick(1), Seq(1))
        else {
            panic!("maturation should affect existing humans");
        };
        assert!(effects.values().all(|effect| effect.age_ticks_delta == 1));
    }

    #[test]
    fn a_gosh_resolves_identically_under_wildly_different_seeds() {
        let humans = humans();
        let subjects = [HumanId(1)];
        let view = WorldView {
            humans: &humans,
            subjects: &subjects,
            next_human_id: HumanId(3),
        };
        let payload = EventPayload::Gosh(GoshKind::Bless {
            subject: HumanId(1),
            boon: Boon::Fertility(20),
        });
        assert_eq!(
            resolve(&payload, &view, &Rng::new(0), Tick(9), Seq(2)),
            resolve(&payload, &view, &Rng::new(u64::MAX), Tick(9), Seq(2))
        );
    }
}
