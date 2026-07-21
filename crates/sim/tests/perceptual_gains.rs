//! Perceptual-gain validation reads only canonical snapshots and reports inheritance,
//! virus-associated allele change, and cultural loss without assuming their direction.

use std::collections::BTreeSet;

use anana_core::{
    Genome, Infection, NoveltyToleranceAllele, PerceptualGain, Phenotype, SkillId,
    ThreatSalienceAllele,
};
use anana_sim::{Config, build_headless_app, snapshot, step};

fn run(seed: u64, ticks: u64, virus_on: bool, fixed_median: bool) -> anana_core::WorldSnapshot {
    let mut config = Config {
        initial_population: 80,
        carrying_capacity: 220,
        ..Config::default()
    };
    if !virus_on {
        config.initial_virus.spreadscore = 0;
        config.initial_virus.virulence = 0;
        config.initial_virus.mutation_rate = anana_core::Permille::ZERO;
    }
    let mut app = build_headless_app(seed, config);
    if !virus_on {
        let entities = {
            let world = app.world_mut();
            let mut query = world.query::<(bevy::prelude::Entity, &Infection)>();
            query
                .iter(world)
                .map(|(entity, _)| entity)
                .collect::<Vec<_>>()
        };
        for entity in entities {
            app.world_mut().entity_mut(entity).remove::<Infection>();
        }
    }
    if fixed_median {
        let world = app.world_mut();
        let mut query = world.query::<(&mut Genome, &mut Phenotype)>();
        for (mut genome, mut phenotype) in query.iter_mut(world) {
            genome.threat_salience.maternal = ThreatSalienceAllele::Median;
            genome.threat_salience.paternal = ThreatSalienceAllele::Median;
            genome.novelty_tolerance.maternal = NoveltyToleranceAllele::Median;
            genome.novelty_tolerance.paternal = NoveltyToleranceAllele::Median;
            phenotype.threat_salience = PerceptualGain::MEDIAN;
            phenotype.novelty_tolerance = PerceptualGain::MEDIAN;
        }
    }
    for _ in 0..ticks {
        step(&mut app);
    }
    snapshot(&mut app)
}

fn threat_distribution(snapshot: &anana_core::WorldSnapshot) -> [u64; 3] {
    let mut counts = [0_u64; 3];
    for human in snapshot.humans.values() {
        for allele in [
            human.genome.threat_salience.maternal,
            human.genome.threat_salience.paternal,
        ] {
            let index = match allele {
                ThreatSalienceAllele::Low => 0,
                ThreatSalienceAllele::Median => 1,
                ThreatSalienceAllele::High => 2,
            };
            counts[index] = counts[index].saturating_add(1);
        }
    }
    counts
}

fn distribution_distance(left: [u64; 3], right: [u64; 3]) -> f64 {
    let left_total = left.iter().sum::<u64>().max(1) as f64;
    let right_total = right.iter().sum::<u64>().max(1) as f64;
    left.iter()
        .zip(right)
        .map(|(left, right)| (*left as f64 / left_total - right as f64 / right_total).abs())
        .sum::<f64>()
        / 2.0
}

fn pearson(samples: &[(f64, f64)]) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    let count = samples.len() as f64;
    let mean_x = samples.iter().map(|(x, _)| x).sum::<f64>() / count;
    let mean_y = samples.iter().map(|(_, y)| y).sum::<f64>() / count;
    let numerator = samples
        .iter()
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum::<f64>();
    let x_var = samples
        .iter()
        .map(|(x, _)| (x - mean_x).powi(2))
        .sum::<f64>();
    let y_var = samples
        .iter()
        .map(|(_, y)| (y - mean_y).powi(2))
        .sum::<f64>();
    let denominator = (x_var * y_var).sqrt();
    (denominator > 0.0).then_some(numerator / denominator)
}

