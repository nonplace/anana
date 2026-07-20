//! Population-scale tests prove that density-dependent reproduction and age-structured mortality
//! sustain many human generations without either exponential growth or collapse.

use std::collections::{BTreeMap, BTreeSet};

use anana_core::{DeadHuman, HumanId, HumanState, Lineage, WorldSnapshot};
use anana_sim::{
    Config, HashHistory, PopulationHistory, PopulationPoint, SimulationFaults, build_headless_app,
    snapshot, step,
};

fn run(seed: u64, ticks: u64) -> (anana_sim::App, Vec<PopulationPoint>) {
    let mut app = build_headless_app(seed, Config::default());
    for _ in 0..ticks {
        step(&mut app);
    }
    let history = app.world().resource::<PopulationHistory>().0.clone();
    (app, history)
}

fn point_at(history: &[PopulationPoint], tick: u64) -> PopulationPoint {
    history
        .iter()
        .find(|point| point.tick.0 == tick)
        .copied()
        .expect("the requested tick was recorded")
}

#[test]
#[ignore = "three-seed burn-in validation; run with cargo test -p anana-sim --test population three_seed -- --ignored --test-threads=1"]
fn three_seed_populations_settle_without_collapse_explosion_or_catastrophic_ticks() {
    for seed in [41, 42, 43] {
        let (app, history) = run(seed, 2_500);
        let settled = history
            .iter()
            .filter(|point| point.tick.0 >= 1_000)
            .collect::<Vec<_>>();
        assert!(settled.iter().all(|point| point.living >= 120));
        assert!(
            settled
                .iter()
                .all(|point| point.living <= u64::from(Config::default().carrying_capacity))
        );
        assert!(settled.windows(2).all(|pair| {
            pair[0].living.saturating_sub(pair[1].living)
                <= pair[0].living.saturating_div(10).max(1)
        }));
        assert!(app.world().resource::<SimulationFaults>().0.is_empty());
    }
}

fn lineage_map(snapshot: &WorldSnapshot) -> BTreeMap<HumanId, Lineage> {
    snapshot
        .humans
        .iter()
        .map(|(id, human)| (*id, human.lineage.clone()))
        .chain(
            snapshot
                .dead
                .iter()
                .map(|(id, human)| (*id, human.lineage.clone())),
        )
        .collect()
}

fn has_three_dead_ancestors(
    human: &HumanState,
    dead: &BTreeMap<HumanId, DeadHuman>,
    lineages: &BTreeMap<HumanId, Lineage>,
) -> bool {
    let mut frontier = human
        .lineage
        .mother
        .into_iter()
        .chain(human.lineage.father)
        .collect::<BTreeSet<_>>();
    for _ in 0..3 {
        let Some(ancestor) = frontier.iter().find(|id| dead.contains_key(id)).copied() else {
            return false;
        };
        frontier.clear();
        let Some(lineage) = lineages.get(&ancestor) else {
            return false;
        };
        frontier.extend(lineage.mother);
        frontier.extend(lineage.father);
    }
    true
}

#[test]
#[ignore = "full lineage validation; run with cargo test -p anana-sim --test population a_default_length -- --ignored --test-threads=1"]
fn a_default_length_history_reaches_five_generations_and_remembers_dead_ancestors() {
    let (mut app, _) = run(42, 5_000);
    let current = snapshot(&mut app);
    let lineages = lineage_map(&current);
    assert!(
        current
            .humans
            .values()
            .map(|human| human.lineage.generation)
            .max()
            .is_some_and(|generation| generation >= 5)
    );
    assert!(
        current
            .humans
            .values()
            .any(|human| has_three_dead_ancestors(human, &current.dead, &lineages))
    );
}

#[test]
#[ignore = "density settling validation; run with cargo test -p anana-sim --test population growth_slows -- --ignored --test-threads=1"]
fn growth_slows_after_density_brings_the_population_near_capacity() {
    let (_, history) = run(42, 2_500);
    let initial = u64::from(Config::default().initial_population);
    let early_growth = point_at(&history, 1_000).living.saturating_sub(initial);
    let late_start = point_at(&history, 1_500).living;
    let late_growth = point_at(&history, 2_500).living.saturating_sub(late_start);
    assert!(early_growth > late_growth.saturating_mul(2));
}

#[test]
fn the_extended_social_run_is_deterministic_tick_for_tick() {
    let (first, _) = run(73, 500);
    let (second, _) = run(73, 500);
    assert_eq!(
        first.world().resource::<HashHistory>().0,
        second.world().resource::<HashHistory>().0
    );
}

#[test]
#[ignore = "full default-length determinism check; run with cargo test -p anana-sim --test population -- --ignored"]
fn the_full_default_length_social_run_is_deterministic_tick_for_tick() {
    let (first, _) = run(73, 5_000);
    let (second, _) = run(73, 5_000);
    assert_eq!(
        first.world().resource::<HashHistory>().0,
        second.world().resource::<HashHistory>().0
    );
}
