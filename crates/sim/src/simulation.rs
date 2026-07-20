use std::collections::BTreeMap;

use anana_core::{
    Body, Consciousness, DiseaseAllele, EyeAllele, GenePair, Genome, God, GodId, HandAllele,
    HumanId, Infection, InfectionPhase, Instincts, LifeStage, Lineage, PolySublocus,
    PolygenicLocus, Rng, SexAllele, SkillId, SkillState, Skills, Tick, express,
};
use bevy::{
    app::ScheduleRunnerPlugin,
    prelude::{App, MinimalPlugins, Plugin, PluginGroup},
};

use crate::{
    Config, EventIntake, EventLog, Gods, HashHistory, NextHumanId, PendingBirths, SimulationFaults,
    SimulationRng, SimulationStats, Viruses, WorldClock,
};

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
            .insert_resource(EventLog::default())
            .insert_resource(EventIntake::default())
            .insert_resource(PendingBirths::default())
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
            )])));
    }
}

fn founder_genome(index: u32) -> Genome {
    let zero = PolySublocus {
        maternal: 0,
        paternal: 0,
    };
    let mixed = PolySublocus {
        maternal: 0,
        paternal: 1,
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
        robustness: PolygenicLocus {
            subloci: [mixed; 4],
        },
        aptitude: PolygenicLocus {
            subloci: [zero, mixed, mixed, zero],
        },
    }
}

fn founder_age(index: u32, lifespan_ticks: u32) -> u32 {
    let elapsed_permille = match index % 5 {
        0 => 400_u32,
        1 => 500,
        2 => 800,
        3 => 250,
        _ => 100,
    };
    lifespan_ticks.saturating_mul(elapsed_permille) / 1000
}

fn founder_fertility(stage: LifeStage) -> u8 {
    match stage {
        LifeStage::Adolescent => 35,
        LifeStage::Adult => 80,
        LifeStage::Infant | LifeStage::Child | LifeStage::Elder => 0,
    }
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
        body.age_ticks = founder_age(index, phenotype.lifespan_ticks);
        body.life_stage = Body::life_stage_for(body.age_ticks, phenotype.lifespan_ticks);
        body.fertility = founder_fertility(body.life_stage);
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
    app.world_mut().resource_mut::<SimulationStats>().living = u64::from(count);
}

pub fn build_headless_app(seed: u64, config: Config) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()))
        .add_plugins(SimPlugin::new(seed, config));
    seed_founders(&mut app);
    app
}

#[cfg(test)]
mod tests {
    //! App construction seeds stable founder components and exactly one initial infection.

    use anana_core::{Body, HumanId, Infection, Lineage, Phenotype};

    use super::*;
    use crate::{Config, NextHumanId, SimulationFaults, Viruses};

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
}
