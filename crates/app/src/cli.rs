use anana_sim::Config;
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub(crate) enum RunMode {
    Live,
    Replay,
    Headless,
}

#[derive(Clone, Debug, Parser)]
#[command(name = "anana", about = "Deterministic life simulation")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
    #[arg(long, default_value_t = 42)]
    pub seed: u64,
    #[arg(long)]
    pub ticks: Option<u64>,
    #[arg(long, value_enum, default_value_t = RunMode::Live)]
    pub mode: RunMode,
    #[arg(long)]
    pub offline: bool,
    #[arg(long, default_value_t = 80)]
    pub initial_population: u32,
    #[arg(long, alias = "max-population", default_value_t = 300)]
    pub carrying_capacity: u32,
    #[arg(long, default_value_t = 10)]
    pub mating_interval: u64,
}

#[derive(Clone, Debug, Subcommand)]
pub(crate) enum Command {
    /// Compare an untouched future with one changed by a single divine decree.
    Counterfactual(CounterfactualArgs),
}

#[derive(Clone, Debug, Args)]
pub(crate) struct CounterfactualArgs {
    /// Master seed of the world to project.
    #[arg(long, default_value_t = 42)]
    pub seed: u64,
    /// Tick at which the untouched world splits into two projected futures.
    #[arg(long)]
    pub branch_at: u64,
    /// Tick at which both projected futures are compared.
    #[arg(long)]
    pub horizon: u64,
    /// The canonical JSON gosh object produced by gosh-mode; use null for no decree.
    #[arg(long)]
    pub gosh: String,
    /// Print the complete comparison as structured JSON.
    #[arg(long)]
    pub json: bool,
    #[arg(long, default_value_t = 80)]
    pub initial_population: u32,
    #[arg(long, alias = "max-population", default_value_t = 300)]
    pub carrying_capacity: u32,
    #[arg(long, default_value_t = 10)]
    pub mating_interval: u64,
}

impl CounterfactualArgs {
    pub(crate) fn simulation_config(&self) -> Config {
        Config {
            initial_population: self.initial_population,
            carrying_capacity: self.carrying_capacity,
            mating_interval: self.mating_interval,
            ..Config::default()
        }
    }
}

impl Cli {
    pub(crate) fn simulation_config(&self) -> Config {
        Config {
            initial_population: self.initial_population,
            carrying_capacity: self.carrying_capacity,
            mating_interval: self.mating_interval,
            ..Config::default()
        }
    }
}