fn parent_child_samples(
    snapshot: &anana_core::WorldSnapshot,
    gain: fn(&Phenotype) -> PerceptualGain,
) -> Vec<(f64, f64)> {
    snapshot
        .humans
        .values()
        .flat_map(|child| {
            [child.lineage.mother, child.lineage.father]
                .into_iter()
                .flatten()
                .filter_map(|parent| snapshot.humans.get(&parent))
                .map(|parent| {
                    (
                        f64::from(gain(&parent.phenotype).value()),
                        f64::from(gain(&child.phenotype).value()),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn extinct_skills(snapshot: &anana_core::WorldSnapshot) -> BTreeSet<SkillId> {
    let ever_held = snapshot
        .dead
        .values()
        .flat_map(|human| {
            human
                .skills
                .levels
                .iter()
                .filter_map(|(skill, state)| state.learned.then_some(*skill))
        })
        .collect::<BTreeSet<_>>();
    let living = snapshot
        .humans
        .values()
        .flat_map(|human| {
            human
                .skills
                .levels
                .iter()
                .filter_map(|(skill, state)| state.learned.then_some(*skill))
        })
        .collect::<BTreeSet<_>>();
    ever_held.difference(&living).copied().collect()
}

#[test]
fn parent_to_offspring_perceptual_correlations_remain_genetic_in_scale() {
    let world = run(42, 800, false, false);
    let threat = parent_child_samples(&world, |phenotype| phenotype.threat_salience);
    let novelty = parent_child_samples(&world, |phenotype| phenotype.novelty_tolerance);
    assert!(threat.len() >= 30);
    assert!(novelty.len() >= 30);
    let threat_correlation = pearson(&threat).expect("both inherited traits vary");
    let novelty_correlation = pearson(&novelty).expect("both inherited traits vary");
    println!(
        "parent_offspring threat_r={threat_correlation:.3} novelty_r={novelty_correlation:.3} pairs={}",
        threat.len()
    );
    assert!((0.20..=0.80).contains(&threat_correlation));
    assert!((0.20..=0.80).contains(&novelty_correlation));
}

#[test]
#[ignore = "long-run perceptual validation; run with cargo test -p anana-sim --test perceptual_gains long_run -- --ignored --nocapture --test-threads=1"]
fn long_run_virus_and_cultural_loss_measurements_are_reported_without_assuming_a_direction() {
    let mut virus_distances = Vec::new();
    let mut fixed_distances = Vec::new();
    let mut novelty_and_extinction = Vec::new();
    for seed in [41_u64, 42, 43, 44, 45] {
        let with_virus = run(seed, 2_000, true, false);
        let without_virus = run(seed, 2_000, false, false);
        let fixed_with = run(seed, 1_000, true, true);
        let fixed_without = run(seed, 1_000, false, true);
        virus_distances.push(distribution_distance(
            threat_distribution(&with_virus),
            threat_distribution(&without_virus),
        ));
        fixed_distances.push(distribution_distance(
            threat_distribution(&fixed_with),
            threat_distribution(&fixed_without),
        ));
        let mean_novelty = without_virus
            .humans
            .values()
            .map(|human| f64::from(human.phenotype.novelty_tolerance.value()))
            .sum::<f64>()
            / without_virus.humans.len().max(1) as f64;
        novelty_and_extinction.push((mean_novelty, extinct_skills(&without_virus).len() as f64));
    }
    let mean_virus_distance = virus_distances.iter().sum::<f64>() / virus_distances.len() as f64;
    let mean_fixed_distance = fixed_distances.iter().sum::<f64>() / fixed_distances.len() as f64;
    let extinction_correlation = pearson(&novelty_and_extinction);
    println!(
        "perceptual_validation virus_allele_distance={mean_virus_distance:.4} fixed_control_distance={mean_fixed_distance:.4} novelty_extinction_r={extinction_correlation:?} samples={novelty_and_extinction:?}"
    );
    assert_eq!(mean_fixed_distance, 0.0);
    assert!(mean_virus_distance.is_finite());
}
