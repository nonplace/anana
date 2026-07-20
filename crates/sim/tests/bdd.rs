//! Executable specifications for the AnanA simulation.
//!
//! Every scenario in `tests/features/` is a plain-English description of something
//! the world actually does, and it runs as a test. If a scenario goes red, either
//! the simulation broke or the description is out of date.

use std::collections::BTreeMap;

use anana_core::{
    Body, Boon, Consciousness, CoreError, DeterministicKind, DiseaseAllele, EventAuthor,
    EventPayload, GenePair, Genome, GoshKind, HandAllele, HumanId, Instincts, LifeStage, Lineage,
    ObservationFactors, Permille, Phenotype, PolySublocus, PolygenicLocus, PracticeKind, Residence,
    ResidenceId, Rng, SexAllele, SkillId, SkillState, Skills, Tick, Virus, VirusId, apply_learning,
    conceive, decay_unpractised, express, observational_gain, optimal_teaching_gap, p_infect,
    practise_skill, teaching_gain,
};
use anana_sim::{
    App, Config, EventIntake, EventLog, HashHistory, NextHumanId, SimulationStats, WorldClock,
    build_headless_app, replay, snapshot, step,
};
use cucumber::{World as _, given, then, when};

#[derive(Default, cucumber::World)]
pub struct AnanaWorld {
    seed: u64,
    app: Option<App>,
    other: Option<App>,
    replayed: Option<App>,
    ages_before: BTreeMap<HumanId, u32>,
    stages_before: BTreeMap<HumanId, LifeStage>,
    mother: Option<Genome>,
    father: Option<Genome>,
    child: Option<Genome>,
    second_child: Option<Genome>,
    expressed_before: Option<(HumanId, Phenotype)>,
    skills: Option<Skills>,
    consciousness: Option<Consciousness>,
    learning_phenotype: Option<Phenotype>,
    learning_result: Option<Result<(), CoreError>>,
    full_practice_gain: u32,
    virus: Option<Virus>,
    second_virus: Option<Virus>,
    probability: Option<Permille>,
    selected: Option<HumanId>,
    health_before: u16,
    first_healing: u16,
    second_healing: u16,
    history_before: usize,
    original_hashes: Vec<[u8; 32]>,
    birth_ticks: Vec<Tick>,
    open_births: u64,
    crowded_births: u64,
    dead_subject: Option<HumanId>,
    social_values: Vec<u32>,
    social_gaps: Vec<u16>,
}

impl std::fmt::Debug for AnanaWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnanaWorld")
            .field("seed", &self.seed)
            .finish_non_exhaustive()
    }
}

fn locus(maternal: u8, paternal: u8) -> PolygenicLocus {
    PolygenicLocus {
        subloci: [PolySublocus { maternal, paternal }; 4],
    }
}

fn known_genome(female: bool, carrier: bool) -> Genome {
    Genome {
        eye: GenePair {
            maternal: anana_core::EyeAllele::Brown,
            paternal: anana_core::EyeAllele::Blue,
        },
        hand: GenePair {
            maternal: HandAllele::Right,
            paternal: HandAllele::Left,
        },
        disease_x: GenePair {
            maternal: DiseaseAllele::Healthy,
            paternal: if carrier {
                DiseaseAllele::Risk
            } else {
                DiseaseAllele::Healthy
            },
        },
        sex: GenePair {
            maternal: SexAllele::X,
            paternal: if female { SexAllele::X } else { SexAllele::Y },
        },
        robustness: locus(0, 1),
        aptitude: locus(1, 0),
    }
}

fn learning_phenotype() -> Phenotype {
    Phenotype {
        sex: anana_core::Sex::Female,
        eye_color: anana_core::EyeColor::Brown,
        handedness: anana_core::Handedness::Right,
        disease_x: anana_core::DiseaseStatus::Clear,
        robustness: 4,
        aptitude: 0,
        base_max_health: 100,
        learning_rate: Permille::ONE,
        lifespan_ticks: 22_000,
    }
}

fn remembering_skills() -> Skills {
    let mut skills = Skills::default();
    skills.levels.insert(
        SkillId::Recall,
        SkillState {
            xp: 100,
            learned: true,
        },
    );
    skills
}

fn complete_observation() -> ObservationFactors {
    ObservationFactors {
        attention: Permille::ONE,
        retention: Permille::ONE,
        reproduction: Permille::ONE,
        motivation: Permille::ONE,
    }
}

#[given("an attentive remembering adult watches a more capable neighbour")]
fn an_attentive_adult_watches_a_capable_neighbour(w: &mut AnanaWorld) {
    w.social_values = vec![observational_gain(100, 80, 20, complete_observation()), 100];
}

