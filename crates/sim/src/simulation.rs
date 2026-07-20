use std::collections::BTreeMap;

use anana_core::{
    Body, Consciousness, DiseaseAllele, EyeAllele, GenePair, Genome, God, GodId, HandAllele,
    HumanId, Infection, InfectionPhase, Instincts, LifeStage, Lineage, PolySublocus,
    PolygenicLocus, Residence, ResidenceId, Rng, SexAllele, SkillId, SkillState, Skills,
    SocialBonds, Tick, express,
};
use bevy::{
    app::ScheduleRunnerPlugin,
    ecs::schedule::{MultiThreadedExecutor, ScheduleLabel, SingleThreadedExecutor, SystemSet},
    prelude::{App, IntoScheduleConfigs, MinimalPlugins, Plugin, PluginGroup},
};

use crate::systems::{
    advance_clock, aging_health, birth, build_snapshot, death, events, learning, logging_and_hash,
    mating, virus_spread,
};
use crate::{
    Coalitions, Config, DeadRegistry, EventDigest, EventIntake, EventLog, Gods, HashHistory,
    NextHumanId, NextResidenceId, PendingBirths, PopulationHistory, SimulationFaults,
    SimulationRng, SimulationStats, Viruses, WorldClock,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct SimTick;

#[derive(Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
enum TickPhase {
    AdvanceClock,
    AgingHealth,
    Learning,
    Mating,
    Birth,
    VirusSpread,
    Events,
    Death,
    LoggingAndHash,
}

pub struct SimPlugin {
    seed: u64,
    config: Config,
}

impl SimPlugin {
    #[must_use]
    pub fn new(seed: u64, config: Config) -> Self {
        Self { seed, config }
    }
}

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone())
            .insert_resource(WorldClock(Tick(0)))
            .insert_resource(SimulationRng(Rng::new(self.seed)))
            .insert_resource(NextHumanId(HumanId(1)))
            .insert_resource(NextResidenceId(ResidenceId(1)))
            .insert_resource(EventLog::default())
            .insert_resource(EventDigest::default())
            .insert_resource(EventIntake::default())
            .insert_resource(PendingBirths::default())
            .insert_resource(DeadRegistry::default())
            .insert_resource(PopulationHistory::default())
            .insert_resource(SimulationStats::default())
            .insert_resource(SimulationFaults::default())
            .insert_resource(HashHistory::default())
            .insert_resource(Viruses(BTreeMap::from([(
                self.config.initial_virus.id,
                self.config.initial_virus.clone(),
            )])))
            .insert_resource(Gods(BTreeMap::from([(
                GodId(1),
                God {
                    id: GodId(1),
                    goshes_spoken: 0,
                },
            )])))
            .insert_resource(Coalitions::default())
            .init_schedule(SimTick)
            .configure_sets(
                SimTick,
                (
                    TickPhase::AdvanceClock,
                    TickPhase::AgingHealth,
                    TickPhase::Learning,
                    TickPhase::Mating,
                    TickPhase::Birth,
                    TickPhase::VirusSpread,
                    TickPhase::Events,
                    TickPhase::Death,
                    TickPhase::LoggingAndHash,
                )
                    .chain(),
            )
            .add_systems(SimTick, advance_clock.in_set(TickPhase::AdvanceClock))
            .add_systems(SimTick, aging_health.in_set(TickPhase::AgingHealth))
            .add_systems(SimTick, learning.in_set(TickPhase::Learning))
            .add_systems(SimTick, mating.in_set(TickPhase::Mating))
            .add_systems(SimTick, birth.in_set(TickPhase::Birth))
            .add_systems(SimTick, virus_spread.in_set(TickPhase::VirusSpread))
            .add_systems(SimTick, events.in_set(TickPhase::Events))
            .add_systems(SimTick, death.in_set(TickPhase::Death))
            .add_systems(SimTick, logging_and_hash.in_set(TickPhase::LoggingAndHash));
        if self.config.requested_threads <= 1 {
            app.edit_schedule(SimTick, |schedule| {
                schedule.set_executor(SingleThreadedExecutor::new());
            });
        } else {
            app.edit_schedule(SimTick, |schedule| {
                schedule.set_executor(MultiThreadedExecutor::new());
            });
        }
    }
}

