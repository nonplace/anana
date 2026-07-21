use std::collections::{BTreeMap, BTreeSet};

use anana_core::{
    GoshKind, HumanId, HumanState, Lineage, SkillId, Tick, WorldSnapshot, event_log_hash,
    world_hash,
};
use serde::Serialize;
use thiserror::Error;

use crate::{
    App, Coalitions, Config, DeadRegistry, EventDigest, EventIntake, EventLog, Gods, HashHistory,
    NextHumanId, NextResidenceId, PopulationHistory, SimulationFaults, SimulationStats, Viruses,
    WorldClock, build_empty_app, snapshot, step,
};

const EXAMPLE_LIMIT: usize = 5;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CounterfactualRequest {
    pub seed: u64,
    pub config: Config,
    pub branch_at: u64,
    pub horizon: u64,
    pub decree: Option<GoshKind>,
}

#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum CounterfactualError {
    #[error("the horizon tick {horizon} must be after the branch tick {branch_at}")]
    HorizonNotAfterBranch { branch_at: u64, horizon: u64 },
    #[error("the counterfactual simulation failed: {0}")]
    Simulation(#[from] crate::SimError),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchSide {
    Untouched,
    Decreed,
}

impl std::fmt::Display for BranchSide {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Untouched => formatter.write_str("untouched"),
            Self::Decreed => formatter.write_str("decreed"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize)]
pub struct BranchScopedHumanId {
    pub branch: BranchSide,
    pub local_id: HumanId,
}