#[when("their observational learning is compared with doing the same task")]
fn observational_learning_is_compared_with_direct_experience(_w: &mut AnanaWorld) {}

#[then("watching produces some learning but less than direct experience")]
fn watching_helps_less_than_doing(w: &mut AnanaWorld) {
    assert!(w.social_values[0] > 0);
    assert!(w.social_values[0] < w.social_values[1]);
}

#[given("four otherwise ready observers each missing one stage of observation")]
fn four_observers_each_lack_one_stage(w: &mut AnanaWorld) {
    w.social_values.clear();
    for missing in 0..4 {
        let mut factors = complete_observation();
        match missing {
            0 => factors.attention = Permille::ZERO,
            1 => factors.retention = Permille::ZERO,
            2 => factors.reproduction = Permille::ZERO,
            _ => factors.motivation = Permille::ZERO,
        }
        w.social_values
            .push(observational_gain(100, 80, 20, factors));
    }
}

#[when("each watches the same capable neighbour")]
fn each_observer_watches_the_same_neighbour(_w: &mut AnanaWorld) {}

#[then("none of the four observers learns from watching")]
fn no_incomplete_observer_learns(w: &mut AnanaWorld) {
    assert!(w.social_values.iter().all(|gain| *gain == 0));
}

#[given("a beginner can choose a peer, a nearby teacher, or a distant expert")]
fn a_beginner_can_choose_three_teachers(w: &mut AnanaWorld) {
    let beginner = 0;
    let nearby = optimal_teaching_gap(beginner);
    w.social_values = vec![
        teaching_gain(beginner, beginner, 1_000),
        teaching_gain(beginner, nearby, 1_000),
        teaching_gain(beginner, 100, 1_000),
    ];
}

#[when("the beginner receives the same length lesson from each")]
fn the_beginner_receives_equal_lessons(_w: &mut AnanaWorld) {}

#[then("the nearby teacher transfers the most")]
fn the_nearby_teacher_transfers_most(w: &mut AnanaWorld) {
    assert!(w.social_values[1] > w.social_values[0]);
    assert!(w.social_values[1] > w.social_values[2]);
}

#[given("a beginner and an already capable learner can choose among teachers")]
fn learners_at_two_levels_can_choose_teachers(w: &mut AnanaWorld) {
    w.social_gaps = vec![optimal_teaching_gap(10), optimal_teaching_gap(80)];
}

#[when("each chooses the lesson that transfers the most")]
fn each_learner_chooses_the_best_lesson(_w: &mut AnanaWorld) {}

#[then("the capable learner chooses a teacher further ahead")]
fn the_capable_learner_chooses_a_wider_gap(w: &mut AnanaWorld) {
    assert!(w.social_gaps[1] > w.social_gaps[0]);
}

#[given("equal experience is massed for one learner and retrieved over time by another")]
fn equal_experience_is_massed_and_spaced(w: &mut AnanaWorld) {
    let mind = Consciousness {
        awareness: 100,
        focus: 100,
        memory_capacity: 1_000,
    };
    let phenotype = learning_phenotype();
    let mut massed = remembering_skills();
    let mut spaced = remembering_skills();
    for tick in [1, 2, 3] {
        practise_skill(
            &mut massed,
            &mind,
            &phenotype,
            SkillId::Motor,
            100,
            Tick(tick),
            PracticeKind::Restudy,
        )
        .expect("massed practice is available");
    }
    for tick in [1, 21, 41] {
        practise_skill(
            &mut spaced,
            &mind,
            &phenotype,
            SkillId::Motor,
            100,
            Tick(tick),
            PracticeKind::Retrieval,
        )
        .expect("spaced retrieval is available");
    }
    w.social_values = vec![
        massed.levels[&SkillId::Motor].xp,
        spaced.levels[&SkillId::Motor].xp,
    ];
    decay_unpractised(&mut massed, Tick(241));
    decay_unpractised(&mut spaced, Tick(241));
    w.social_values.extend([
        massed.levels[&SkillId::Motor].xp,
        spaced.levels[&SkillId::Motor].xp,
    ]);
}

#[when("both are tested immediately and again after a long delay")]
fn both_are_tested_now_and_later(_w: &mut AnanaWorld) {}

#[then("massed restudy looks better immediately but spaced retrieval lasts longer")]
fn the_retrieval_advantage_reverses_with_delay(w: &mut AnanaWorld) {
    assert!(w.social_values[0] > w.social_values[1]);
    assert!(w.social_values[3] > w.social_values[2]);
}

fn virus(spreadscore: u8) -> Virus {
    Virus {
        id: VirusId(1),
        spreadscore,
        virulence: 20,
        incubation_ticks: 8,
        mutation_rate: Permille::ZERO,
    }
}

