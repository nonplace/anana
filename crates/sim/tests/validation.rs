//! These validations inspect only public canonical snapshots and the event log. They deliberately
//! share no thresholds or constants with the mechanisms they audit: otherwise a passing check
//! would be a tautology rather than evidence that social mechanisms produce population patterns.

use std::collections::{BTreeMap, BTreeSet};

use anana_core::{
    Body, Bond, HumanId, Permille, Residence, SkillId, SocialBonds, Tick, WorldSnapshot,
};
use anana_sim::{Config, ValidationSample, build_headless_app, snapshot, step, validation_sample};
use bevy::prelude::Entity;

fn run_samples(seed: u64, ticks: u64, interval: u64) -> Vec<ValidationSample> {
    let mut app = build_headless_app(
        seed,
        Config {
            innovation_skill: Some(SkillId::ToolUse),
            ..Config::default()
        },
    );
    let mut samples = Vec::new();
    for tick in 1..=ticks {
        step(&mut app);
        if tick.is_multiple_of(interval) {
            let current = snapshot(&mut app);
            samples.push(validation_sample(&current, SkillId::ToolUse));
        }
    }
    samples
}

fn increments(samples: &[ValidationSample]) -> Vec<u64> {
    samples
        .windows(2)
        .map(|pair| pair[1].adopters.saturating_sub(pair[0].adopters))
        .collect()
}

fn smoothed(values: &[u64]) -> Vec<u64> {
    values
        .windows(3)
        .map(|window| window.iter().copied().sum())
        .collect()
}

fn founder_ancestors(snapshot: &WorldSnapshot, subject: HumanId) -> BTreeSet<HumanId> {
    let lineages = snapshot
        .humans
        .iter()
        .map(|(id, human)| (*id, human.lineage.clone()))
        .chain(
            snapshot
                .dead
                .iter()
                .map(|(id, human)| (*id, human.lineage.clone())),
        )
        .collect::<BTreeMap<_, _>>();
    let mut founders = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut pending = vec![subject];
    while let Some(id) = pending.pop() {
        if !visited.insert(id) {
            continue;
        }
        let Some(lineage) = lineages.get(&id) else {
            continue;
        };
        if lineage.generation == 0 {
            founders.insert(id);
        } else {
            pending.extend(lineage.mother);
            pending.extend(lineage.father);
        }
    }
    founders
}

fn cultural_loss_world(teaching: bool) -> (anana_sim::App, HumanId) {
    let mut app = build_headless_app(
        91,
        Config {
            initial_population: 20,
            carrying_capacity: 60,
            mortality_interval: u64::MAX,
            innovation_skill: Some(SkillId::ToolUse),
            innovation_external_rate: Permille::ZERO,
            ..Config::default()
        },
    );
    let initial = snapshot(&mut app);
    let holder = initial
        .humans
        .values()
        .find(|human| human.skills.level_of(SkillId::ToolUse) >= 1)
        .expect("the experimental innovation was seeded");
    let learner = initial
        .humans
        .values()
        .filter(|human| {
            human.id != holder.id
                && human.skills.recall_learned()
                && human.body.life_stage == anana_core::LifeStage::Adult
        })
        .min_by_key(|human| human.body.age_ticks)
        .expect("the founder cohort contains another remembering adult");
    let holder_id = holder.id;
    let learner_id = learner.id;
    let holder_lifespan = holder.phenotype.lifespan_ticks;
    let entities = {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &HumanId)>();
        query
            .iter(world)
            .map(|(entity, id)| (*id, entity))
            .collect::<BTreeMap<_, _>>()
    };
    let holder_entity = entities[&holder_id];
    if let Some(mut body) = app.world_mut().get_mut::<Body>(holder_entity) {
        body.age_ticks = holder_lifespan.saturating_mul(2) / 5;
        body.health = body.max_health;
        body.fertility = 0;
    }
    if let Some(mut residence) = app.world_mut().get_mut::<Residence>(holder_entity) {
        residence.id = anana_core::ResidenceId(900);
    }
    if teaching {
        let learner_entity = entities[&learner_id];
        if let Some(mut residence) = app.world_mut().get_mut::<Residence>(learner_entity) {
            residence.id = anana_core::ResidenceId(900);
        }
        for (entity, other) in [(holder_entity, learner_id), (learner_entity, holder_id)] {
            if let Some(mut social) = app.world_mut().get_mut::<SocialBonds>(entity) {
                social.bonds.insert(
                    other,
                    Bond {
                        strength: Permille(800),
                        last_interaction: Tick(0),
                        last_decay_tick: Tick(0),
                        positive_interactions: 20,
                        defections: 0,
                    },
                );
            }
        }
    }
    (app, holder_id)
}

fn end_life(app: &mut anana_sim::App, subject: HumanId) {
    let entity = {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &HumanId)>();
        query
            .iter(world)
            .find_map(|(entity, id)| (*id == subject).then_some(entity))
            .expect("the innovation holder is still alive")
    };
    if let Some(mut body) = app.world_mut().get_mut::<Body>(entity) {
        body.health = 0;
    }
}

#[test]
fn a_seeded_innovation_begins_with_exactly_one_adopter() {
    let samples = run_samples(42, 1, 1);
    assert_eq!(samples[0].adopters, 1);
}

