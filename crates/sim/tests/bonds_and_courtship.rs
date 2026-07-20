//! These integration tests prove that repeated co-residence builds mutual courtship,
//! while newborn rearing cues remain directional and canonical in the public snapshot.

use anana_core::{HumanId, RearingAversion};
use anana_sim::{Config, EventLog, SimulationStats, build_headless_app, snapshot, step};

fn correlation(pairs: &[(f64, f64)]) -> f64 {
    let count = pairs.len() as f64;
    let left_mean = pairs.iter().map(|pair| pair.0).sum::<f64>() / count;
    let right_mean = pairs.iter().map(|pair| pair.1).sum::<f64>() / count;
    let covariance = pairs
        .iter()
        .map(|pair| (pair.0 - left_mean) * (pair.1 - right_mean))
        .sum::<f64>();
    let left_variance = pairs
        .iter()
        .map(|pair| (pair.0 - left_mean).powi(2))
        .sum::<f64>();
    let right_variance = pairs
        .iter()
        .map(|pair| (pair.1 - right_mean).powi(2))
        .sum::<f64>();
    covariance / (left_variance * right_variance).sqrt()
}

#[test]
fn courtship_precedes_the_first_birth_and_then_supports_a_population() {
    let mut app = build_headless_app(42, Config::default());
    for _ in 0..20 {
        step(&mut app);
    }
    assert_eq!(app.world().resource::<SimulationStats>().births, 0);
    for _ in 20..250 {
        step(&mut app);
    }
    assert!(app.world().resource::<SimulationStats>().births > 0);
}

#[test]
fn a_newborn_creates_a_direct_cue_in_older_residents_and_a_duration_cue_toward_them() {
    let mut app = build_headless_app(42, Config::default());
    for _ in 0..300 {
        step(&mut app);
    }
    let world = snapshot(&mut app);
    let child = world
        .humans
        .values()
        .find(|human| human.lineage.generation > 0)
        .expect("the social world produces a child");
    let older = world
        .humans
        .values()
        .find(|human| {
            human.id != child.id
                && human.residence == child.residence
                && human.lineage.birth_tick < child.lineage.birth_tick
        })
        .expect("the child has an older co-resident");
    let older_cue = older
        .social_bonds
        .rearing_aversions
        .get(&child.id)
        .expect("the older resident saw the newborn arrive");
    let younger_cue = child
        .social_bonds
        .rearing_aversions
        .get(&older.id)
        .expect("the child accumulated co-residence");
    assert!(older_cue.direct_cue);
    assert!(!younger_cue.direct_cue);
    assert_ne!(older_cue.strength(), younger_cue.strength());
}

#[test]
fn social_bonds_are_present_in_the_public_canonical_snapshot() {
    let mut app = build_headless_app(7, Config::default());
    for _ in 0..50 {
        step(&mut app);
    }
    let world = snapshot(&mut app);
    let human = world
        .humans
        .get(&HumanId(1))
        .expect("the first founder remains in this short run");
    assert!(!human.social_bonds.bonds.is_empty());
    assert!(
        human
            .social_bonds
            .rearing_aversions
            .values()
            .all(|cue: &RearingAversion| cue.strength().0 <= 1000)
    );
}

#[test]
#[ignore = "long-run partner statistic; run with cargo test -p anana-sim --test bonds_and_courtship partner_traits -- --ignored --nocapture"]
fn partner_traits_assort_in_distinct_imperfect_bands_across_seeds() {
    let mut traits = [
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ];
    for seed in [7, 42, 99] {
        let mut app = build_headless_app(seed, Config::default());
        let mut records_seen = 0;
        for _ in 0..250 {
            for _ in 0..10 {
                step(&mut app);
            }
            let world = snapshot(&mut app);
            let records = app.world().resource::<EventLog>().records();
            for record in records.iter().skip(records_seen).filter(|record| {
                record.subjects.len() == 3
                    && matches!(
                        record.payload,
                        anana_core::EventPayload::Deterministic(
                            anana_core::DeterministicKind::Maturation
                        )
                    )
            }) {
                let (Some(mother), Some(father)) = (
                    world.humans.get(&record.subjects[0]),
                    world.humans.get(&record.subjects[1]),
                ) else {
                    continue;
                };
                traits[0].push((
                    f64::from(mother.body.age_ticks) / f64::from(mother.phenotype.lifespan_ticks),
                    f64::from(father.body.age_ticks) / f64::from(father.phenotype.lifespan_ticks),
                ));
                traits[1].push((
                    f64::from(mother.instincts.social + mother.instincts.reproduction) / 2.0,
                    f64::from(father.instincts.social + father.instincts.reproduction) / 2.0,
                ));
                traits[2].push((
                    f64::from(mother.phenotype.aptitude),
                    f64::from(father.phenotype.aptitude),
                ));
                traits[3].push((
                    f64::from(mother.phenotype.robustness),
                    f64::from(father.phenotype.robustness),
                ));
                traits[4].push((
                    f64::from(mother.instincts.fear),
                    f64::from(father.instincts.fear),
                ));
                let desirability = |human: &anana_core::HumanState| {
                    let health = u32::from(human.body.health).saturating_mul(60)
                        / u32::from(human.body.max_health.max(1));
                    let skill = u32::from(human.skills.level_of(anana_core::SkillId::SocialBond))
                        .saturating_add(u32::from(
                            human.skills.level_of(anana_core::SkillId::ToolUse),
                        ))
                        .saturating_mul(4);
                    f64::from(health.saturating_add(skill).min(100))
                };
                traits[5].push((desirability(mother), desirability(father)));
            }
            records_seen = records.len();
        }
    }
    assert!(traits.iter().all(|pairs| pairs.len() > 100));
    let values = traits.map(|pairs| correlation(&pairs));
    eprintln!("partner correlations: {values:?}");
    let expected_bands = [0.35..0.90, 0.15..0.75, 0.10..0.65, 0.07..0.50, 0.05..0.40];
    assert!(
        expected_bands
            .iter()
            .zip(values.iter())
            .all(|(band, value)| band.contains(value))
    );
    assert!(values[..5].windows(2).all(|pair| pair[0] > pair[1]));
    assert!((0.02..0.98).contains(&values[5]));
}
