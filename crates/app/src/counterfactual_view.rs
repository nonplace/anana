use std::collections::BTreeSet;

use anana_sim::{
    Bane, Boon, BranchScopedHumanId, CounterfactualComparison, GoshKind, GoshTarget, HumanId,
    IndividualFate, SkillId,
};
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

fn scoped_ids(values: &[BranchScopedHumanId]) -> String {
    if values.is_empty() {
        return String::from("none");
    }
    values
        .iter()
        .take(EXAMPLE_LIMIT)
        .map(|id| format!("H{}", id.local_id.0))
        .collect::<Vec<_>>()
        .join(", ")
}

fn gosh_target(target: &GoshTarget) -> String {
    match target {
        GoshTarget::One(id) => format!("H{}", id.0),
        GoshTarget::Lineage(id) => format!("H{}'s lineage", id.0),
        GoshTarget::All => String::from("everyone"),
    }
}

fn decree_description(decree: &GoshKind) -> String {
    match decree {
        GoshKind::Bless { subject, boon } => match boon {
            Boon::Heal(amount) => format!("God heals H{} by {amount}", subject.0),
            Boon::Fertility(amount) => {
                format!("God raises H{}'s fertility by {amount}", subject.0)
            }
            Boon::GrantImmunity(virus) => {
                format!("God grants H{} immunity to V{}", subject.0, virus.0)
            }
        },
        GoshKind::Afflict { target, bane } => match bane {
            Bane::Harm(_) => format!("God harms {}", gosh_target(target)),
            Bane::Infect(virus) => {
                format!("God infects {} with V{}", gosh_target(target), virus.0)
            }
        },
        GoshKind::Teach { subject, skill, xp } => {
            format!("God teaches H{} {skill:?} with {xp} experience", subject.0)
        }
        GoshKind::Seed { .. } => String::from("God seeds a new life"),
    }
}

fn earlier_death(difference: &anana_sim::BranchIndividualDifference) -> bool {
    match (&difference.untouched, &difference.decreed) {
        (IndividualFate::Alive { .. }, IndividualFate::Died { .. }) => true,
        (
            IndividualFate::Died {
                death_tick: untouched,
                ..
            },
            IndividualFate::Died {
                death_tick: decreed,
                ..
            },
        ) => decreed < untouched,
        _ => false,
    }
}

fn earlier_death_count(comparison: &CounterfactualComparison) -> u64 {
    comparison
        .differences
        .branch_individuals
        .iter()
        .filter(|difference| earlier_death(difference))
        .count() as u64
}

