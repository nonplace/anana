use anana_core::{
    Bane, Boon, ChanceTemplate, DeterministicKind, EventAuthor, EventOutcome, EventPayload,
    EventRecord, GoshKind, GoshTarget, HumanId,
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{
    AppState,
    app_state::PresentationMoment,
    palette::{DIVINE_AMBER, HISTORICAL, LIVE, STRUCTURE, panel},
};

fn is_birth(record: &EventRecord) -> Option<HumanId> {
    let EventOutcome::Occurred(effects) = &record.outcome else {
        return None;
    };
    effects
        .iter()
        .find_map(|(id, effect)| effect.seeded_genome.is_some().then_some(*id))
}

fn birth_description(child: HumanId, state: &AppState) -> String {
    let lineage = state
        .snapshot
        .humans
        .get(&child)
        .map(|human| &human.lineage)
        .or_else(|| state.snapshot.dead.get(&child).map(|human| &human.lineage));
    match lineage {
        Some(lineage) if lineage.generation == 0 => format!("H{} began a new lineage", child.0),
        Some(lineage) => match (lineage.mother, lineage.father) {
            (Some(mother), Some(father)) => {
                format!("H{} was born to H{} and H{}", child.0, mother.0, father.0)
            }
            _ => format!(
                "H{} was born into generation {}",
                child.0, lineage.generation
            ),
        },
        None => format!("H{} was born", child.0),
    }
}

fn joined_humans(ids: &[HumanId]) -> String {
    match ids {
        [] => String::from("someone"),
        [only] => format!("H{}", only.0),
        [first, second] => format!("H{} and H{}", first.0, second.0),
        _ => {
            let mut names = ids
                .iter()
                .map(|id| format!("H{}", id.0))
                .collect::<Vec<_>>();
            let last = names.pop().unwrap_or_else(|| String::from("someone"));
            format!("{} and {last}", names.join(", "))
        }
    }
}

fn affected_humans(record: &EventRecord) -> Vec<HumanId> {
    match &record.outcome {
        EventOutcome::Occurred(effects) if !effects.is_empty() => effects.keys().copied().collect(),
        EventOutcome::Occurred(_) | EventOutcome::NoOp => record.subjects.clone(),
    }
}

fn infected_human(record: &EventRecord) -> Option<HumanId> {
    let EventOutcome::Occurred(effects) = &record.outcome else {
        return None;
    };
    effects
        .iter()
        .find_map(|(id, effect)| effect.infection.is_some().then_some(*id))
}

fn is_death(record: &EventRecord, state: &AppState) -> Option<HumanId> {
    if !matches!(
        record.payload,
        EventPayload::Deterministic(DeterministicKind::HealthTick)
    ) {
        return None;
    }
    record.subjects.first().copied().filter(|id| {
        state
            .snapshot
            .dead
            .get(id)
            .is_some_and(|dead| dead.death_tick == record.tick)
    })
}

fn gosh_description(kind: &GoshKind, record: &EventRecord) -> String {
    match kind {
        GoshKind::Bless {
            subject,
            boon: Boon::Heal(_),
        } => format!("God healed H{}", subject.0),
        GoshKind::Bless {
            subject,
            boon: Boon::Fertility(_),
        } => format!("God blessed H{} with fertility", subject.0),
        GoshKind::Bless {
            subject,
            boon: Boon::GrantImmunity(virus),
        } => format!("God granted H{} immunity to V{}", subject.0, virus.0),
        GoshKind::Afflict {
            target,
            bane: Bane::Harm(_),
        } => match target {
            GoshTarget::One(human) => format!("God harmed H{}", human.0),
            GoshTarget::Lineage(root) => format!("God harmed H{}'s lineage", root.0),
            GoshTarget::All => String::from("God harmed everyone"),
        },
        GoshKind::Afflict {
            target,
            bane: Bane::Infect(virus),
        } => match target {
            GoshTarget::One(human) => format!("God made H{} ill with V{}", human.0, virus.0),
            GoshTarget::Lineage(root) => {
                format!("God spread V{} through H{}'s lineage", virus.0, root.0)
            }
            GoshTarget::All => format!("God spread V{} through the world", virus.0),
        },
        GoshKind::Teach { subject, skill, .. } => {
            format!("God taught H{} {}", subject.0, super::skill_name(*skill))
        }
        GoshKind::Seed { .. } => is_birth(record).map_or_else(
            || String::from("God seeded a new life"),
            |child| format!("God seeded H{} as a new life", child.0),
        ),
    }
}

fn chance_description(template: ChanceTemplate, record: &EventRecord) -> String {
    if let Some(target) = infected_human(record) {
        let source = record
            .subjects
            .iter()
            .copied()
            .find(|subject| *subject != target);
        return source.map_or_else(
            || format!("H{} fell ill", target.0),
            |source| format!("H{} fell ill after contact with H{}", target.0, source.0),
        );
    }
    let people = joined_humans(&affected_humans(record));
    match template {
        ChanceTemplate::Accident => format!("{people} survived an accident"),
        ChanceTemplate::Discovery => format!("{people} made a discovery"),
        ChanceTemplate::Conflict => format!("{people} faced a conflict"),
        ChanceTemplate::Windfall => format!("{people} found an unexpected windfall"),
    }
}

pub(super) fn is_visible(record: &EventRecord, state: &AppState) -> bool {
    if record.narration.is_some() || record.author == EventAuthor::God {
        return true;
    }
    if is_birth(record).is_some() || is_death(record, state).is_some() {
        return true;
    }
    matches!(
        (&record.payload, &record.outcome),
        (EventPayload::Chance { .. }, EventOutcome::Occurred(effects)) if !effects.is_empty()
    ) || (record.author == EventAuthor::Ai && matches!(record.outcome, EventOutcome::Occurred(_)))
}

pub(super) fn description(record: &EventRecord, state: &AppState) -> String {
    if let Some(narration) = &record.narration {
        return narration.clone();
    }
    if let EventPayload::Gosh(kind) = &record.payload {
        return gosh_description(kind, record);
    }
    if let Some(child) = is_birth(record) {
        return birth_description(child, state);
    }
    if let Some(human) = is_death(record, state) {
        return format!("H{} died", human.0);
    }
    match &record.payload {
        EventPayload::Chance { template, .. } => chance_description(*template, record),
        EventPayload::Gosh(kind) => gosh_description(kind, record),
        EventPayload::Deterministic(_) if record.author == EventAuthor::Ai => {
            format!("AI changed {}", joined_humans(&affected_humans(record)))
        }
        EventPayload::Deterministic(_) => String::from("The world changed"),
    }
}

fn record_line(record: &EventRecord, state: &AppState) -> Line<'static> {
    let author = match record.author {
        EventAuthor::Engine => "WORLD",
        EventAuthor::Ai => "AI",
        EventAuthor::God => "GOD",
    };
    let prefix = format!("t{:>6} s{:>4} {:<6} ", record.tick.0, record.seq.0, author,);
    let divine = record.author == EventAuthor::God;
    let prefix_style = if divine {
        Style::default()
            .fg(DIVINE_AMBER)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(STRUCTURE)
    };
    let detail_style = if divine {
        Style::default().fg(DIVINE_AMBER)
    } else {
        Style::default().fg(HISTORICAL)
    };
    Line::from(vec![
        Span::styled(prefix, prefix_style),
        Span::styled(description(record, state), detail_style),
    ])
}