fn age_map(app: &mut App) -> BTreeMap<HumanId, u32> {
    let mut query = app.world_mut().query::<(&HumanId, &Body)>();
    query
        .iter(app.world())
        .map(|(id, body)| (*id, body.age_ticks))
        .collect()
}

fn stage_map(app: &mut App) -> BTreeMap<HumanId, LifeStage> {
    let mut query = app.world_mut().query::<(&HumanId, &Body)>();
    query
        .iter(app.world())
        .map(|(id, body)| (*id, body.life_stage))
        .collect()
}

fn stage_rank(stage: LifeStage) -> u8 {
    match stage {
        LifeStage::Infant => 0,
        LifeStage::Child => 1,
        LifeStage::Adolescent => 2,
        LifeStage::Adult => 3,
        LifeStage::Elder => 4,
    }
}

fn health_of(app: &mut App, subject: HumanId) -> u16 {
    let mut query = app.world_mut().query::<(&HumanId, &Body)>();
    query
        .iter(app.world())
        .find_map(|(id, body)| (*id == subject).then_some(body.health))
        .expect("the scenario names a living human")
}

fn injure(app: &mut App, subject: HumanId, amount: u16) -> u16 {
    let mut query = app.world_mut().query::<(&HumanId, &mut Body)>();
    for (id, mut body) in query.iter_mut(app.world_mut()) {
        if *id == subject {
            body.health = body.health.saturating_sub(amount);
            return body.health;
        }
    }
    0
}

fn cast_healing(app: &App, subject: HumanId) {
    app.world()
        .resource::<EventIntake>()
        .cast_gosh(
            app.world().resource::<WorldClock>().0,
            GoshKind::Bless {
                subject,
                boon: Boon::Heal(10),
            },
        )
        .expect("the scenario event intake is available");
}

fn spawn_scenario_human(app: &mut App, id: HumanId, female: bool, age_permille: u32) {
    let genome = known_genome(female, false);
    let phenotype = express(&genome, &Rng::new(42), Tick(0), id);
    let mut body = Body::at_birth(&phenotype);
    body.age_ticks = phenotype.lifespan_ticks.saturating_mul(age_permille) / 1000;
    body.life_stage = Body::life_stage_for(body.age_ticks, phenotype.lifespan_ticks);
    body.fertility = if (200..700).contains(&age_permille) {
        100
    } else {
        0
    };
    let mut skills = Skills::default();
    skills.levels.insert(
        SkillId::Recall,
        SkillState {
            xp: 100,
            learned: true,
        },
    );
    app.world_mut().spawn((
        id,
        genome,
        phenotype,
        Instincts {
            survival: 90,
            reproduction: 100,
            hunger: 50,
            fear: 40,
            social: 80,
        },
        Consciousness {
            awareness: 80,
            focus: 80,
            memory_capacity: 900,
        },
        body,
        skills,
        Lineage::new(id, None, None, 0, Tick(0)),
        Residence { id: ResidenceId(1) },
    ));
}

fn couple_world(extra_children: u64) -> App {
    let mut app = build_headless_app(
        42,
        Config {
            initial_population: 0,
            carrying_capacity: 20,
            initial_virus: virus(0),
            ..Config::default()
        },
    );
    spawn_scenario_human(&mut app, HumanId(1), true, 400);
    spawn_scenario_human(&mut app, HumanId(2), false, 400);
    for offset in 0..extra_children {
        spawn_scenario_human(&mut app, HumanId(3 + offset), offset.is_multiple_of(2), 0);
    }
    app.world_mut().resource_mut::<NextHumanId>().0 = HumanId(3 + extra_children);
    app.world_mut().resource_mut::<SimulationStats>().living = 2 + extra_children;
    app
}

fn compact_spec_config() -> Config {
    Config {
        initial_population: 5,
        carrying_capacity: 32,
        ..Config::default()
    }
}

#[given("a healthy fertile couple in an otherwise empty world")]
fn a_healthy_fertile_couple_in_an_empty_world(w: &mut AnanaWorld) {
    w.app = Some(couple_world(0));
}

#[when("their world advances through several chances to conceive")]
fn their_world_advances_through_several_chances_to_conceive(w: &mut AnanaWorld) {
    let app = w.app.as_mut().expect("a fertile world was prepared");
    for _ in 0..240 {
        step(app);
    }
    w.birth_ticks = app
        .world()
        .resource::<EventLog>()
        .records()
        .iter()
        .filter(|record| {
            record.subjects.len() == 3
                && matches!(
                    record.payload,
                    EventPayload::Deterministic(DeterministicKind::Maturation)
                )
        })
        .map(|record| record.tick)
        .collect();
}