fn founder_genome(index: u32) -> Genome {
    let robustness = PolygenicLocus {
        subloci: std::array::from_fn(|locus| {
            let variable = locus < 2;
            let dose = ((index >> locus) & 1) as u8;
            PolySublocus {
                maternal: if variable { dose } else { 0 },
                paternal: if variable { dose } else { 1 },
            }
        }),
    };
    let aptitude_code = index.saturating_mul(37).saturating_add(11);
    let aptitude = PolygenicLocus {
        subloci: std::array::from_fn(|locus| {
            let variable = locus < 2;
            let dose = ((aptitude_code >> locus) & 1) as u8;
            PolySublocus {
                maternal: if variable { dose } else { 0 },
                paternal: if variable { dose } else { 1 },
            }
        }),
    };
    let sex = if index.is_multiple_of(2) {
        GenePair {
            maternal: SexAllele::X,
            paternal: SexAllele::X,
        }
    } else {
        GenePair {
            maternal: SexAllele::X,
            paternal: SexAllele::Y,
        }
    };
    Genome {
        eye: GenePair {
            maternal: EyeAllele::Brown,
            paternal: if index.is_multiple_of(3) {
                EyeAllele::Blue
            } else {
                EyeAllele::Brown
            },
        },
        hand: GenePair {
            maternal: HandAllele::Right,
            paternal: if index.is_multiple_of(2) {
                HandAllele::Left
            } else {
                HandAllele::Right
            },
        },
        disease_x: GenePair {
            maternal: DiseaseAllele::Healthy,
            paternal: if index.is_multiple_of(4) {
                DiseaseAllele::Risk
            } else {
                DiseaseAllele::Healthy
            },
        },
        sex,
        robustness,
        aptitude,
    }
}

fn founder_age(index: u32, count: u32, lifespan_ticks: u32) -> u32 {
    let rank = u64::from(index.saturating_add(1));
    let count = u64::from(count.max(1));
    let elapsed_permille =
        rank.saturating_mul(rank).saturating_mul(900) / count.saturating_mul(count);
    u64::from(lifespan_ticks)
        .saturating_mul(elapsed_permille)
        .saturating_div(1000)
        .min(u64::from(u32::MAX)) as u32
}

fn founder_consciousness(stage: LifeStage) -> Consciousness {
    let (awareness, focus, memory_capacity) = match stage {
        LifeStage::Infant => (3, 10, 20),
        LifeStage::Child => (15, 35, 200),
        LifeStage::Adolescent => (45, 60, 500),
        LifeStage::Adult => (80, 80, 900),
        LifeStage::Elder => (90, 70, 1000),
    };
    Consciousness {
        awareness,
        focus,
        memory_capacity,
    }
}

fn founder_skills(stage: LifeStage) -> Skills {
    let mut skills = Skills::default();
    if matches!(stage, LifeStage::Adult | LifeStage::Elder) {
        skills.levels.insert(
            SkillId::Recall,
            SkillState {
                xp: 100,
                learned: true,
            },
        );
    }
    skills
}

fn seed_founders(app: &mut App) {
    let count = app.world().resource::<Config>().initial_population;
    let seed = app.world().resource::<SimulationRng>().0;
    let initial_virus = app.world().resource::<Config>().initial_virus.clone();
    for index in 0..count {
        let residence_id = ResidenceId(u64::from(index / 25).saturating_add(1));
        let id = match app.world_mut().resource_mut::<NextHumanId>().allocate() {
            Ok(id) => id,
            Err(error) => {
                app.world_mut()
                    .resource_mut::<SimulationFaults>()
                    .0
                    .push(error);
                continue;
            }
        };
        let genome = founder_genome(index);
        let phenotype = express(&genome, &seed, Tick(0), id);
        let mut body = Body::at_birth(&phenotype);
        body.age_ticks = founder_age(index, count, phenotype.lifespan_ticks);
        body.life_stage = Body::life_stage_for(body.age_ticks, phenotype.lifespan_ticks);
        body.fertility =
            crate::systems::fertility_for_age(body.age_ticks, phenotype.lifespan_ticks);
        if body.life_stage == LifeStage::Elder {
            body.health /= 2;
        }
        let instincts = Instincts {
            survival: 55_u8.saturating_add((index % 20) as u8),
            reproduction: 70_u8.saturating_sub((index % 20) as u8),
            hunger: 50,
            fear: 35_u8.saturating_add((index % 30) as u8),
            social: 65_u8.saturating_sub((index % 25) as u8),
        };
        let consciousness = founder_consciousness(body.life_stage);
        let skills = founder_skills(body.life_stage);
        let lineage = Lineage::new(id, None, None, 0, Tick(0));
        let mut entity = app.world_mut().spawn((
            id,
            genome,
            phenotype,
            instincts,
            consciousness,
            body,
            skills,
            lineage,
            Residence { id: residence_id },
            SocialBonds::default(),
        ));
        if index == 0 {
            entity.insert(Infection {
                strain: initial_virus.id,
                ticks: initial_virus.incubation_ticks,
                severity: initial_virus.virulence,
                phase: InfectionPhase::Infectious,
            });
        }
    }
    let stats = &mut *app.world_mut().resource_mut::<SimulationStats>();
    stats.living = u64::from(count);
    stats.surviving_founder_lineages = count;
    app.world_mut().resource_mut::<NextResidenceId>().0 =
        ResidenceId(u64::from(count.saturating_add(24) / 25).saturating_add(1));
}

