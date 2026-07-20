mod cli;
mod driver;
mod terminal;

use anana_mind::{AnyMind, GptMind, OfflineMind};
use anyhow::Result;
use clap::Parser;

use cli::{Cli, RunMode};
use driver::{hash_hex, run_headless, run_live, run_replay};

fn select_mind(force_offline: bool) -> AnyMind {
    if force_offline {
        return AnyMind::Offline(OfflineMind);
    }
    match std::env::var("OPENAI_API_KEY") {
        Ok(api_key) if !api_key.is_empty() => {
            GptMind::new(api_key).map_or(AnyMind::Offline(OfflineMind), AnyMind::Gpt)
        }
        _ => AnyMind::Offline(OfflineMind),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = cli.simulation_config();
    match cli.mode {
        RunMode::Live => run_live(cli.seed, config, cli.ticks, &select_mind(cli.offline)).await,
        RunMode::Replay => run_replay(cli.seed, config, cli.ticks.unwrap_or(5_000)),
        RunMode::Headless => {
            let result = run_headless(
                cli.seed,
                config,
                cli.ticks.unwrap_or(5_000),
                &select_mind(cli.offline),
            )
            .await?;
            println!(
                "hash={} tick={} living={} births={} deaths={} infections={} generation={} lineages={} lived={} faults={}",
                hash_hex(result.final_hash),
                result.tick,
                result.stats.living,
                result.stats.births,
                result.stats.deaths,
                result.stats.infections,
                result.stats.deepest_generation,
                result.stats.surviving_founder_lineages,
                result.stats.living.saturating_add(result.stats.deaths),
                result.faults.len(),
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    //! The CLI accepts documented modes and a fixed-seed headless run remains deterministic and fault-free.

    use anana_mind::OfflineMind;
    use clap::Parser;

    use super::*;

    #[test]
    fn documented_cli_arguments_are_accepted_and_preserved() {
        let cli = Cli::try_parse_from([
            "anana",
            "--seed",
            "99",
            "--ticks",
            "20",
            "--mode",
            "replay",
            "--offline",
            "--initial-population",
            "7",
            "--carrying-capacity",
            "70",
            "--mating-interval",
            "12",
        ])
        .expect("the documented arguments parse");
        assert_eq!(cli.seed, 99);
        assert_eq!(cli.ticks, Some(20));
        assert_eq!(cli.mode, RunMode::Replay);
        assert!(cli.offline);
        assert_eq!(
            (
                cli.initial_population,
                cli.carrying_capacity,
                cli.mating_interval
            ),
            (7, 70, 12)
        );
    }

    #[test]
    fn bare_cli_defaults_are_reproducible_and_match_simulation_defaults() {
        let cli = Cli::try_parse_from(["anana"]).expect("bare invocation parses");
        let defaults = anana_sim::Config::default();
        assert_eq!(cli.seed, 42);
        assert_eq!(cli.mode, RunMode::Live);
        assert_eq!(cli.ticks, None);
        assert_eq!(cli.initial_population, defaults.initial_population);
        assert_eq!(cli.carrying_capacity, defaults.carrying_capacity);
        assert_eq!(cli.mating_interval, defaults.mating_interval);
    }

    #[test]
    fn an_unknown_mode_is_rejected_instead_of_guessed() {
        assert!(Cli::try_parse_from(["anana", "--mode", "dream"]).is_err());
    }

    #[tokio::test]
    async fn a_fixed_seed_headless_offline_run_has_no_faults_and_a_stable_hash() {
        let first = run_headless(42, anana_sim::Config::default(), 40, &OfflineMind)
            .await
            .expect("the first headless run completes");
        let second = run_headless(42, anana_sim::Config::default(), 40, &OfflineMind)
            .await
            .expect("the second headless run completes");
        assert!(first.faults.is_empty());
        assert_eq!(first.final_hash, second.final_hash);
        assert_eq!(
            first.final_hash,
            [
                16, 70, 34, 167, 79, 143, 42, 93, 240, 78, 26, 100, 35, 119, 4, 79, 136, 243, 41,
                130, 221, 9, 18, 72, 136, 198, 86, 234, 77, 240, 236, 7,
            ]
        );
    }
}
