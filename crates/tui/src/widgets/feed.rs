use anana_core::{DeterministicKind, EventAuthor, EventOutcome, EventPayload, HumanId, SkillId};
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

fn is_birth(record: &anana_core::EventRecord) -> Option<HumanId> {
    let EventOutcome::Occurred(effects) = &record.outcome else {
        return None;
    };
    effects
        .iter()
        .find_map(|(id, effect)| effect.seeded_genome.is_some().then_some(*id))
}

fn is_recall_moment(record: &anana_core::EventRecord, state: &AppState) -> Option<HumanId> {
    let EventOutcome::Occurred(effects) = &record.outcome else {
        return None;
    };
    effects.iter().find_map(|(id, effect)| {
        let grants_recall = effect.skill_xp.contains_key(&SkillId::Recall);
        let recall_is_online = state
            .snapshot
            .humans
            .get(id)
            .is_some_and(|human| human.skills.recall_learned());
        (grants_recall && recall_is_online).then_some(*id)
    })
}

fn birth_description(child: HumanId, state: &AppState) -> String {
    let lineage = state
        .snapshot
        .humans
        .get(&child)
        .map(|human| &human.lineage)
        .or_else(|| state.snapshot.dead.get(&child).map(|human| &human.lineage));
    match lineage {
        Some(lineage) if lineage.generation == 0 => {
            format!("BIRTH — H{} BEGINS A NEW LINEAGE", child.0)
        }
        Some(lineage) => format!(
            "BIRTH — H{} CONTINUES GENERATION {}",
            child.0, lineage.generation
        ),
        None => format!("BIRTH — H{}", child.0),
    }
}

pub(super) fn description(record: &anana_core::EventRecord, state: &AppState) -> String {
    if let Some(narration) = &record.narration {
        return narration.clone();
    }
    if let Some(human) = is_recall_moment(record, state) {
        return format!("RECALL ONLINE — H{} BEGINS A HISTORY", human.0);
    }
    if let Some(child) = is_birth(record) {
        return birth_description(child, state);
    }
    if matches!(
        record.payload,
        EventPayload::Deterministic(DeterministicKind::HealthTick)
    ) && record
        .subjects
        .first()
        .is_some_and(|id| state.snapshot.dead.contains_key(id))
    {
        return record
            .subjects
            .first()
            .map_or_else(|| String::from("DEATH"), |id| format!("DEATH — H{}", id.0));
    }
    match &record.payload {
        EventPayload::Chance { template, .. } => format!("{template:?}"),
        EventPayload::Deterministic(kind) => format!("{kind:?}"),
        EventPayload::Gosh(kind) => format!("{kind:?}"),
    }
}

fn record_line(record: &anana_core::EventRecord, state: &AppState) -> Line<'static> {
    let prefix = format!(
        "t{:>6} s{:>4} {:<6} ",
        record.tick.0,
        record.seq.0,
        format!("{:?}", record.author)
    );
    let divine = record.author == EventAuthor::God;
    let recall = is_recall_moment(record, state).is_some();
    let prefix_style = if divine {
        Style::default()
            .fg(DIVINE_AMBER)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(STRUCTURE)
    };
    let detail_style = if divine {
        Style::default().fg(DIVINE_AMBER)
    } else if recall {
        Style::default()
            .fg(LIVE)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
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
                "t{:>6}        RECALL ONLINE — H{} BEGINS A HISTORY",
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
                .map(|skill| format!("{skill:?}"))
                .collect::<Vec<_>>()
                .join(", ");
            Line::styled(
                format!(
                    "t{:>6}        KNOWLEDGE LOST — {} DIED WITH H{}",
                    tick.0, names, human.0
                ),
                Style::default().fg(LIVE).add_modifier(Modifier::BOLD),
            )
        }
    }
}

fn moment_key(moment: &PresentationMoment) -> (u64, u64) {
    let tick = match moment {
        PresentationMoment::RecallLearned { tick, .. }
        | PresentationMoment::KnowledgeLost { tick, .. } => tick.0,
    };
    (tick, u64::MAX)
}

fn moment_human(moment: &PresentationMoment) -> HumanId {
    match moment {
        PresentationMoment::RecallLearned { human, .. }
        | PresentationMoment::KnowledgeLost { human, .. } => *human,
    }
}

pub(super) fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let mut entries = state
        .visible_events()
        .into_iter()
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
                        .is_some_and(|selected| selected == moment_human(moment))
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
    frame.render_widget(
        Paragraph::new(lines)
            .scroll((state.feed_scroll, 0))
            .block(panel(format!(" EVENTS · {filter} "))),
        area,
    );
}
