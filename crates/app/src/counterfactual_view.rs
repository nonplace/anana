use std::collections::BTreeSet;

use anana_sim::{BranchScopedHumanId, CounterfactualComparison, HumanId, IndividualFate, SkillId};
use anana_tui::{ANSI_DIVINE, ANSI_LIVE, ANSI_RESET, ANSI_STRUCTURE};

const COLUMN_WIDTH: usize = 48;
const EXAMPLE_LIMIT: usize = 5;

fn hash_half(hash: [u8; 32], skip: usize) -> String {
    hash.iter()
        .skip(skip)
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn fate(value: &IndividualFate) -> String {
    match value {
        IndividualFate::Alive { age_ticks } => format!("alive · age {age_ticks} ticks"),
        IndividualFate::Died {
            death_tick,
            age_ticks,
        } => format!("died t{} · age {} ticks", death_tick.0, age_ticks),
    }
}

fn ids(values: &BTreeSet<HumanId>) -> String {
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

fn skills(values: &BTreeSet<SkillId>) -> String {
    if values.is_empty() {
        return String::from("none");
    }
    values
        .iter()
        .take(EXAMPLE_LIMIT)
        .map(|skill| format!("{skill:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn scoped(values: &[BranchScopedHumanId]) -> String {
    if values.is_empty() {
        return String::from("none");
    }
    values
        .iter()
        .take(EXAMPLE_LIMIT)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn push_line(output: &mut String, line: impl AsRef<str>) {
    output.push_str(line.as_ref());
    output.push('\n');
}

fn push_columns(output: &mut String, left: &str, right: &str, divine: bool) {
    let right_color = if divine { ANSI_DIVINE } else { ANSI_LIVE };
    push_line(
        output,
        format!(
            "{ANSI_LIVE}{left:<COLUMN_WIDTH$}{ANSI_RESET} {ANSI_STRUCTURE}│{ANSI_RESET} {right_color}{right}{ANSI_RESET}"
        ),
    );
}

fn push_headline(output: &mut String, label: &str, value: u64, divine: bool) {
    let color = if divine { ANSI_DIVINE } else { ANSI_LIVE };
    push_line(output, format!("{color}{label:<32} {value:>6}{ANSI_RESET}"));
}

pub(crate) fn format_counterfactual(comparison: &CounterfactualComparison) -> String {
    let divine = comparison.identity.decree.is_some();
    let mut output = String::new();
    push_line(
        &mut output,
        format!(
            "{ANSI_LIVE}A n a n A · COUNTERFACTUAL{ANSI_RESET}\n{ANSI_STRUCTURE}seed {} · branch t{} · horizon t{} · branch hash {}{}{ANSI_RESET}",
            comparison.identity.seed,
            comparison.identity.branch_at.0,
            comparison.identity.horizon.0,
            hash_half(comparison.identity.branch_world_hash, 0),
            hash_half(comparison.identity.branch_world_hash, 16),
        ),
    );
    match &comparison.identity.decree {
        Some(decree) => push_line(
            &mut output,
            format!("{ANSI_DIVINE}decree · {decree:?}{ANSI_RESET}"),
        ),
        None => push_line(
            &mut output,
            format!("{ANSI_STRUCTURE}decree · none{ANSI_RESET}"),
        ),
    }
    push_line(&mut output, "");
    push_columns(&mut output, "UNTOUCHED", "DECREED", divine);
    push_columns(
        &mut output,
        &format!("hash {}", hash_half(comparison.untouched.world_hash, 0)),
        &format!("hash {}", hash_half(comparison.decreed.world_hash, 0)),
        divine,
    );
    push_columns(
        &mut output,
        &format!("     {}", hash_half(comparison.untouched.world_hash, 16)),
        &format!("     {}", hash_half(comparison.decreed.world_hash, 16)),
        divine,
    );
    push_columns(
        &mut output,
        &format!(
            "living              {:>6}",
            comparison.untouched.population.living
        ),
        &format!(
            "living              {:>6}",
            comparison.decreed.population.living
        ),
        divine,
    );
    push_columns(
        &mut output,
        &format!(
            "births after branch  {:>6}",
            comparison.untouched.population.births_after_branch
        ),
        &format!(
            "births after branch  {:>6}",
            comparison.decreed.population.births_after_branch
        ),
        divine,
    );
    push_columns(
        &mut output,
        &format!(
            "deaths after branch  {:>6}",
            comparison.untouched.population.deaths_after_branch
        ),
        &format!(
            "deaths after branch  {:>6}",
            comparison.decreed.population.deaths_after_branch
        ),
        divine,
    );
    push_columns(
        &mut output,
        &format!(
            "surviving lineages  {:>6}",
            comparison.untouched.surviving_lineages
        ),
        &format!(
            "surviving lineages  {:>6}",
            comparison.decreed.surviving_lineages
        ),
        divine,
    );
    push_columns(
        &mut output,
        &format!(
            "knowledge held      {:>6}",
            comparison.untouched.living_knowledge.len()
        ),
        &format!(
            "knowledge held      {:>6}",
            comparison.decreed.living_knowledge.len()
        ),
        divine,
    );

    let died_who_lived = u64::try_from(
        comparison
            .differences
            .branch_individuals
            .iter()
            .filter(|difference| {
                matches!(difference.untouched, IndividualFate::Alive { .. })
                    && matches!(difference.decreed, IndividualFate::Died { .. })
            })
            .count(),
    )
    .map_or(u64::MAX, |value| value);
    push_line(&mut output, "");
    push_line(
        &mut output,
        format!("{ANSI_STRUCTURE}WHAT THE DECREE CHANGED{ANSI_RESET}"),
    );
    push_headline(
        &mut output,
        "DIED WHO OTHERWISE LIVED",
        died_who_lived,
        divine,
    );
    push_headline(
        &mut output,
        "NEVER BORN",
        comparison.differences.post_branch.never_born_in_decreed,
        divine,
    );
    push_headline(
        &mut output,
        "LINEAGES ENDED",
        u64::try_from(comparison.differences.lineages.only_untouched.len())
            .map_or(u64::MAX, |value| value),
        divine,
    );
    push_headline(
        &mut output,
        "KNOWLEDGE LOST",
        u64::try_from(comparison.differences.knowledge.only_untouched.len())
            .map_or(u64::MAX, |value| value),
        divine,
    );

    push_line(&mut output, "");
    push_line(
        &mut output,
        format!("{ANSI_STRUCTURE}PEOPLE ALIVE AT THE BRANCH{ANSI_RESET}"),
    );
    if comparison.differences.branch_individuals.is_empty() {
        push_columns(&mut output, "no changed fates", "no changed fates", divine);
    } else {
        for difference in comparison
            .differences
            .branch_individuals
            .iter()
            .take(EXAMPLE_LIMIT)
        {
            push_columns(
                &mut output,
                &format!("H{} · {}", difference.human.0, fate(&difference.untouched)),
                &format!("H{} · {}", difference.human.0, fate(&difference.decreed)),
                divine,
            );
        }
    }

    push_line(&mut output, "");
    push_line(
        &mut output,
        format!("{ANSI_STRUCTURE}AFTER THE BRANCH · AGGREGATES ONLY{ANSI_RESET}"),
    );
    push_columns(
        &mut output,
        &format!(
            "lineages surviving here: {}",
            ids(&comparison.differences.lineages.only_untouched)
        ),
        "the same lineages are extinct",
        divine,
    );
    push_columns(
        &mut output,
        &format!(
            "lives unique here: {}",
            scoped(&comparison.differences.post_branch.untouched_examples)
        ),
        &format!(
            "additional lives here: {}",
            scoped(&comparison.differences.post_branch.decreed_examples)
        ),
        divine,
    );
    push_columns(
        &mut output,
        &format!(
            "knowledge surviving here: {}",
            skills(&comparison.differences.knowledge.only_untouched)
        ),
        &format!(
            "knowledge unique here: {}",
            skills(&comparison.differences.knowledge.only_decreed)
        ),
        divine,
    );
    push_line(&mut output, "");
    push_line(
        &mut output,
        format!(
            "{ANSI_STRUCTURE}This compares two runs of a model; it is not a claim about real people.{ANSI_RESET}"
        ),
    );
    output
}

#[cfg(test)]
mod tests {
    //! The human-readable counterfactual makes divine consequences scannable without changing its structured data.

    use anana_sim::{
        Bane, Config, CounterfactualRequest, GoshKind, GoshTarget, HumanId, run_counterfactual,
    };

    use super::*;

    fn comparison(decree: Option<GoshKind>) -> CounterfactualComparison {
        run_counterfactual(CounterfactualRequest {
            seed: 42,
            config: Config {
                initial_population: 8,
                carrying_capacity: 24,
                ..Config::default()
            },
            branch_at: 1,
            horizon: 8,
            decree,
        })
        .expect("the visual fixture projects both futures")
    }

    #[test]
    fn the_human_counterfactual_aligns_untouched_and_decreed_futures_in_two_columns() {
        let output = format_counterfactual(&comparison(Some(GoshKind::Afflict {
            target: GoshTarget::One(HumanId(1)),
            bane: Bane::Harm(u16::MAX),
        })));
        assert!(
            output
                .lines()
                .any(|line| line.contains("UNTOUCHED") && line.contains("DECREED"))
        );
        assert!(output.contains("DIED WHO OTHERWISE LIVED"));
        assert!(output.contains("NEVER BORN"));
        assert!(output.contains("LINEAGES ENDED"));
        assert!(output.contains("KNOWLEDGE LOST"));
        assert!(output.contains("Afflict"));
        assert!(output.contains("This compares two runs of a model"));
    }

    #[test]
    fn amber_appears_in_counterfactual_text_only_when_a_divine_cause_exists() {
        let no_decree = format_counterfactual(&comparison(None));
        let decree = format_counterfactual(&comparison(Some(GoshKind::Afflict {
            target: GoshTarget::One(HumanId(1)),
            bane: Bane::Harm(u16::MAX),
        })));
        assert!(!no_decree.contains(anana_tui::ANSI_DIVINE));
        assert!(decree.contains(anana_tui::ANSI_DIVINE));
    }
}