#[test]
#[ignore = "long diffusion audit; run with cargo test -p anana-sim --test validation skill_diffusion -- --ignored --nocapture --test-threads=1"]
fn skill_diffusion_is_s_shaped_and_peaks_away_from_both_ends() {
    let samples = run_samples(42, 5_000, 100);
    assert!(
        samples
            .windows(2)
            .all(|pair| pair[0].adopters <= pair[1].adopters)
    );
    let gains = smoothed(&increments(&samples));
    let (peak_index, peak_gain) = gains
        .iter()
        .copied()
        .enumerate()
        .max_by_key(|(index, gain)| (*gain, std::cmp::Reverse(*index)))
        .expect("the diffusion run has intervals");
    let eventual = samples.last().expect("the run has a final sample").adopters;
    let adopted_at_peak = samples[peak_index.saturating_add(2)].adopters;
    println!(
        "diffusion series={:?}",
        samples
            .iter()
            .map(|sample| (sample.tick.0, sample.adopters))
            .collect::<Vec<_>>()
    );
    println!(
        "diffusion eventual={eventual} peak_tick={} adopted_at_peak={adopted_at_peak} peak_fraction_permille={}",
        samples[peak_index.saturating_add(2)].tick.0,
        adopted_at_peak.saturating_mul(1000) / eventual.max(1)
    );
    assert!(peak_gain > 0);
    assert!(eventual > 20);
    assert!(adopted_at_peak.saturating_mul(4) >= eventual);
    assert!(adopted_at_peak.saturating_mul(3) <= eventual.saturating_mul(2));
    assert!(
        samples.last().expect("final sample").adopters
            <= samples.last().expect("final sample").total_lived
    );
}

#[test]
#[ignore = "three-seed population audit; run with cargo test -p anana-sim --test validation population_curve -- --ignored --nocapture --test-threads=1"]
fn population_curve_settles_without_exponential_growth_or_catastrophic_loss() {
    for seed in [41, 42, 43] {
        let samples = run_samples(seed, 5_000, 100);
        let settled = &samples[19..];
        let minimum = settled
            .iter()
            .map(|point| point.living)
            .min()
            .expect("samples");
        let maximum = settled
            .iter()
            .map(|point| point.living)
            .max()
            .expect("samples");
        assert!(minimum >= 100);
        assert!(maximum <= 330);
        assert!(
            settled
                .windows(2)
                .all(|pair| pair[0].living.saturating_sub(pair[1].living) <= pair[0].living / 4)
        );
        println!(
            "population seed={seed} minimum={minimum} maximum={maximum} final={}",
            settled.last().expect("samples").living
        );
    }
}

#[test]
#[ignore = "kin-structure audit; run with cargo test -p anana-sim --test validation kin_structure -- --ignored --nocapture --test-threads=1"]
fn residence_groups_develop_kin_structure_across_deep_lineages() {
    let mut app = build_headless_app(42, Config::default());
    for _ in 0..5_000 {
        step(&mut app);
    }
    let current = snapshot(&mut app);
    let mut within_shared = 0_u64;
    let mut within_total = 0_u64;
    let mut between_shared = 0_u64;
    let mut between_total = 0_u64;
    let people = current.humans.values().collect::<Vec<_>>();
    for (index, first) in people.iter().enumerate() {
        for second in people.iter().skip(index.saturating_add(1)) {
            let related = !founder_ancestors(&current, first.id)
                .is_disjoint(&founder_ancestors(&current, second.id));
            if first.residence == second.residence {
                within_total = within_total.saturating_add(1);
                within_shared = within_shared.saturating_add(u64::from(related));
            } else {
                between_total = between_total.saturating_add(1);
                between_shared = between_shared.saturating_add(u64::from(related));
            }
        }
    }
    assert!(
        current
            .humans
            .values()
            .any(|human| human.lineage.generation >= 5)
    );
    assert!(
        within_shared.saturating_mul(between_total) > between_shared.saturating_mul(within_total)
    );
    println!("kin within={within_shared}/{within_total} between={between_shared}/{between_total}");
}

#[test]
fn social_networks_bind_below_capacity_and_concentrate_effort_inward() {
    let samples = run_samples(73, 500, 100);
    let final_sample = samples.last().expect("the run has a final sample");
    assert!(final_sample.maximum_relationships <= 150);
    assert!(final_sample.people_at_capacity < final_sample.living);
    assert!(final_sample.inner_layer_relationships < final_sample.active_relationships);
    assert!(final_sample.inner_layer_effort > final_sample.outer_layer_effort);
}

#[test]
fn knowledge_dies_with_its_only_holder_but_survives_when_taught() {
    let (mut isolated, isolated_holder) = cultural_loss_world(false);
    let (mut taught, taught_holder) = cultural_loss_world(true);
    for _ in 0..600 {
        step(&mut isolated);
        step(&mut taught);
    }
    let before_death = snapshot(&mut taught);
    println!(
        "before death={:?}",
        before_death
            .humans
            .values()
            .filter_map(
                |human| human.skills.levels.get(&SkillId::ToolUse).map(|state| (
                    human.id,
                    state.xp,
                    state.learned
                ))
            )
            .collect::<Vec<_>>()
    );
    end_life(&mut isolated, isolated_holder);
    end_life(&mut taught, taught_holder);
    for _ in 0..200 {
        step(&mut isolated);
        step(&mut taught);
    }
    let isolated = validation_sample(&snapshot(&mut isolated), SkillId::ToolUse);
    let taught_snapshot = snapshot(&mut taught);
    let taught = validation_sample(&taught_snapshot, SkillId::ToolUse);
    assert_eq!(isolated.living_adopters, 0);
    assert!(taught.living_adopters > 0);
}