#[then("their children are born with recovery time between births")]
fn their_children_are_spaced_apart(w: &mut AnanaWorld) {
    assert!(w.birth_ticks.len() >= 2, "births={:?}", w.birth_ticks);
    assert!(w.birth_ticks.windows(2).all(|pair| {
        pair.get(1)
            .zip(pair.first())
            .is_some_and(|(later, earlier)| later.0.saturating_sub(earlier.0) >= 40)
    }));
}

#[given("two equally fertile worlds, one open and one nearly full")]
fn two_equally_fertile_worlds_with_different_crowding(w: &mut AnanaWorld) {
    w.app = Some(couple_world(0));
    w.other = Some(couple_world(14));
}

#[when("both worlds reach a chance to conceive")]
fn both_worlds_reach_chances_to_conceive(w: &mut AnanaWorld) {
    let open = w.app.as_mut().expect("an open world was prepared");
    let crowded = w.other.as_mut().expect("a crowded world was prepared");
    for _ in 0..400 {
        step(open);
        step(crowded);
    }
    w.open_births = open.world().resource::<SimulationStats>().births;
    w.crowded_births = crowded.world().resource::<SimulationStats>().births;
}

#[then("the nearly full world has fewer births without forbidding them at a wall")]
fn crowding_dampens_births_without_a_hard_wall(w: &mut AnanaWorld) {
    assert!(w.crowded_births > 0);
    assert!(
        w.crowded_births < w.open_births,
        "open={}, crowded={}",
        w.open_births,
        w.crowded_births
    );
}

#[given("a living human whose life is about to end")]
fn a_living_human_near_the_end_of_life(w: &mut AnanaWorld) {
    let mut app = build_headless_app(
        42,
        Config {
            initial_population: 0,
            carrying_capacity: 1,
            initial_virus: virus(0),
            ..Config::default()
        },
    );
    let subject = HumanId(1);
    spawn_scenario_human(&mut app, subject, false, 999);
    app.world_mut().resource_mut::<NextHumanId>().0 = HumanId(2);
    app.world_mut().resource_mut::<SimulationStats>().living = 1;
    w.dead_subject = Some(subject);
    w.app = Some(app);
}

#[when("the world advances beyond that life")]
fn the_world_advances_beyond_that_life(w: &mut AnanaWorld) {
    let app = w
        .app
        .as_mut()
        .expect("a nearly completed life was prepared");
    for _ in 0..10 {
        step(app);
    }
}

#[then("the human is gone from the living population")]
fn the_dead_human_is_no_longer_living(w: &mut AnanaWorld) {
    let subject = w.dead_subject.expect("a dying human was prepared");
    let app = w.app.as_mut().expect("a running world was prepared");
    assert!(!snapshot(app).humans.contains_key(&subject));
}

#[then("their lineage and learned skills remain in the world's memory")]
fn the_dead_humans_lineage_and_skills_remain(w: &mut AnanaWorld) {
    let subject = w.dead_subject.expect("a dying human was prepared");
    let app = w.app.as_mut().expect("a running world was prepared");
    let world = snapshot(app);
    let remembered = world
        .dead
        .get(&subject)
        .expect("the dead human is remembered");
    assert_eq!(remembered.lineage.id, subject);
    assert!(remembered.skills.recall_learned());
}

#[given(expr = "a new society seeded with {int}")]
fn a_new_society_seeded_with(w: &mut AnanaWorld, seed: u64) {
    w.seed = seed;
    w.app = Some(build_headless_app(seed, Config::default()));
}

#[when(expr = "the society lives through {int} ticks")]
fn the_society_lives_through_ticks(w: &mut AnanaWorld, ticks: u64) {
    let app = w.app.as_mut().expect("a society was prepared");
    for _ in 0..ticks {
        step(app);
    }
}

#[then("hundreds of people remain alive within the world's carrying capacity")]
fn hundreds_remain_within_capacity(w: &mut AnanaWorld) {
    let app = w.app.as_ref().expect("a society was prepared");
    let living = app.world().resource::<SimulationStats>().living;
    assert!((200..=300).contains(&living), "living={living}");
}

#[then("the society has reached at least five generations")]
fn society_reaches_several_generations(w: &mut AnanaWorld) {
    let app = w.app.as_ref().expect("a society was prepared");
    let generation = app.world().resource::<SimulationStats>().deepest_generation;
    assert!(generation >= 5, "generation={generation}");
}

#[given(expr = "a new world seeded with {int}")]
fn a_new_world(w: &mut AnanaWorld, seed: u64) {
    w.seed = seed;
    let mut app = build_headless_app(seed, compact_spec_config());
    w.ages_before = age_map(&mut app);
    w.stages_before = stage_map(&mut app);
    w.app = Some(app);
}

#[when(expr = "the world advances {int} tick(s)")]
fn the_world_advances(w: &mut AnanaWorld, ticks: u64) {
    let app = w.app.as_mut().expect("a running world was prepared");
    for _ in 0..ticks {
        step(app);
    }
}

