use anana_sim::Config;
use clap::{Parser, ValueEnum};

#[derive(Clone, Copy, PartialEq, Eq, Debug, ValueEnum)]
pub(crate) enum RunMode {
    Live,
    Replay,
    Headless,
}

#[derive(Clone, Debug, Parser)]
#[command(name = "anana", about = "Deterministic life simulation")]
pub(crate) struct Cli {
    #[arg(long, default_value_t = 42)]
    pub seed: u64,
    #[arg(long)]
    pub ticks: Option<u64>,
    #[arg(long, value_enum, default_value_t = RunMode::Live)]
    pub mode: RunMode,
    #[arg(long)]
    pub offline: bool,
    #[arg(long, default_value_t = 5)]
    pub initial_population: u32,
    #[arg(long, default_value_t = 64)]
    pub max_population: u32,
    #[arg(long, default_value_t = 10)]
    pub mating_interval: u64,
}

impl Cli {
    pub(crate) fn simulation_config(&self) -> Config {
        Config {
            initial_population: self.initial_population,
            max_population: self.max_population,
            mating_interval: self.mating_interval,
            ..Config::default()
        }
    }
}