pub fn build_headless_app(seed: u64, config: Config) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()))
        .add_plugins(SimPlugin::new(seed, config));
    seed_founders(&mut app);
    app
}

pub fn step(app: &mut App) {
    app.world_mut().run_schedule(SimTick);
}

/// Returns the current canonical state without changing the simulation.
pub fn snapshot(app: &mut App) -> anana_core::WorldSnapshot {
    build_snapshot(app.world_mut())
}

#[cfg(test)]
mod tests {
    //! App construction seeds stable founder components and exactly one initial infection.

    use std::collections::BTreeMap;

    use anana_core::{Body, Boon, EventAuthor, GoshKind, HumanId, Infection, Lineage, Phenotype};

    use super::*;
    use crate::{
        Config, EventIntake, EventLog, HashHistory, NextHumanId, SimulationFaults, Viruses,
        WorldClock,
    };

    #[test]
    fn headless_app_construction_seeds_the_requested_founders_in_domain_id_order() {
        let config = Config {
            initial_population: 5,
            ..Config::default()
        };
        let mut app = build_headless_app(42, config);
        let mut query = app
            .world_mut()
            .query::<(&HumanId, &Phenotype, &Body, &Lineage)>();
        let mut founders = query
            .iter(app.world())
            .map(|(id, phenotype, body, lineage)| {
                (*id, phenotype.clone(), body.clone(), lineage.clone())
            })
            .collect::<Vec<_>>();
        founders.sort_by_key(|founder| founder.0);
        assert_eq!(
            founders.iter().map(|founder| founder.0).collect::<Vec<_>>(),
            vec![HumanId(1), HumanId(2), HumanId(3), HumanId(4), HumanId(5)]
        );
        assert!(
            founders
                .iter()
                .all(
                    |(_, phenotype, body, lineage)| body.max_health == phenotype.base_max_health
                        && lineage.generation == 0
                )
        );
        assert_eq!(app.world().resource::<NextHumanId>().0, HumanId(6));
    }

    #[test]
    fn founder_world_starts_with_one_virus_one_infection_and_no_faults() {
        let mut app = build_headless_app(42, Config::default());
        let infected = app
            .world_mut()
            .query::<&Infection>()
            .iter(app.world())
            .count();
        assert_eq!(infected, 1);
        assert_eq!(app.world().resource::<Viruses>().0.len(), 1);
        assert!(app.world().resource::<SimulationFaults>().0.is_empty());
    }

    #[test]
    fn one_explicit_step_advances_the_clock_and_every_living_body_once() {
        let mut app = build_headless_app(42, Config::default());
        let before = app
            .world_mut()
            .query::<(&HumanId, &Body)>()
            .iter(app.world())
            .map(|(id, body)| (*id, body.age_ticks))
            .collect::<BTreeMap<_, _>>();
        step(&mut app);
        assert_eq!(app.world().resource::<WorldClock>().0.0, 1);
        let after = app
            .world_mut()
            .query::<(&HumanId, &Body)>()
            .iter(app.world())
            .map(|(id, body)| (*id, body.age_ticks))
            .collect::<BTreeMap<_, _>>();
        assert!(
            before
                .iter()
                .all(|(id, age)| after.get(id) == Some(&age.saturating_add(1)))
        );
    }

    #[test]
    fn a_captured_gosh_is_logged_and_reproduces_the_same_tick_hash() {
        let mut first = build_headless_app(42, Config::default());
        let mut second = build_headless_app(42, Config::default());
        for app in [&mut first, &mut second] {
            let mut query = app.world_mut().query::<(&HumanId, &mut Body)>();
            for (id, mut body) in query.iter_mut(app.world_mut()) {
                if *id == HumanId(1) {
                    body.health = body.health.saturating_sub(10);
                }
            }
            app.world()
                .resource::<EventIntake>()
                .cast_gosh(
                    app.world().resource::<WorldClock>().0,
                    GoshKind::Bless {
                        subject: HumanId(1),
                        boon: Boon::Heal(5),
                    },
                )
                .expect("intake is available");
            step(app);
        }
        let records = first.world().resource::<EventLog>().records();
        let gosh = records
            .iter()
            .find(|record| record.author == EventAuthor::God)
            .expect("the captured gosh is recorded among same-tick world events");
        assert_eq!(gosh.tick.0, 0);
        assert_eq!(
            first.world().resource::<HashHistory>().0,
            second.world().resource::<HashHistory>().0
        );
    }
}