fn moment_line(moment: &PresentationMoment) -> Line<'static> {
    match moment {
        PresentationMoment::RecallLearned { tick, human } => Line::styled(
            format!(
                "t{:>6}        H{} learned Recall and can now remember",
                tick.0, human.0
            ),
            Style::default()
                .fg(LIVE)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ),
        PresentationMoment::KnowledgeLost {
            tick,
            human,
            skills,
        } => {
            let names = skills
                .iter()
                .map(|skill| super::skill_name(*skill))
                .collect::<Vec<_>>()
                .join(", ");
            Line::styled(
                format!(
                    "t{:>6}        H{} died; {} was lost",
                    tick.0, human.0, names
                ),
                Style::default().fg(LIVE).add_modifier(Modifier::BOLD),
            )
        }
        PresentationMoment::Recovered { tick, human, virus } => Line::styled(
            format!(
                "t{:>6}        H{} recovered from illness V{}",
                tick.0, human.0, virus.0
            ),
            Style::default().fg(HISTORICAL),
        ),
        PresentationMoment::BondFormed {
            tick,
            first,
            second,
        } => Line::styled(
            format!(
                "t{:>6}        H{} and H{} formed a bond",
                tick.0, first.0, second.0
            ),
            Style::default().fg(HISTORICAL),
        ),
    }
}

fn moment_key(moment: &PresentationMoment) -> (u64, u64) {
    let tick = match moment {
        PresentationMoment::RecallLearned { tick, .. }
        | PresentationMoment::KnowledgeLost { tick, .. }
        | PresentationMoment::Recovered { tick, .. }
        | PresentationMoment::BondFormed { tick, .. } => tick.0,
    };
    (tick, u64::MAX)
}

fn moment_names_human(moment: &PresentationMoment, selected: HumanId) -> bool {
    match moment {
        PresentationMoment::RecallLearned { human, .. }
        | PresentationMoment::KnowledgeLost { human, .. }
        | PresentationMoment::Recovered { human, .. } => *human == selected,
        PresentationMoment::BondFormed { first, second, .. } => {
            *first == selected || *second == selected
        }
    }
}

pub(super) fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let mut entries = state
        .visible_events()
        .into_iter()
        .filter(|record| is_visible(record, state))
        .map(|record| {
            (
                (record.tick.0, u64::from(record.seq.0)),
                record_line(record, state),
            )
        })
        .collect::<Vec<_>>();
    entries.extend(
        state
            .moments
            .iter()
            .filter(|moment| {
                !state.feed_selected_only
                    || state
                        .selected
                        .is_some_and(|selected| moment_names_human(moment, selected))
            })
            .map(|moment| (moment_key(moment), moment_line(moment))),
    );
    entries.sort_by_key(|(key, _)| *key);
    let lines = entries
        .into_iter()
        .map(|(_, line)| line)
        .collect::<Vec<_>>();
    let filter = if state.feed_selected_only {
        "SELECTED"
    } else {
        "ALL"
    };
    let block = panel(format!(" EVENTS · {filter} "));
    let visible_height = usize::from(block.inner(area).height.max(1));
    let tail = lines.len().saturating_sub(visible_height);
    let back = usize::from(state.feed_scroll).min(tail);
    let offset = u16::try_from(tail.saturating_sub(back)).unwrap_or(u16::MAX);
    frame.render_widget(Paragraph::new(lines).scroll((offset, 0)).block(block), area);
}