#[when(expr = "the world advances {int} ticks of practice")]
fn the_world_advances_through_practice(w: &mut AnanaWorld, ticks: u64) {
    let skills = w.skills.as_mut().expect("skills were prepared");
    let consciousness = w.consciousness.as_ref().expect("a mind was prepared");
    let phenotype = w
        .learning_phenotype
        .as_ref()
        .expect("a phenotype was prepared");
    for _ in 0..ticks {
        w.learning_result = Some(apply_learning(
            skills,
            consciousness,
            phenotype,
            SkillId::Motor,
            100,
        ));
    }
}

#[then(expr = "the world clock reads tick {int}")]
fn the_clock_reads(w: &mut AnanaWorld, expected: u64) {
    let app = w.app.as_ref().expect("a running world was prepared");
    assert_eq!(app.world().resource::<WorldClock>().0, Tick(expected));
}

#[then("every living human is one tick older")]
fn every_living_human_is_older(w: &mut AnanaWorld) {
    let app = w.app.as_mut().expect("a running world was prepared");
    for (id, age) in age_map(app) {
        assert_eq!(
            Some(&age),
            w.ages_before.get(&id).map(|before| before + 1).as_ref()
        );
    }
}

#[then("at least one human has reached a later stage of life than they were born into")]
fn at_least_one_human_reached_a_later_stage(w: &mut AnanaWorld) {
    let app = w.app.as_mut().expect("a running world was prepared");
    let now = stage_map(app);
    assert!(
        now.values()
            .any(|stage| stage_rank(*stage) > stage_rank(LifeStage::Infant)),
        "before={:?}, now={now:?}",
        w.stages_before
    );
}

#[given("a mother and a father with known genes")]
fn parents_with_known_genes(w: &mut AnanaWorld) {
    w.seed = 42;
    w.mother = Some(known_genome(true, true));
    w.father = Some(known_genome(false, false));
}

#[when("they conceive a child")]
fn they_conceive_a_child(w: &mut AnanaWorld) {
    w.child = Some(conceive(
        w.mother.as_ref().expect("a mother was prepared"),
        w.father.as_ref().expect("a father was prepared"),
        &Rng::new(w.seed),
        Tick(1),
        HumanId(100),
    ));
}

#[when("they conceive a child twice from the same seed")]
fn they_conceive_twice(w: &mut AnanaWorld) {
    they_conceive_a_child(w);
    w.second_child = Some(conceive(
        w.mother.as_ref().expect("a mother was prepared"),
        w.father.as_ref().expect("a father was prepared"),
        &Rng::new(w.seed),
        Tick(1),
        HumanId(100),
    ));
}

#[then("the child carries one copy from the mother and one from the father at every gene")]
fn the_child_carries_one_copy_from_each_parent(w: &mut AnanaWorld) {
    let mother = w.mother.as_ref().expect("a mother was prepared");
    let father = w.father.as_ref().expect("a father was prepared");
    let child = w.child.as_ref().expect("a child was conceived");
    assert!([mother.eye.maternal, mother.eye.paternal].contains(&child.eye.maternal));
    assert!([father.eye.maternal, father.eye.paternal].contains(&child.eye.paternal));
    assert!([mother.hand.maternal, mother.hand.paternal].contains(&child.hand.maternal));
    assert!([father.hand.maternal, father.hand.paternal].contains(&child.hand.paternal));
    assert!(
        [mother.disease_x.maternal, mother.disease_x.paternal].contains(&child.disease_x.maternal)
    );
    assert!(
        [father.disease_x.maternal, father.disease_x.paternal].contains(&child.disease_x.paternal)
    );
    assert!([mother.sex.maternal, mother.sex.paternal].contains(&child.sex.maternal));
    assert!([father.sex.maternal, father.sex.paternal].contains(&child.sex.paternal));
    for index in 0..4 {
        let maternal = mother.robustness.subloci[index];
        let paternal = father.robustness.subloci[index];
        assert!(
            [maternal.maternal, maternal.paternal]
                .contains(&child.robustness.subloci[index].maternal)
        );
        assert!(
            [paternal.maternal, paternal.paternal]
                .contains(&child.robustness.subloci[index].paternal)
        );
    }
}

#[then("both children are genetically identical")]
fn both_children_are_identical(w: &mut AnanaWorld) {
    assert_eq!(w.child, w.second_child);
}

#[given("a parent who carries the disease gene without showing the disease")]
fn a_hidden_disease_carrier(w: &mut AnanaWorld) {
    w.seed = 73;
    w.mother = Some(known_genome(true, true));
    w.father = Some(known_genome(false, false));
}

