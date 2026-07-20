//! These tests prove that social standing emerges from living conferral ledgers, active
//! networks remain bounded, and oversized groups respond reproducibly with structure.

use std::collections::{BTreeMap, BTreeSet};

use anana_core::{HumanId, Residence, ResidenceId, SocialBonds, prestige_of};
use anana_sim::{Config, HashHistory, build_headless_app, snapshot, step};

#[test]
fn demonstrated_competence_receives_living_revocable_deference() {
    let mut app = build_headless_app(42, Config::default());
    for _ in 0..50 {
        step(&mut app);
    }
    let world = snapshot(&mut app);
    let ledgers = world
        .humans
        .iter()
        .map(|(id, human)| (*id, human.social_bonds.clone()))
        .collect::<BTreeMap<_, _>>();
    let living = world.humans.keys().copied().collect::<BTreeSet<_>>();
    let highest = living
        .iter()
        .map(|id| prestige_of(*id, &ledgers, &living))
        .max()
        .unwrap_or(0);
    assert!(highest > 0);

    let conferrer = ledgers
        .iter()
        .find_map(|(id, social)| {
            social
                .deference
                .values()
                .any(|value| value.0 > 0)
                .then_some(*id)
        })
        .expect("at least one living human confers deference");
    let without_conferrer = living
        .iter()
        .copied()
        .filter(|id| *id != conferrer)
        .collect::<BTreeSet<_>>();
    assert!(living.iter().any(|subject| {
        prestige_of(*subject, &ledgers, &without_conferrer)
            < prestige_of(*subject, &ledgers, &living)
    }));
}

#[test]
fn no_human_exceeds_the_social_capacity_during_a_long_run() {
    let config = Config::default();
    let capacity = config.social_capacity;
    let mut app = build_headless_app(7, config);
    for _ in 0..500 {
        step(&mut app);
        let world = snapshot(&mut app);
        assert!(
            world
                .humans
                .values()
                .all(|human| human.social_bonds.bonds.len() <= capacity)
        );
    }
}

fn one_oversized_residence(seed: u64) -> anana_sim::App {
    let mut app = build_headless_app(
        seed,
        Config {
            initial_population: 160,
            carrying_capacity: 300,
            ..Config::default()
        },
    );
    let world = app.world_mut();
    let mut query = world.query::<(&HumanId, &mut Residence, &mut SocialBonds)>();
    for (_, mut residence, mut social) in query.iter_mut(world) {
        residence.id = ResidenceId(1);
        social.bonds.clear();
    }
    app
}

#[test]
fn an_oversized_group_develops_structure_and_replays_the_same_response() {
    let mut first = one_oversized_residence(99);
    let mut second = one_oversized_residence(99);
    for _ in 0..10 {
        step(&mut first);
        step(&mut second);
    }
    let first_world = snapshot(&mut first);
    let group_count = first_world
        .humans
        .values()
        .map(|human| human.residence.id)
        .collect::<BTreeSet<_>>()
        .len();
    let structured = first_world
        .coalitions
        .values()
        .any(|coalition| coalition.stratified);
    assert!(group_count > 1 || structured);
    assert_eq!(
        first.world().resource::<HashHistory>().0,
        second.world().resource::<HashHistory>().0
    );
}