fn opening_sentence(comparison: &CounterfactualComparison) -> String {
    if comparison.identity.decree.is_none() {
        return String::from("No decree. The two projected futures are identical.");
    }
    if !comparison.differences.worlds_diverged {
        return String::from("One decree. The two projected futures are identical.");
    }

    let mut sentences = vec![String::from("One decree.")];
    if comparison.differences.branch_individuals.len() == 1 {
        if let Some(difference) = comparison.differences.branch_individuals.first() {
            let consequence = match (&difference.untouched, &difference.decreed) {
                (
                    IndividualFate::Died {
                        death_tick: untouched,
                        ..
                    },
                    IndividualFate::Died {
                        death_tick: decreed,
                        ..
                    },
                ) if decreed < untouched => Some(format!(
                    "H{} dies at tick {} instead of tick {}.",
                    difference.human.0, decreed.0, untouched.0
                )),
                (
                    IndividualFate::Alive { .. },
                    IndividualFate::Died {
                        death_tick: decreed,
                        ..
                    },
                ) => Some(format!(
                    "H{} dies at tick {} instead of living to tick {}.",
                    difference.human.0, decreed.0, comparison.identity.horizon.0
                )),
                (
                    IndividualFate::Died {
                        death_tick: untouched,
                        ..
                    },
                    IndividualFate::Alive { .. },
                ) => Some(format!(
                    "H{} lives to tick {} instead of dying at tick {}.",
                    difference.human.0, comparison.identity.horizon.0, untouched.0
                )),
                _ => None,
            };
            if let Some(consequence) = consequence {
                sentences.push(consequence);
            }
        }
    } else if !comparison.differences.branch_individuals.is_empty() {
        sentences.push(format!(
            "{} people alive at the split meet different fates.",
            comparison.differences.branch_individuals.len()
        ));
    }

    let lineages_ended = comparison.differences.lineages.only_untouched.len();
    let births_added = comparison
        .differences
        .post_branch
        .additional_births_in_decreed;
    let more_living = if comparison.differences.population.living_delta < 0 {
        comparison
            .differences
            .population
            .living_delta
            .unsigned_abs()
    } else {
        0
    };
    let consequence = match (lineages_ended, births_added, more_living) {
        (1, births, living) if births > 0 && living > 0 => {
            let root = comparison
                .differences
                .lineages
                .only_untouched
                .first()
                .map_or(String::from("One"), |id| format!("H{}'s", id.0));
            format!(
                "{root} lineage ends, yet the world grows: {births} more lives are born and {living} more people are alive at tick {}.",
                comparison.identity.horizon.0
            )
        }
        (ended, births, living) if ended > 0 && births > 0 && living > 0 => format!(
            "{ended} lineages end, yet the world grows: {births} more lives are born and {living} more people are alive at tick {}.",
            comparison.identity.horizon.0
        ),
        (_, births, living) if births > 0 && living > 0 => format!(
            "The world grows: {births} more lives are born and {living} more people are alive at tick {}.",
            comparison.identity.horizon.0
        ),
        (1, _, _) => String::from("One lineage ends."),
        (ended, _, _) if ended > 1 => format!("{ended} lineages end."),
        _ => String::from("The projected world changes."),
    };
    sentences.push(consequence);
    sentences.join(" ")
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
        format!("{ANSI_LIVE}A n a n A · COUNTERFACTUAL{ANSI_RESET}"),
    );
    match &comparison.identity.decree {
        Some(decree) => push_line(
            &mut output,
            format!("{ANSI_DIVINE}{}{ANSI_RESET}", decree_description(decree)),
        ),
        None => push_line(
            &mut output,
            format!("{ANSI_STRUCTURE}No decree applied.{ANSI_RESET}"),
        ),
    }
    push_line(&mut output, "");
    let story_color = if divine { ANSI_DIVINE } else { ANSI_LIVE };
    push_line(
        &mut output,
        format!("{story_color}{}{ANSI_RESET}", opening_sentence(comparison)),
    );

    let more_living = if comparison.differences.population.living_delta < 0 {
        comparison
            .differences
            .population
            .living_delta
            .unsigned_abs()
    } else {
        0
    };
    let fewer_living = if comparison.differences.population.living_delta > 0 {
        comparison
            .differences
            .population
            .living_delta
            .unsigned_abs()
    } else {
        0
    };
    push_line(&mut output, "");
    push_line(
        &mut output,
        format!("{ANSI_STRUCTURE}WHAT CHANGED{ANSI_RESET}"),
    );
    push_headline(
        &mut output,
        "MORE PEOPLE ALIVE AT HORIZON",
        more_living,
        divine,
    );
    push_headline(
        &mut output,
        "FEWER PEOPLE ALIVE AT HORIZON",
        fewer_living,
        divine,
    );
    push_headline(
        &mut output,
        "MORE LIVES BORN",
        comparison
            .differences
            .post_branch
            .additional_births_in_decreed,
        divine,
    );
    push_headline(
        &mut output,
        "LIVES NEVER BORN",
        comparison.differences.post_branch.never_born_in_decreed,
        divine,
    );
    push_headline(
        &mut output,
        "PEOPLE WHO DIED EARLIER",
        earlier_death_count(comparison),
        divine,
    );
    push_headline(
        &mut output,
        "LINEAGES ENDED",
        comparison.differences.lineages.only_untouched.len() as u64,
        divine,
    );
    push_headline(
        &mut output,
        "LINEAGES SAVED",
        comparison.differences.lineages.only_decreed.len() as u64,
        divine,
    );
    push_headline(
        &mut output,
        "KNOWLEDGE LOST",
        comparison.differences.knowledge.only_untouched.len() as u64,
        divine,
    );
    push_headline(
        &mut output,
        "KNOWLEDGE SAVED",
        comparison.differences.knowledge.only_decreed.len() as u64,
        divine,
    );

    push_line(&mut output, "");
    push_line(
        &mut output,
        format!("{ANSI_STRUCTURE}AT THE HORIZON{ANSI_RESET}"),
    );
    push_columns(&mut output, "UNTOUCHED", "DECREED", divine);
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
        format!("{ANSI_STRUCTURE}AFTER THE BRANCH · TOTALS ONLY{ANSI_RESET}"),
    );
    push_line(
        &mut output,
        format!(
            "{ANSI_STRUCTURE}People born after the split are compared as totals, never person to person.{ANSI_RESET}"
        ),
    );
    push_columns(
        &mut output,
        &format!(
            "lineages present only here: {}",
            ids(&comparison.differences.lineages.only_untouched)
        ),
        &format!(
            "lineages present only here: {}",
            ids(&comparison.differences.lineages.only_decreed)
        ),
        divine,
    );
    push_columns(
        &mut output,
        &format!(
            "lives born only here: {}",
            scoped_ids(&comparison.differences.post_branch.untouched_examples)
        ),
        &format!(
            "lives born only here: {}",
            scoped_ids(&comparison.differences.post_branch.decreed_examples)
        ),
        divine,
    );
    push_columns(
        &mut output,
        &format!(
            "knowledge present only here: {}",
            skills(&comparison.differences.knowledge.only_untouched)
        ),
        &format!(
            "knowledge present only here: {}",
            skills(&comparison.differences.knowledge.only_decreed)
        ),
        divine,
    );

    push_line(&mut output, "");
    push_line(
        &mut output,
        format!("{ANSI_STRUCTURE}VERIFICATION{ANSI_RESET}"),
    );
    push_line(
        &mut output,
        format!(
            "{ANSI_STRUCTURE}seed {} · branch tick {} · horizon tick {}{ANSI_RESET}",
            comparison.identity.seed,
            comparison.identity.branch_at.0,
            comparison.identity.horizon.0
        ),
    );
    push_line(
        &mut output,
        format!(
            "{ANSI_STRUCTURE}branch world hash {}{}{ANSI_RESET}",
            hash_half(comparison.identity.branch_world_hash, 0),
            hash_half(comparison.identity.branch_world_hash, 16)
        ),
    );
    push_columns(&mut output, "UNTOUCHED", "DECREED", divine);
    push_columns(
        &mut output,
        &format!(
            "world hash {}",
            hash_half(comparison.untouched.world_hash, 0)
        ),
        &format!("world hash {}", hash_half(comparison.decreed.world_hash, 0)),
        divine,
    );
    push_columns(
        &mut output,
        &format!(
            "           {}",
            hash_half(comparison.untouched.world_hash, 16)
        ),
        &format!(
            "           {}",
            hash_half(comparison.decreed.world_hash, 16)
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
        Bane, BranchIndividualDifference, BranchScopedHumanId, BranchSide, Config,
        CounterfactualRequest, GoshKind, GoshTarget, HumanId, IndividualFate, run_counterfactual,
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
        assert!(output.contains("PEOPLE WHO DIED EARLIER"));
        assert!(output.contains("LIVES NEVER BORN"));
        assert!(output.contains("MORE LIVES BORN"));
        assert!(output.contains("LINEAGES ENDED"));
        assert!(output.contains("LINEAGES SAVED"));
        assert!(output.contains("KNOWLEDGE LOST"));
        assert!(output.contains("KNOWLEDGE SAVED"));
        assert!(output.contains("God harms H1"));
        assert!(output.contains("This compares two runs of a model"));
    }

    fn comparison_with_growth_after_an_early_death() -> CounterfactualComparison {
        let mut comparison = comparison(None);
        comparison.identity.branch_at.0 = 300;
        comparison.identity.horizon.0 = 2_500;
        comparison.identity.decree = Some(GoshKind::Afflict {
            target: GoshTarget::One(HumanId(7)),
            bane: Bane::Harm(u16::MAX),
        });
        comparison.differences.worlds_diverged = true;
        comparison.differences.population.living_delta = -5;
        comparison.differences.population.births_delta = -5;
        let mut untouched_death_tick = comparison.identity.branch_at;
        untouched_death_tick.0 = 1_240;
        let mut decreed_death_tick = comparison.identity.branch_at;
        decreed_death_tick.0 = 301;
        comparison.differences.branch_individuals = vec![BranchIndividualDifference {
            human: HumanId(7),
            untouched: IndividualFate::Died {
                death_tick: untouched_death_tick,
                age_ticks: 1_254,
            },
            decreed: IndividualFate::Died {
                death_tick: decreed_death_tick,
                age_ticks: 315,
            },
        }];
        comparison
            .differences
            .lineages
            .only_untouched
            .insert(HumanId(7));
        comparison
            .differences
            .post_branch
            .additional_births_in_decreed = 5;
        comparison.differences.post_branch.birth_count_delta = -5;
        comparison.differences.post_branch.decreed_examples = (125..130)
            .map(|id| BranchScopedHumanId {
                branch: BranchSide::Decreed,
                local_id: HumanId(id),
            })
            .collect();
        comparison.decreed.population.living =
            comparison.untouched.population.living.saturating_add(5);
        comparison.decreed.population.births_after_branch = comparison
            .untouched
            .population
            .births_after_branch
            .saturating_add(5);
        comparison
    }

    #[test]
    fn the_opening_sentence_names_the_early_death_ended_lineage_and_lives_gained() {
        let output = format_counterfactual(&comparison_with_growth_after_an_early_death());
        assert!(output.contains(
            "One decree. H7 dies at tick 301 instead of tick 1240. H7's lineage ends, yet the world grows: 5 more lives are born and 5 more people are alive at tick 2500."
        ));
        let story = output
            .find("One decree.")
            .expect("the human story is present");
        let columns = output
            .find("UNTOUCHED")
            .expect("the detailed comparison is present");
        assert!(story < columns);
    }

    #[test]
    fn added_lives_make_the_headline_nonzero_even_when_every_loss_is_zero() {
        let mut comparison = comparison_with_growth_after_an_early_death();
        comparison.differences.branch_individuals.clear();
        comparison.differences.lineages.only_untouched.clear();
        let output = format_counterfactual(&comparison);
        let headline_start = output
            .find("WHAT CHANGED")
            .expect("the headline block is present");
        let headline_end = output
            .find("AT THE HORIZON")
            .expect("the horizon comparison follows it");
        let headline = &output[headline_start..headline_end];
        assert!(headline.contains("MORE LIVES BORN"));
        assert!(headline.lines().any(|line| {
            line.contains("MORE LIVES BORN")
                && line
                    .split_whitespace()
                    .last()
                    .is_some_and(|value| value.contains('5'))
        }));
    }

    #[test]
    fn verification_hashes_follow_the_human_consequences_instead_of_leading_them() {
        let output = format_counterfactual(&comparison_with_growth_after_an_early_death());
        let story = output.find("One decree.").expect("the story is present");
        let verification = output
            .find("VERIFICATION")
            .expect("the verification block is present");
        let branch_hash = output
            .find("branch world hash")
            .expect("the branch hash is retained");
        assert!(story < verification);
        assert!(verification < branch_hash);
    }

    #[test]
    fn branch_scoped_examples_are_explained_by_their_columns_not_raw_internal_prefixes() {
        let output = format_counterfactual(&comparison_with_growth_after_an_early_death());
        assert!(output.contains("H125, H126, H127, H128, H129"));
        assert!(!output.contains("decreed:H125"));
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
