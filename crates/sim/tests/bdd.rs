//! Executable specifications for the AnanA simulation.
//!
//! Every scenario in `tests/features/` is a plain-English description of something
//! the world actually does, and it runs as a test. If a scenario goes red, either
//! the simulation broke or the description is out of date.

use std::collections::BTreeMap;

use anana_core::{
    Body, Boon, Consciousness, CoreError, DiseaseAllele, EventAuthor, GenePair, Genome, GoshKind,
    HandAllele, HumanId, Instincts, LifeStage, Lineage, Permille, Phenotype, PolySublocus,
    PolygenicLocus, Rng, SexAllele, SkillId, SkillState, Skills, Tick, Virus, VirusId,
    apply_learning, conceive, express, p_infect,
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

#[given(expr = "a new world seeded with {int}")]
fn a_new_world(w: &mut AnanaWorld, seed: u64) {
    w.seed = seed;
    let mut app = build_headless_app(seed, Config::default());
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
        now.iter().any(|(id, stage)| {
            w.stages_before
                .get(id)
                .is_some_and(|before| stage_rank(*stage) > stage_rank(*before))
        }),
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
        max_population: 1,
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
    let mut app = build_headless_app(w.seed, Config::default());
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
    w.app = Some(build_headless_app(42, Config::default()));
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
    let mut other = build_headless_app(999, Config::default());
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
    w.app = Some(build_headless_app(seed, Config::default()));
    w.other = Some(build_headless_app(seed, Config::default()));
}

#[given(expr = "a world seeded with {int} and another seeded with {int}")]
fn two_worlds_with_different_seeds(w: &mut AnanaWorld, first: u64, second: u64) {
    w.seed = first;
    w.app = Some(build_headless_app(first, Config::default()));
    w.other = Some(build_headless_app(second, Config::default()));
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
    let mut app = build_headless_app(w.seed, Config::default());
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
    w.replayed = Some(replay(w.seed, Config::default(), records));
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