impl std::fmt::Display for BranchScopedHumanId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}:H{}", self.branch, self.local_id.0)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IndividualFate {
    Alive { age_ticks: u32 },
    Died { death_tick: Tick, age_ticks: u32 },
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct BranchIndividualDifference {
    pub human: HumanId,
    pub untouched: IndividualFate,
    pub decreed: IndividualFate,
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize)]
pub struct PopulationDifference {
    pub living_delta: i64,
    pub births_delta: i64,
    pub deaths_delta: i64,
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize)]
pub struct LineageDifference {
    pub only_untouched: BTreeSet<HumanId>,
    pub only_decreed: BTreeSet<HumanId>,
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize)]
pub struct PostBranchDifference {
    pub birth_count_delta: i64,
    pub never_born_in_decreed: u64,
    pub additional_births_in_decreed: u64,
    pub untouched_examples: Vec<BranchScopedHumanId>,
    pub decreed_examples: Vec<BranchScopedHumanId>,
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize)]
pub struct KnowledgeDifference {
    pub only_untouched: BTreeSet<SkillId>,
    pub only_decreed: BTreeSet<SkillId>,
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize)]
pub struct CounterfactualDifferences {
    pub worlds_diverged: bool,
    pub population: PopulationDifference,
    pub branch_individuals: Vec<BranchIndividualDifference>,
    pub lineages: LineageDifference,
    pub post_branch: PostBranchDifference,
    pub knowledge: KnowledgeDifference,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct ContinuationPopulation {
    pub living: u64,
    pub births_after_branch: u64,
    pub deaths_after_branch: u64,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct ContinuationSummary {
    pub world_hash: [u8; 32],
    pub population: ContinuationPopulation,
    pub surviving_lineages: u64,
    pub living_knowledge: BTreeSet<SkillId>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct CounterfactualIdentity {
    pub seed: u64,
    pub branch_at: Tick,
    pub branch_world_hash: [u8; 32],
    pub decree: Option<GoshKind>,
    pub horizon: Tick,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct CounterfactualComparison {
    pub identity: CounterfactualIdentity,
    pub untouched: ContinuationSummary,
    pub decreed: ContinuationSummary,
    pub differences: CounterfactualDifferences,
}

struct ProjectedContinuation {
    snapshot: WorldSnapshot,
    hash: [u8; 32],
}

fn restore_snapshot(snapshot: WorldSnapshot, config: Config) -> Result<App, CounterfactualError> {
    let seed = snapshot.seed;
    let branch_hash = world_hash(&snapshot);
    let event_hash = event_log_hash(&snapshot.event_log);
    let records_hashed = snapshot.event_log.len();
    let event_log = EventLog::from_records(snapshot.event_log)?;
    let living = snapshot.humans.len() as u64;
    let deaths = snapshot.dead.len() as u64;
    let allocated = snapshot.next_human_id.0.saturating_sub(1);
    let births = allocated.saturating_sub(u64::from(config.initial_population));
    let mut app = build_empty_app(seed, config);

    app.insert_resource(WorldClock(snapshot.tick))
        .insert_resource(NextHumanId(snapshot.next_human_id))
        .insert_resource(NextResidenceId(snapshot.next_residence_id))
        .insert_resource(event_log)
        .insert_resource(EventDigest {
            hash: event_hash,
            records_hashed,
        })
        .insert_resource(DeadRegistry(snapshot.dead))
        .insert_resource(Viruses(snapshot.viruses))
        .insert_resource(Gods(snapshot.gods))
        .insert_resource(Coalitions(snapshot.coalitions))
        .insert_resource(PopulationHistory::default())
        .insert_resource(HashHistory(vec![branch_hash]))
        .insert_resource(SimulationStats {
            births,
            deaths,
            living,
            ..SimulationStats::default()
        })
        .insert_resource(SimulationFaults::default());

    for human in snapshot.humans.into_values() {
        spawn_restored_human(&mut app, human);
    }
    Ok(app)
}

fn spawn_restored_human(app: &mut App, human: HumanState) {
    let infection = human.infection.clone();
    let mut entity = app.world_mut().spawn((
        human.id,
        human.genome,
        human.phenotype,
        human.instincts,
        human.consciousness,
        human.body,
        human.skills,
        human.lineage,
        human.residence,
        human.social_bonds,
    ));
    if let Some(infection) = infection {
        entity.insert(infection);
    }
}

fn final_hash(app: &mut App) -> [u8; 32] {
    app.world()
        .resource::<HashHistory>()
        .0
        .last()
        .copied()
        .unwrap_or_else(|| world_hash(&snapshot(app)))
}

fn post_branch_births(snapshot: &WorldSnapshot, branch_next_id: HumanId) -> u64 {
    snapshot.next_human_id.0.saturating_sub(branch_next_id.0)
}

fn post_branch_deaths(snapshot: &WorldSnapshot, branch_tick: Tick) -> u64 {
    snapshot
        .dead
        .values()
        .filter(|human| human.death_tick.0 > branch_tick.0)
        .count() as u64
}

fn living_knowledge(snapshot: &WorldSnapshot) -> BTreeSet<SkillId> {
    snapshot
        .humans
        .values()
        .flat_map(|human| {
            human
                .skills
                .levels
                .iter()
                .filter_map(|(skill, state)| state.learned.then_some(*skill))
        })
        .collect()
}

fn lineage_map(snapshot: &WorldSnapshot) -> BTreeMap<HumanId, Lineage> {
    snapshot
        .humans
        .iter()
        .map(|(id, human)| (*id, human.lineage.clone()))
        .chain(
            snapshot
                .dead
                .iter()
                .map(|(id, human)| (*id, human.lineage.clone())),
        )
        .collect()
}

fn lineage_roots(subject: HumanId, lineages: &BTreeMap<HumanId, Lineage>) -> BTreeSet<HumanId> {
    let mut roots = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut pending = vec![subject];
    while let Some(id) = pending.pop() {
        if !visited.insert(id) {
            continue;
        }
        let Some(lineage) = lineages.get(&id) else {
            continue;
        };
        if lineage.mother.is_none() && lineage.father.is_none() {
            roots.insert(id);
        } else {
            pending.extend(lineage.mother);
            pending.extend(lineage.father);
        }
    }
    roots
}

fn surviving_lineages(snapshot: &WorldSnapshot) -> BTreeSet<HumanId> {
    let lineages = lineage_map(snapshot);
    snapshot
        .humans
        .keys()
        .flat_map(|id| lineage_roots(*id, &lineages))
        .collect()
}

fn fate(
    snapshot: &WorldSnapshot,
    id: HumanId,
    branch_age: u32,
    branch_tick: Tick,
) -> Option<IndividualFate> {
    if let Some(human) = snapshot.humans.get(&id) {
        return Some(IndividualFate::Alive {
            age_ticks: human.body.age_ticks,
        });
    }
    snapshot.dead.get(&id).map(|dead| IndividualFate::Died {
        death_tick: dead.death_tick,
        age_ticks: branch_age.saturating_add(
            dead.death_tick
                .0
                .saturating_sub(branch_tick.0)
                .min(u64::from(u32::MAX)) as u32,
        ),
    })
}

fn branch_individual_differences(
    branch: &WorldSnapshot,
    untouched: &WorldSnapshot,
    decreed: &WorldSnapshot,
) -> Vec<BranchIndividualDifference> {
    branch
        .humans
        .iter()
        .filter_map(|(id, human)| {
            let untouched_fate = fate(untouched, *id, human.body.age_ticks, branch.tick)?;
            let decreed_fate = fate(decreed, *id, human.body.age_ticks, branch.tick)?;
            (untouched_fate != decreed_fate).then_some(BranchIndividualDifference {
                human: *id,
                untouched: untouched_fate,
                decreed: decreed_fate,
            })
        })
        .collect()
}

fn signed_delta(left: u64, right: u64) -> i64 {
    if left >= right {
        i64::try_from(left.saturating_sub(right)).unwrap_or(i64::MAX)
    } else {
        -i64::try_from(right.saturating_sub(left)).unwrap_or(i64::MAX)
    }
}

fn post_branch_examples(
    side: BranchSide,
    branch_next_id: HumanId,
    count: u64,
) -> Vec<BranchScopedHumanId> {
    (0..count.min(EXAMPLE_LIMIT as u64))
        .map(|offset| BranchScopedHumanId {
            branch: side,
            local_id: HumanId(branch_next_id.0.saturating_add(offset)),
        })
        .collect()
}

fn continuation_summary(
    snapshot: &WorldSnapshot,
    hash: [u8; 32],
    branch: &WorldSnapshot,
) -> ContinuationSummary {
    let lineages = surviving_lineages(snapshot);
    ContinuationSummary {
        world_hash: hash,
        population: ContinuationPopulation {
            living: snapshot.humans.len() as u64,
            births_after_branch: post_branch_births(snapshot, branch.next_human_id),
            deaths_after_branch: post_branch_deaths(snapshot, branch.tick),
        },
        surviving_lineages: lineages.len() as u64,
        living_knowledge: living_knowledge(snapshot),
    }
}

fn compare(
    seed: u64,
    decree: Option<GoshKind>,
    branch: WorldSnapshot,
    horizon: Tick,
    untouched: ProjectedContinuation,
    decreed: ProjectedContinuation,
) -> CounterfactualComparison {
    let untouched_summary = continuation_summary(&untouched.snapshot, untouched.hash, &branch);
    let decreed_summary = continuation_summary(&decreed.snapshot, decreed.hash, &branch);
    let untouched_lineages = surviving_lineages(&untouched.snapshot);
    let decreed_lineages = surviving_lineages(&decreed.snapshot);
    let existed_at_branch = |id: &&HumanId| id.0 < branch.next_human_id.0;
    let births_delta = signed_delta(
        untouched_summary.population.births_after_branch,
        decreed_summary.population.births_after_branch,
    );
    let never_born = u64::try_from(births_delta.max(0)).unwrap_or(u64::MAX);
    let additional = births_delta
        .checked_neg()
        .and_then(|value| u64::try_from(value.max(0)).ok())
        .unwrap_or(0);
    let untouched_knowledge = untouched_summary.living_knowledge.clone();
    let decreed_knowledge = decreed_summary.living_knowledge.clone();
    let differences = CounterfactualDifferences {
        worlds_diverged: untouched.hash != decreed.hash,
        population: PopulationDifference {
            living_delta: signed_delta(
                untouched_summary.population.living,
                decreed_summary.population.living,
            ),
            births_delta,
            deaths_delta: signed_delta(
                untouched_summary.population.deaths_after_branch,
                decreed_summary.population.deaths_after_branch,
            ),
        },
        branch_individuals: branch_individual_differences(
            &branch,
            &untouched.snapshot,
            &decreed.snapshot,
        ),
        lineages: LineageDifference {
            only_untouched: untouched_lineages
                .difference(&decreed_lineages)
                .filter(existed_at_branch)
                .copied()
                .collect(),
            only_decreed: decreed_lineages
                .difference(&untouched_lineages)
                .filter(existed_at_branch)
                .copied()
                .collect(),
        },
        post_branch: PostBranchDifference {
            birth_count_delta: births_delta,
            never_born_in_decreed: never_born,
            additional_births_in_decreed: additional,
            untouched_examples: post_branch_examples(
                BranchSide::Untouched,
                branch.next_human_id,
                never_born,
            ),
            decreed_examples: post_branch_examples(
                BranchSide::Decreed,
                branch.next_human_id,
                additional,
            ),
        },
        knowledge: KnowledgeDifference {
            only_untouched: untouched_knowledge
                .difference(&decreed_knowledge)
                .copied()
                .collect(),
            only_decreed: decreed_knowledge
                .difference(&untouched_knowledge)
                .copied()
                .collect(),
        },
    };
    CounterfactualComparison {
        identity: CounterfactualIdentity {
            seed,
            branch_at: branch.tick,
            branch_world_hash: world_hash(&branch),
            decree,
            horizon,
        },
        untouched: untouched_summary,
        decreed: decreed_summary,
        differences,
    }
}

pub fn project_counterfactual(
    branch_world: &mut App,
    horizon: u64,
    decree: Option<GoshKind>,
) -> Result<CounterfactualComparison, CounterfactualError> {
    let branch = snapshot(branch_world);
    if horizon <= branch.tick.0 {
        return Err(CounterfactualError::HorizonNotAfterBranch {
            branch_at: branch.tick.0,
            horizon,
        });
    }
    let config = branch_world.world().resource::<Config>().clone();

    // Identity policy: humans alive here keep their HumanId and are compared directly. Every
    // later birth is reported through a branch-scoped identity and is compared only in aggregate,
    // because two lives with different causal origins are not the same person.
    let mut untouched = restore_snapshot(branch.clone(), config.clone())?;
    let mut decreed = restore_snapshot(branch.clone(), config)?;
    if let Some(gosh) = decree.clone() {
        decreed
            .world()
            .resource::<EventIntake>()
            .cast_gosh(branch.tick, gosh)?;
    }
    let remaining = horizon.saturating_sub(branch.tick.0);
    for _ in 0..remaining {
        step(&mut untouched);
    }
    for _ in 0..remaining {
        step(&mut decreed);
    }
    let untouched_hash = final_hash(&mut untouched);
    let decreed_hash = final_hash(&mut decreed);
    let untouched_snapshot = snapshot(&mut untouched);
    let decreed_snapshot = snapshot(&mut decreed);
    Ok(compare(
        branch.seed,
        decree,
        branch,
        Tick(horizon),
        ProjectedContinuation {
            snapshot: untouched_snapshot,
            hash: untouched_hash,
        },
        ProjectedContinuation {
            snapshot: decreed_snapshot,
            hash: decreed_hash,
        },
    ))
}

pub fn run_counterfactual(
    request: CounterfactualRequest,
) -> Result<CounterfactualComparison, CounterfactualError> {
    if request.horizon <= request.branch_at {
        return Err(CounterfactualError::HorizonNotAfterBranch {
            branch_at: request.branch_at,
            horizon: request.horizon,
        });
    }
    let mut branch = crate::build_headless_app(request.seed, request.config);
    for _ in 0..request.branch_at {
        step(&mut branch);
    }
    project_counterfactual(&mut branch, request.horizon, request.decree)
}

fn hash_hex(hash: [u8; 32]) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn format_ids(values: &BTreeSet<HumanId>) -> String {
    if values.is_empty() {
        return String::from("none");
    }
    values
        .iter()
        .take(EXAMPLE_LIMIT)
        .map(|id| format!("H{}", id.0))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_skills(values: &BTreeSet<SkillId>) -> String {
    if values.is_empty() {
        return String::from("none");
    }
    values
        .iter()
        .map(|skill| format!("{skill:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_fate(fate: &IndividualFate) -> String {
    match fate {
        IndividualFate::Alive { age_ticks } => format!("alive at age {age_ticks} ticks"),
        IndividualFate::Died {
            death_tick,
            age_ticks,
        } => format!("died at tick {}, age {} ticks", death_tick.0, age_ticks),
    }
}

impl std::fmt::Display for CounterfactualComparison {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(formatter, "COUNTERFACTUAL")?;
        writeln!(
            formatter,
            "seed={} branch_tick={} horizon={} branch_hash={}",
            self.identity.seed,
            self.identity.branch_at.0,
            self.identity.horizon.0,
            hash_hex(self.identity.branch_world_hash)
        )?;
        writeln!(formatter, "decree={:?}", self.identity.decree)?;
        writeln!(formatter)?;
        writeln!(formatter, "WORLD HASHES")?;
        writeln!(
            formatter,
            "  untouched {}",
            hash_hex(self.untouched.world_hash)
        )?;
        writeln!(
            formatter,
            "  decreed   {}",
            hash_hex(self.decreed.world_hash)
        )?;
        writeln!(formatter)?;
        writeln!(formatter, "POPULATION AT HORIZON")?;
        writeln!(
            formatter,
            "  untouched living={} births_after_branch={} deaths_after_branch={}",
            self.untouched.population.living,
            self.untouched.population.births_after_branch,
            self.untouched.population.deaths_after_branch
        )?;
        writeln!(
            formatter,
            "  decreed   living={} births_after_branch={} deaths_after_branch={}",
            self.decreed.population.living,
            self.decreed.population.births_after_branch,
            self.decreed.population.deaths_after_branch
        )?;
        writeln!(formatter)?;
        writeln!(formatter, "PEOPLE PRESENT AT THE BRANCH")?;
        writeln!(
            formatter,
            "  different fates={}",
            self.differences.branch_individuals.len()
        )?;
        if self.differences.branch_individuals.is_empty() {
            writeln!(formatter, "  examples: none")?;
        } else {
            for difference in self
                .differences
                .branch_individuals
                .iter()
                .take(EXAMPLE_LIMIT)
            {
                writeln!(
                    formatter,
                    "  H{}: untouched={}, decreed={}",
                    difference.human.0,
                    format_fate(&difference.untouched),
                    format_fate(&difference.decreed)
                )?;
            }
        }
        writeln!(formatter)?;
        writeln!(formatter, "LINEAGES")?;
        writeln!(
            formatter,
            "  surviving: untouched={} decreed={}",
            self.untouched.surviving_lineages, self.decreed.surviving_lineages
        )?;
        writeln!(
            formatter,
            "  extinct only after decree: {}",
            format_ids(&self.differences.lineages.only_untouched)
        )?;
        writeln!(
            formatter,
            "  surviving only after decree: {}",
            format_ids(&self.differences.lineages.only_decreed)
        )?;
        writeln!(formatter)?;
        writeln!(formatter, "LIVES AFTER THE BRANCH")?;
        writeln!(
            formatter,
            "  never born after decree={} additional births after decree={}",
            self.differences.post_branch.never_born_in_decreed,
            self.differences.post_branch.additional_births_in_decreed
        )?;
        let examples = self
            .differences
            .post_branch
            .untouched_examples
            .iter()
            .chain(&self.differences.post_branch.decreed_examples)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        writeln!(
            formatter,
            "  examples: {}",
            if examples.is_empty() {
                String::from("none")
            } else {
                examples.join(", ")
            }
        )?;
        writeln!(formatter)?;
        writeln!(formatter, "KNOWLEDGE HELD BY THE LIVING")?;
        writeln!(
            formatter,
            "  held: untouched={} decreed={}",
            self.untouched.living_knowledge.len(),
            self.decreed.living_knowledge.len()
        )?;
        writeln!(
            formatter,
            "  lost after decree: {}",
            format_skills(&self.differences.knowledge.only_untouched)
        )?;
        writeln!(
            formatter,
            "  present only after decree: {}",
            format_skills(&self.differences.knowledge.only_decreed)
        )?;
        writeln!(formatter)?;
        write!(
            formatter,
            "This compares two runs of a model; it is not a claim about real people."
        )
    }
}
