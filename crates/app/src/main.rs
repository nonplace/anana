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
        RunMode::Replay => run_replay(cli.seed, config, cli.ticks.unwrap_or(100)),
        RunMode::Headless => {
            let result = run_headless(
                cli.seed,
                config,
                cli.ticks.unwrap_or(100),
                &select_mind(cli.offline),
            )
            .await?;
            println!(
                "hash={} tick={} living={} births={} deaths={} infections={} faults={}",
                hash_hex(result.final_hash),
                result.tick,
                result.stats.living,
                result.stats.births,
                result.stats.deaths,
                result.stats.infections,
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
            "--max-population",
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
                cli.max_population,
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
        assert_eq!(cli.max_population, defaults.max_population);
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
                30, 151, 146, 226, 53, 9, 6, 208, 193, 124, 122, 39, 71, 143, 96, 79, 16, 9, 198,
                151, 171, 10, 174, 250, 184, 206, 178, 73, 150, 137, 118, 126,
            ]
        );
    }
}