#[when("they pass their genes to a child")]
fn the_carrier_passes_genes(w: &mut AnanaWorld) {
    let mother = w.mother.as_ref().expect("a carrier parent was prepared");
    let father = w.father.as_ref().expect("another parent was prepared");
    w.child = (1..=2_000).find_map(|child| {
        let genome = conceive(mother, father, &Rng::new(w.seed), Tick(1), HumanId(child));
        (genome.disease_x.maternal == DiseaseAllele::Risk).then_some(genome)
    });
}

#[then("the child can still inherit the disease gene")]
fn the_child_inherits_the_hidden_gene(w: &mut AnanaWorld) {
    let child = w
        .child
        .as_ref()
        .expect("the sample found an inheriting child");
    assert!(
        child.disease_x.maternal == DiseaseAllele::Risk
            || child.disease_x.paternal == DiseaseAllele::Risk
    );
}

#[given("a newborn whose traits have been expressed")]
fn a_newborn_with_expressed_traits(w: &mut AnanaWorld) {
    w.seed = 42;
    let config = Config {
        initial_population: 0,
        carrying_capacity: 1,
        ..Config::default()
    };
    let mut app = build_headless_app(w.seed, config);
    let id = HumanId(1);
    let genome = known_genome(true, true);
    let phenotype = express(&genome, &Rng::new(w.seed), Tick(0), id);
    app.world_mut().spawn((
        id,
        genome,
        phenotype.clone(),
        Instincts {
            survival: 50,
            reproduction: 50,
            hunger: 50,
            fear: 50,
            social: 50,
        },
        Consciousness {
            awareness: 1,
            focus: 10,
            memory_capacity: 20,
        },
        Body::at_birth(&phenotype),
        Skills::default(),
        Lineage::new(id, None, None, 0, Tick(0)),
        Residence { id: ResidenceId(1) },
    ));
    app.world_mut().resource_mut::<NextHumanId>().0 = HumanId(2);
    app.world_mut().resource_mut::<SimulationStats>().living = 1;
    w.expressed_before = Some((id, phenotype));
    w.app = Some(app);
}

#[then("the newborn's expressed traits are unchanged")]
fn the_newborns_traits_are_unchanged(w: &mut AnanaWorld) {
    let (id, expected) = w.expressed_before.as_ref().expect("a newborn was prepared");
    let app = w.app.as_mut().expect("a running world was prepared");
    let current = snapshot(app)
        .humans
        .get(id)
        .expect("the newborn remains alive")
        .phenotype
        .clone();
    assert_eq!(&current, expected);
}

#[given("a newborn who has not learned Recall")]
fn a_newborn_without_recall(w: &mut AnanaWorld) {
    w.skills = Some(Skills::default());
    w.consciousness = Some(Consciousness {
        awareness: 100,
        focus: 100,
        memory_capacity: 1_000,
    });
    w.learning_phenotype = Some(learning_phenotype());
    w.full_practice_gain = 2_000;
}

#[given("a human who has just learned Recall")]
fn a_human_with_recall(w: &mut AnanaWorld) {
    a_newborn_without_recall(w);
    w.skills
        .as_mut()
        .expect("skills were prepared")
        .levels
        .insert(
            SkillId::Recall,
            SkillState {
                xp: 100,
                learned: true,
            },
        );
}

#[given("a human whose awareness is below the threshold for Recall")]
fn a_mind_below_the_recall_gate(w: &mut AnanaWorld) {
    w.skills = Some(Skills::default());
    w.consciousness = Some(Consciousness {
        awareness: 4,
        focus: 100,
        memory_capacity: 20,
    });
    w.learning_phenotype = Some(learning_phenotype());
}

#[when("they try to learn Recall")]
fn they_try_to_learn_recall(w: &mut AnanaWorld) {
    w.learning_result = Some(apply_learning(
        w.skills.as_mut().expect("skills were prepared"),
        w.consciousness.as_ref().expect("a mind was prepared"),
        w.learning_phenotype
            .as_ref()
            .expect("a phenotype was prepared"),
        SkillId::Recall,
        100,
    ));
}

#[then("their skill experience decays instead of accumulating")]
fn experience_decays_instead_of_accumulating(w: &mut AnanaWorld) {
    let xp = w
        .skills
        .as_ref()
        .and_then(|skills| skills.levels.get(&SkillId::Motor))
        .map_or(0, |state| state.xp);
    assert!(xp > 0 && xp < w.full_practice_gain);
}

#[then("no skill has been marked as learned")]
fn no_skill_is_learned(w: &mut AnanaWorld) {
    assert!(
        w.skills
            .as_ref()
            .expect("skills were prepared")
            .levels
            .values()
            .all(|state| !state.learned)
    );
}

