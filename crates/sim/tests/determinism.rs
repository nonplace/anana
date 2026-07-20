//! These scenarios prove that complete simulation trajectories are reproducible from their seeds.

use anana_core::{Seq, world_hash};
use anana_sim::{
    Config, EventLog, HashHistory, SimulationFaults, SimulationStats, build_headless_app, snapshot,
    step,
};

fn run(seed: u64, config: Config, ticks: u64) -> anana_sim::App {
    let mut app = build_headless_app(seed, config);
    for _ in 0..ticks {
        step(&mut app);
    }
    app
}

#[test]
fn the_same_seed_reproduces_every_tick_of_the_same_world() {
    let first = run(42, Config::default(), 500);
    let second = run(42, Config::default(), 500);
    let first_history = &first.world().resource::<HashHistory>().0;
    let second_history = &second.world().resource::<HashHistory>().0;
    assert_eq!(first_history.len(), 500);
    assert_eq!(first_history, second_history);
}

#[test]
fn different_seeds_produce_different_world_trajectories() {
    let first = run(41, Config::default(), 200);
    let second = run(42, Config::default(), 200);
    assert_ne!(
        first.world().resource::<HashHistory>().0,
        second.world().resource::<HashHistory>().0
    );
}

#[test]
fn executor_thread_requests_cannot_change_random_draws_or_world_order() {
    let single = run(
        42,
        Config {
            requested_threads: 1,
            ..Config::default()
        },
        200,
    );
    let multiple = run(
        42,
        Config {
            requested_threads: 4,
            ..Config::default()
        },
        200,
    );
    assert_eq!(
        single.world().resource::<HashHistory>().0,
        multiple.world().resource::<HashHistory>().0
    );
}

#[test]
fn the_golden_scenario_keeps_living_humans_an_ordered_log_and_no_faults() {
    let app = run(42, Config::default(), 50);
    let stats = app.world().resource::<SimulationStats>();
    let records = app.world().resource::<EventLog>().records();
    assert!(stats.living > 0);
    assert!(!records.is_empty());
    assert!(records.windows(2).all(|pair| pair[0].seq < pair[1].seq));
    assert_eq!(records.first().map(|record| record.seq), Some(Seq(0)));
    assert!(app.world().resource::<SimulationFaults>().0.is_empty());
}

#[test]
fn the_public_snapshot_matches_the_hash_recorded_for_the_latest_tick() {
    let mut app = run(42, Config::default(), 3);
    let current = snapshot(&mut app);
    assert_eq!(
        app.world().resource::<HashHistory>().0.last(),
        Some(&world_hash(&current))
    );
}
