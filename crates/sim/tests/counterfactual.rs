//! Counterfactual projections preserve the branch world, identify pre-branch people directly,
//! and compare every post-branch life only through branch-scoped identities and aggregates.

use anana_core::{Bane, GoshKind, GoshTarget, HumanId};
use anana_sim::{
    BranchSide, Config, CounterfactualDifferences, CounterfactualRequest, HashHistory,
    build_headless_app, project_counterfactual, run_counterfactual, snapshot, step,
};

fn request(decree: Option<GoshKind>) -> CounterfactualRequest {
    CounterfactualRequest {
        seed: 42,
        config: Config {
            initial_population: 20,
            carrying_capacity: 80,
            ..Config::default()
        },
        branch_at: 40,
        horizon: 120,
        decree,
    }
}

#[test]
fn the_same_counterfactual_is_byte_identical_every_time() {
    let request = request(Some(GoshKind::Afflict {
        target: GoshTarget::One(HumanId(12)),
        bane: Bane::Harm(u16::MAX),
    }));
    let first = run_counterfactual(request.clone()).expect("the first projection succeeds");
    let second = run_counterfactual(request).expect("the repeated projection succeeds");
    assert_eq!(
        serde_json::to_vec(&first).expect("the first comparison serializes"),
        serde_json::to_vec(&second).expect("the repeated comparison serializes")
    );
}

#[test]
fn the_untouched_continuation_matches_a_world_that_never_branched() {
    let request = request(Some(GoshKind::Afflict {
        target: GoshTarget::One(HumanId(12)),
        bane: Bane::Harm(u16::MAX),
    }));
    let comparison = run_counterfactual(request.clone()).expect("the projection succeeds");
    let mut straight = build_headless_app(request.seed, request.config);
    for _ in 0..request.horizon {
        step(&mut straight);
    }
    assert_eq!(
        comparison.untouched.world_hash,
        straight
            .world()
            .resource::<HashHistory>()
            .0
            .last()
            .copied()
            .expect("the straight run records its horizon hash")
    );
}

#[test]
fn an_empty_decree_produces_no_difference() {
    let comparison = run_counterfactual(request(None)).expect("the empty projection succeeds");
    assert_eq!(comparison.differences, CounterfactualDifferences::default());
    assert_eq!(
        comparison.untouched.world_hash,
        comparison.decreed.world_hash
    );
}

#[test]
fn projecting_two_continuations_leaves_the_branch_world_untouched() {
    let mut branch = build_headless_app(42, request(None).config);
    for _ in 0..40 {
        step(&mut branch);
    }
    let before = snapshot(&mut branch);
    let _comparison = project_counterfactual(
        &mut branch,
        120,
        Some(GoshKind::Afflict {
            target: GoshTarget::One(HumanId(12)),
            bane: Bane::Harm(u16::MAX),
        }),
    )
    .expect("the branch projects");
    assert_eq!(snapshot(&mut branch), before);
}

#[test]
fn every_post_branch_example_has_an_explicit_branch_scoped_identity() {
    let comparison = run_counterfactual(request(Some(GoshKind::Afflict {
        target: GoshTarget::One(HumanId(12)),
        bane: Bane::Harm(u16::MAX),
    })))
    .expect("the projection succeeds");
    assert!(
        comparison
            .differences
            .post_branch
            .untouched_examples
            .iter()
            .all(|identity| identity.branch == BranchSide::Untouched)
    );
    assert!(
        comparison
            .differences
            .post_branch
            .decreed_examples
            .iter()
            .all(|identity| identity.branch == BranchSide::Decreed)
    );
}