#[then("their skill experience accumulates")]
fn experience_accumulates(w: &mut AnanaWorld) {
    let xp = w
        .skills
        .as_ref()
        .expect("skills were prepared")
        .levels
        .get(&SkillId::Motor)
        .expect("motor practice was recorded")
        .xp;
    assert_eq!(xp, w.full_practice_gain);
}

#[then("a practised skill can be marked as learned")]
fn a_practised_skill_is_learned(w: &mut AnanaWorld) {
    assert!(
        w.skills
            .as_ref()
            .expect("skills were prepared")
            .levels
            .get(&SkillId::Motor)
            .is_some_and(|state| state.learned)
    );
}

#[then("the attempt is refused because the skill is locked")]
fn the_attempt_is_refused(w: &mut AnanaWorld) {
    assert_eq!(
        w.learning_result,
        Some(Err(CoreError::SkillLocked(SkillId::Recall)))
    );
}

#[given(expr = "a virus with a spreadscore of {int}")]
fn a_virus_with_spreadscore(w: &mut AnanaWorld, spreadscore: u8) {
    w.virus = Some(virus(spreadscore));
}

#[given(expr = "a second virus with a spreadscore of {int}")]
fn a_second_virus_with_spreadscore(w: &mut AnanaWorld, spreadscore: u8) {
    w.second_virus = Some(virus(spreadscore));
}

#[when("a completely exposed human is contacted")]
fn a_completely_exposed_human_is_contacted(w: &mut AnanaWorld) {
    w.probability = Some(p_infect(
        w.virus.as_ref().expect("a virus was prepared"),
        Permille::ZERO,
        Permille::ZERO,
        Permille::ONE,
        Permille::ZERO,
    ));
}

#[when("a maximally resistant, fearful and well-treated human is contacted")]
fn a_maximally_defended_human_is_contacted(w: &mut AnanaWorld) {
    w.probability = Some(p_infect(
        w.virus.as_ref().expect("a virus was prepared"),
        Permille::ONE,
        Permille::ONE,
        Permille::ZERO,
        Permille::ONE,
    ));
}

#[then("the chance of infection is none")]
fn infection_chance_is_none(w: &mut AnanaWorld) {
    assert_eq!(w.probability, Some(Permille::ZERO));
}

#[then("the chance of infection is certain")]
fn infection_chance_is_certain(w: &mut AnanaWorld) {
    assert_eq!(w.probability, Some(Permille::ONE));
}

#[then("the more contagious virus is at least as likely to infect")]
fn more_contagious_is_not_less_likely(w: &mut AnanaWorld) {
    let modifiers = (Permille(150), Permille(200), Permille(700), Permille(250));
    let lower = p_infect(
        w.virus.as_ref().expect("a lower virus was prepared"),
        modifiers.0,
        modifiers.1,
        modifiers.2,
        modifiers.3,
    );
    let higher = p_infect(
        w.second_virus
            .as_ref()
            .expect("a higher virus was prepared"),
        modifiers.0,
        modifiers.1,
        modifiers.2,
        modifiers.3,
    );
    assert!(higher >= lower);
}

#[given("a running world with an injured human")]
fn a_running_world_with_an_injured_human(w: &mut AnanaWorld) {
    w.seed = 42;
    let mut app = build_headless_app(w.seed, compact_spec_config());
    let subject = HumanId(2);
    w.health_before = injure(&mut app, subject, 20);
    w.selected = Some(subject);
    w.app = Some(app);
}

#[given("a running world where a human has been blessed")]
fn a_running_world_with_a_blessed_human(w: &mut AnanaWorld) {
    a_running_world_with_an_injured_human(w);
    let subject = w.selected.expect("an injured human was selected");
    let app = w.app.as_mut().expect("a running world was prepared");
    cast_healing(app, subject);
    step(app);
}

#[given("a running world")]
fn a_running_world(w: &mut AnanaWorld) {
    w.app = Some(build_headless_app(42, compact_spec_config()));
    w.selected = Some(HumanId(2));
    w.history_before = 0;
}

#[when("the god blesses that human with healing")]
fn the_god_blesses_with_healing(w: &mut AnanaWorld) {
    let subject = w.selected.expect("an injured human was selected");
    let app = w.app.as_mut().expect("a running world was prepared");
    cast_healing(app, subject);
    step(app);
}

#[when("the same blessing is spoken in two worlds started from different seeds")]
fn the_same_blessing_is_spoken_in_different_worlds(w: &mut AnanaWorld) {
    let subject = w.selected.expect("an injured human was selected");
    let mut other = build_headless_app(999, compact_spec_config());
    let other_before = injure(&mut other, subject, 20);
    let first = w.app.as_mut().expect("the first world was prepared");
    cast_healing(first, subject);
    cast_healing(&other, subject);
    step(first);
    step(&mut other);
    w.first_healing = health_of(first, subject).saturating_sub(w.health_before);
    w.second_healing = health_of(&mut other, subject).saturating_sub(other_before);
    w.other = Some(other);
}

