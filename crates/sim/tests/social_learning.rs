//! Social-learning integration tests prove that canonical co-residence changes skill acquisition without changing the seed.

use anana_core::{
    Body, Consciousness, HumanId, Instincts, LifeStage, Permille, Residence, ResidenceId, SkillId,
    SkillState, Skills, Virus, VirusId,
};
use anana_sim::{Config, EventLog, build_headless_app, step};

fn learning_world(together: bool) -> anana_sim::App {
    let mut app = build_headless_app(
        73,
        Config {
            initial_population: 2,
            carrying_capacity: 10,
            mating_interval: 0,
            mortality_interval: 0,
            initial_virus: Virus {
                id: VirusId(1),
                spreadscore: 0,
                virulence: 0,
                incubation_ticks: 1,
                mutation_rate: Permille::ZERO,
            },
            ..Config::default()
        },
    );
    let mut query = app.world_mut().query::<(
        &HumanId,
        &mut Body,
        &mut Instincts,
        &mut Consciousness,
        &mut Skills,
        &mut Residence,
    )>();
    for (id, mut body, mut instincts, mut consciousness, mut skills, mut residence) in
        query.iter_mut(app.world_mut())
    {
        body.life_stage = LifeStage::Adult;
        body.fertility = 0;
        *instincts = Instincts {
            survival: 100,
            reproduction: 0,
            hunger: 100,
            fear: 0,
            social: 100,
        };
        *consciousness = Consciousness {
            awareness: 100,
            focus: 100,
            memory_capacity: 1_000,
        };
        skills.levels.insert(
            SkillId::Recall,
            SkillState {
                xp: 100,
                learned: true,
            },
        );
        if *id == HumanId(2) {
            skills.levels.insert(
                SkillId::Motor,
                SkillState {
                    xp: 1_500,
                    learned: true,
                },
            );
            residence.id = if together {
                ResidenceId(1)
            } else {
                ResidenceId(2)
            };
        } else {
            residence.id = ResidenceId(1);
        }
    }
    app
}

fn motor_experience(app: &mut anana_sim::App) -> u32 {
    let mut query = app.world_mut().query::<(&HumanId, &Skills)>();
    query
        .iter(app.world())
        .find_map(|(id, skills)| {
            (*id == HumanId(1)).then(|| {
                skills
                    .levels
                    .get(&SkillId::Motor)
                    .map_or(0, |state| state.xp)
            })
        })
        .unwrap_or(0)
}

fn birth_world() -> anana_sim::App {
    let mut app = build_headless_app(
        91,
        Config {
            initial_population: 2,
            carrying_capacity: 10,
            mating_interval: 1,
            mortality_interval: 0,
            initial_virus: Virus {
                id: VirusId(1),
                spreadscore: 0,
                virulence: 0,
                incubation_ticks: 1,
                mutation_rate: Permille::ZERO,
            },
            ..Config::default()
        },
    );
    let mut query = app.world_mut().query::<(
        &HumanId,
        &anana_core::Phenotype,
        &mut Body,
        &mut Instincts,
        &mut Residence,
    )>();
    for (id, phenotype, mut body, mut instincts, mut residence) in query.iter_mut(app.world_mut()) {
        body.age_ticks = phenotype.lifespan_ticks.saturating_mul(2) / 5;
        body.life_stage = LifeStage::Adult;
        body.fertility = 100;
        body.health = body.max_health;
        instincts.reproduction = 100;
        residence.id = if *id == HumanId(1) {
            ResidenceId(10)
        } else {
            ResidenceId(20)
        };
    }
    app
}

#[test]
fn a_human_surrounded_by_skilled_others_learns_faster_than_the_same_human_in_isolation() {
    let mut together = learning_world(true);
    let mut isolated = learning_world(false);
    for _ in 0..50 {
        step(&mut together);
        step(&mut isolated);
    }
    assert!(motor_experience(&mut together) > motor_experience(&mut isolated));
}

#[test]
fn a_child_joins_their_mothers_canonical_residence_group_at_birth() {
    let mut app = birth_world();
    for _ in 0..100 {
        step(&mut app);
    }
    let mut query = app
        .world_mut()
        .query::<(&HumanId, &anana_core::Lineage, &Residence)>();
    let child_residence = query
        .iter(app.world())
        .find_map(|(id, lineage, residence)| {
            (id.0 > 2 && lineage.mother == Some(HumanId(1))).then_some(residence.id)
        });
    assert_eq!(child_residence, Some(ResidenceId(10)));
}

#[test]
fn a_lived_event_names_multiple_co_resident_participants_in_the_canonical_log() {
    let mut app = build_headless_app(
        42,
        Config {
            initial_population: 3,
            carrying_capacity: 10,
            mating_interval: 0,
            mortality_interval: 0,
            initial_virus: Virus {
                id: VirusId(1),
                spreadscore: 0,
                virulence: 0,
                incubation_ticks: 1,
                mutation_rate: Permille::ZERO,
            },
            ..Config::default()
        },
    );
    for _ in 0..10 {
        step(&mut app);
    }
    assert!(
        app.world()
            .resource::<EventLog>()
            .records()
            .iter()
            .any(|record| record.tick.0 == 10 && record.subjects.len() == 3)
    );
}
