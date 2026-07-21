//! Position validation reads only public canonical snapshots. The long measurement reports whether
//! clustering, relationship stability, and socially costly backfire actually appear without
//! changing constants to make the hoped-for signs pass.

use anana_core::{AttachedPosition, Permille, PositionSignal, SkillId, receive_position};
use anana_sim::{Config, build_headless_app, snapshot, step};

fn run(seed: u64, ticks: u64, coalition_cost_enabled: bool) -> anana_core::WorldSnapshot {
    let mut app = build_headless_app(
        seed,
        Config {
            coalition_cost_enabled,
            ..Config::default()
        },
    );
    for _ in 0..ticks {
        step(&mut app);
    }
    snapshot(&mut app)
}

fn spread(snapshot: &anana_core::WorldSnapshot) -> f64 {
    let mut total = 0.0;
    let mut measured = 0_u32;
    for slot in 0..anana_core::POSITION_SLOT_COUNT {
        let values = snapshot
            .humans
            .values()
            .filter_map(|human| {
                let position = human.positions.slots.get(slot)?;
                (position.conviction != Permille::ZERO).then_some(f64::from(position.value))
            })
            .collect::<Vec<_>>();
        if values.len() < 2 {
            continue;
        }
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / values.len() as f64;
        total += variance.sqrt();
        measured = measured.saturating_add(1);
    }
    if measured == 0 {
        0.0
    } else {
        total / f64::from(measured)
    }
}

fn camps(snapshot: &anana_core::WorldSnapshot) -> (usize, usize, usize) {
    snapshot
        .humans
        .values()
        .flat_map(|human| human.positions.slots.iter())
        .filter(|position| position.conviction != Permille::ZERO)
        .fold((0_usize, 0_usize, 0_usize), |counts, position| {
            if position.value <= -400 {
                (counts.0.saturating_add(1), counts.1, counts.2)
            } else if position.value >= 400 {
                (counts.0, counts.1, counts.2.saturating_add(1))
            } else {
                (counts.0, counts.1.saturating_add(1), counts.2)
            }
        })
}

fn movement_relationship_samples(snapshot: &anana_core::WorldSnapshot) -> Vec<(f64, f64)> {
    snapshot
        .humans
        .values()
        .map(|human| &human.positions)
        .chain(snapshot.dead.values().map(|human| &human.positions))
        .filter(|positions| positions.social_exposure_ticks > 0)
        .map(|positions| {
            let relationships = positions.mean_relationships_permille() as f64 / 1_000.0;
            let movement = positions
                .slots
                .iter()
                .map(|position| f64::from(position.lifetime_movement))
                .sum::<f64>();
            (relationships, movement)
        })
        .collect()
}

fn regression_slope(samples: &[(f64, f64)]) -> Option<f64> {
    if samples.len() < 2 {
        return None;
    }
    let mean_x = samples.iter().map(|(x, _)| x).sum::<f64>() / samples.len() as f64;
    let mean_y = samples.iter().map(|(_, y)| y).sum::<f64>() / samples.len() as f64;
    let numerator = samples
        .iter()
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum::<f64>();
    let denominator = samples
        .iter()
        .map(|(x, _)| (x - mean_x).powi(2))
        .sum::<f64>();
    (denominator > 0.0).then_some(numerator / denominator)
}

fn burst_measurement(snapshot: &anana_core::WorldSnapshot) -> (usize, usize, f64, f64) {
    let mut away = 0_usize;
    let mut toward = 0_usize;
    let mut away_attachment = 0_u64;
    let mut toward_attachment = 0_u64;
    for human in snapshot.humans.values() {
        if !human.skills.recall_learned() {
            continue;
        }
        let Some(position) = human.positions.slots.first() else {
            continue;
        };
        if position.value >= 0 || position.conviction == Permille::ZERO {
            continue;
        }
        let attached = human
            .social_bonds
            .bonds
            .iter()
            .filter_map(|(id, bond)| {
                let neighbour = snapshot.humans.get(id)?;
                let neighbour_position = neighbour.positions.slots.first()?;
                (neighbour_position.conviction != Permille::ZERO).then_some(AttachedPosition {
                    value: neighbour_position.value,
                    attachment: bond.strength,
                })
            })
            .collect::<Vec<_>>();
        let attachment = attached
            .iter()
            .map(|neighbour| u64::from(neighbour.attachment.0))
            .sum::<u64>();
        let mut positions = human.positions.clone();
        let change = receive_position(
            &mut positions,
            PositionSignal {
                slot: 0,
                value: 1_000,
                retention: Permille::ONE,
            },
            &attached,
            true,
        );
        if change.moved_away {
            away = away.saturating_add(1);
            away_attachment = away_attachment.saturating_add(attachment);
        } else if change.moved_toward {
            toward = toward.saturating_add(1);
            toward_attachment = toward_attachment.saturating_add(attachment);
        }
    }
    (
        away,
        toward,
        if away == 0 {
            0.0
        } else {
            away_attachment as f64 / away as f64
        },
        if toward == 0 {
            0.0
        } else {
            toward_attachment as f64 / toward as f64
        },
    )
}

#[test]
fn remembering_adults_acquire_anonymous_positions_while_children_begin_without_them() {
    let world = run(42, 200, true);
    assert!(world.humans.values().any(|human| {
        human.skills.level_of(SkillId::Recall) > 0
            && human
                .positions
                .slots
                .iter()
                .any(|position| position.conviction != Permille::ZERO)
    }));
    assert!(
        world
            .humans
            .values()
            .filter(|human| human.lineage.generation > 0)
            .all(|human| {
                human.skills.recall_learned()
                    || human
                        .positions
                        .slots
                        .iter()
                        .all(|position| position.conviction == Permille::ZERO)
            })
    );
}

#[test]
#[ignore = "long-run position measurement; run with cargo test -p anana-sim --test positions long_run -- --ignored --nocapture --test-threads=1"]
fn long_run_position_patterns_are_reported_without_forcing_the_expected_direction() {
    let normal = run(42, 2_000, true);
    let no_coalition = run(42, 2_000, false);
    let normal_spread = spread(&normal);
    let control_spread = spread(&no_coalition);
    let normal_camps = camps(&normal);
    let control_camps = camps(&no_coalition);
    let samples = movement_relationship_samples(&normal);
    let slope = regression_slope(&samples);
    let (away, toward, away_attachment, toward_attachment) = burst_measurement(&normal);
    println!(
        "position_validation normal_spread={normal_spread:.3} normal_camps={normal_camps:?} zero_cost_spread={control_spread:.3} zero_cost_camps={control_camps:?} movement_relationship_slope={slope:?} samples={} burst_away={away} burst_toward={toward} away_attachment={away_attachment:.1} toward_attachment={toward_attachment:.1}",
        samples.len()
    );
    assert!(normal_spread.is_finite());
    assert!(control_spread.is_finite());
    assert!(normal_spread > control_spread * 2.0);
    assert!(normal_camps.0 > 0 && normal_camps.2 > 0);
    assert!(away > 0);
    assert!(away_attachment > toward_attachment);
    assert!(slope.is_none_or(f64::is_finite));
}