#[when("the god inspects a human without speaking")]
fn the_god_only_inspects(w: &mut AnanaWorld) {
    let app = w.app.as_ref().expect("a running world was prepared");
    w.history_before = app.world().resource::<EventLog>().records().len();
    let _selected = w.selected;
}

#[then("that human's health has increased")]
fn the_humans_health_increased(w: &mut AnanaWorld) {
    let subject = w.selected.expect("an injured human was selected");
    let app = w.app.as_mut().expect("a running world was prepared");
    assert!(health_of(app, subject) > w.health_before);
}

#[then("the blessing appears in the world's history")]
#[then("the blessing is still recorded in the world's history")]
fn the_blessing_is_in_history(w: &mut AnanaWorld) {
    let app = w.app.as_ref().expect("a running world was prepared");
    assert!(
        app.world()
            .resource::<EventLog>()
            .records()
            .iter()
            .any(|record| record.author == EventAuthor::God
                && matches!(
                    record.payload,
                    anana_core::EventPayload::Gosh(GoshKind::Bless { .. })
                ))
    );
}

#[then("the blessing has exactly the same effect in both")]
fn the_blessing_has_the_same_effect(w: &mut AnanaWorld) {
    assert_eq!(w.first_healing, w.second_healing);
}

#[then("the world's history is unchanged")]
fn the_history_is_unchanged(w: &mut AnanaWorld) {
    let app = w.app.as_ref().expect("a running world was prepared");
    assert_eq!(
        app.world().resource::<EventLog>().records().len(),
        w.history_before
    );
}

#[given(expr = "two worlds both seeded with {int}")]
fn two_worlds_with_the_same_seed(w: &mut AnanaWorld, seed: u64) {
    w.seed = seed;
    w.app = Some(build_headless_app(seed, compact_spec_config()));
    w.other = Some(build_headless_app(seed, compact_spec_config()));
}

#[given(expr = "a world seeded with {int} and another seeded with {int}")]
fn two_worlds_with_different_seeds(w: &mut AnanaWorld, first: u64, second: u64) {
    w.seed = first;
    w.app = Some(build_headless_app(first, compact_spec_config()));
    w.other = Some(build_headless_app(second, compact_spec_config()));
}

#[when(expr = "both worlds advance {int} ticks")]
fn both_worlds_advance(w: &mut AnanaWorld, ticks: u64) {
    let first = w.app.as_mut().expect("the first world was prepared");
    let second = w.other.as_mut().expect("the second world was prepared");
    for _ in 0..ticks {
        step(first);
        step(second);
    }
}

#[then("the two worlds are identical at every tick")]
fn both_worlds_are_identical(w: &mut AnanaWorld) {
    assert_eq!(
        w.app
            .as_ref()
            .expect("the first world was prepared")
            .world()
            .resource::<HashHistory>()
            .0,
        w.other
            .as_ref()
            .expect("the second world was prepared")
            .world()
            .resource::<HashHistory>()
            .0
    );
}

#[then("the two worlds have diverged")]
fn both_worlds_have_diverged(w: &mut AnanaWorld) {
    assert_ne!(
        w.app
            .as_ref()
            .expect("the first world was prepared")
            .world()
            .resource::<HashHistory>()
            .0,
        w.other
            .as_ref()
            .expect("the second world was prepared")
            .world()
            .resource::<HashHistory>()
            .0
    );
}

#[given("a world that has run 100 ticks and recorded its history")]
fn a_world_with_recorded_history(w: &mut AnanaWorld) {
    w.seed = 42;
    let mut app = build_headless_app(w.seed, compact_spec_config());
    for _ in 0..100 {
        step(&mut app);
    }
    w.original_hashes = app.world().resource::<HashHistory>().0.clone();
    w.app = Some(app);
}

#[when("that history is replayed from the same seed")]
fn that_history_is_replayed(w: &mut AnanaWorld) {
    let records = w
        .app
        .as_ref()
        .expect("the original world was prepared")
        .world()
        .resource::<EventLog>()
        .records()
        .to_vec();
    w.replayed = Some(replay(w.seed, compact_spec_config(), records));
}

#[then("the replayed world matches the original exactly")]
fn the_replayed_world_matches(w: &mut AnanaWorld) {
    assert_eq!(
        w.replayed
            .as_ref()
            .expect("the history was replayed")
            .world()
            .resource::<HashHistory>()
            .0,
        w.original_hashes
    );
}

#[tokio::main]
async fn main() {
    AnanaWorld::cucumber()
        .fail_on_skipped()
        .run_and_exit("tests/features")
        .await;
}
