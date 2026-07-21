mod cli;
mod driver;
mod terminal;

use anana_mind::{AnyMind, GptMind, OfflineMind};
use anyhow::{Context, Result};
use clap::Parser;

use cli::{Cli, Command, CounterfactualArgs, RunMode};
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

fn run_counterfactual_command(args: CounterfactualArgs) -> Result<()> {
    let decree = serde_json::from_str::<Option<anana_sim::GoshKind>>(&args.gosh)
        .context("--gosh must be the canonical JSON form of a gosh, or null")?;
    if !args.json {
        eprintln!(
            "Running seed {} to branch tick {}, then projecting both futures to tick {}...",
            args.seed, args.branch_at, args.horizon
        );
    }
    let comparison = anana_sim::run_counterfactual(anana_sim::CounterfactualRequest {
        seed: args.seed,
        config: args.simulation_config(),
        branch_at: args.branch_at,
        horizon: args.horizon,
        decree,
    })?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&comparison)?);
    } else {
        println!("{comparison}");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(Command::Counterfactual(args)) = cli.command.clone() {
        return run_counterfactual_command(args);
    }
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
        assert!(cli.command.is_none());
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
        assert!(cli.command.is_none());
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

    #[test]
    fn the_counterfactual_subcommand_accepts_the_canonical_gosh_object() {
        let cli = Cli::try_parse_from([
            "anana",
            "counterfactual",
            "--seed",
            "42",
            "--branch-at",
            "40",
            "--horizon",
            "120",
            "--gosh",
            r#"{"Afflict":{"target":{"One":12},"bane":{"Harm":65535}}}"#,
            "--json",
        ])
        .expect("the documented counterfactual invocation parses");
        let Some(Command::Counterfactual(args)) = cli.command else {
            panic!("the counterfactual command is retained");
        };
        assert_eq!((args.seed, args.branch_at, args.horizon), (42, 40, 120));
        assert!(args.json);
        assert_eq!(
            serde_json::from_str::<Option<anana_sim::GoshKind>>(&args.gosh)
                .expect("the canonical gosh parses"),
            Some(anana_sim::GoshKind::Afflict {
                target: anana_sim::GoshTarget::One(anana_sim::HumanId(12)),
                bane: anana_sim::Bane::Harm(u16::MAX),
            })
        );
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
                138, 249, 107, 156, 217, 175, 154, 121, 212, 78, 212, 121, 118, 234, 131, 111, 55,
                194, 32, 133, 247, 217, 14, 163, 181, 168, 200, 180, 110, 140, 198, 172,
            ]
        );
    }
}
