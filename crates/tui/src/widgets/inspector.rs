use anana_core::{HumanState, SkillId, min_awareness};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

use crate::{
    AppState,
    palette::{DIVINE_AMBER, HISTORICAL, LIVE, STRUCTURE, panel},
};

const SKILLS: [SkillId; 9] = [
    SkillId::Recall,
    SkillId::Motor,
    SkillId::Language,
    SkillId::Foraging,
    SkillId::ToolUse,
    SkillId::SocialBond,
    SkillId::Farming,
    SkillId::Medicine,
    SkillId::Planning,
];

fn labelled(label: &'static str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<12}"), Style::default().fg(STRUCTURE)),
        Span::styled(value, Style::default().fg(HISTORICAL)),
    ])
}

fn health_bar(human: &HumanState) -> String {
    let filled = if human.body.max_health == 0 {
        0
    } else {
        usize::from(
            human
                .body
                .health
                .saturating_mul(10)
                .saturating_div(human.body.max_health)
                .min(10),
        )
    };
    format!(
        "{}{}",
        "█".repeat(filled),
        "·".repeat(10_usize.saturating_sub(filled))
    )
}

fn knowledge(human: &HumanState) -> String {
    SKILLS
        .iter()
        .map(|skill| {
            let state = human.skills.levels.get(skill);
            let marker = if human.consciousness.awareness < min_awareness(*skill) {
                "locked"
            } else if state.is_some_and(|state| state.learned) {
                "learned"
            } else {
                "open"
            };
            format!("{skill:?} L{} {marker}", human.skills.level_of(*skill))
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

fn learned_among(human: &HumanState) -> String {
    let teachers = human
        .social_bonds
        .observed_competence
        .keys()
        .take(5)
        .map(|id| format!("H{}", id.0))
        .collect::<Vec<_>>();
    if teachers.is_empty() {
        String::from("Learned among: no observed model recorded")
    } else {
        format!("Learned among: {}", teachers.join(", "))
    }
}

fn attachments(human: &HumanState) -> String {
    let mut bonds = human
        .social_bonds
        .bonds
        .iter()
        .map(|(id, bond)| (*id, bond.strength))
        .collect::<Vec<_>>();
    bonds.sort_by_key(|(id, strength)| (std::cmp::Reverse(*strength), *id));
    if bonds.is_empty() {
        return String::from("none yet");
    }
    bonds
        .into_iter()
        .take(5)
        .map(|(id, strength)| format!("H{} {:>4}‰", id.0, strength.0))
        .collect::<Vec<_>>()
        .join(" · ")
}

fn life_events(state: &AppState, human: &HumanState) -> Vec<Line<'static>> {
    if !human.skills.recall_learned() {
        return vec![Line::styled(
            "No history yet — Recall has not been learned.",
            Style::default().fg(HISTORICAL),
        )];
    }
    let mut records = state
        .snapshot
        .event_log
        .iter()
        .filter(|record| record.subjects.contains(&human.id))
        .rev()
        .take(3)
        .collect::<Vec<_>>();
    records.reverse();
    if records.is_empty() {
        return vec![Line::styled(
            "Recall is online; nothing has reached the record yet.",
            Style::default().fg(HISTORICAL),
        )];
    }
    records
        .into_iter()
        .map(|record| {
            Line::styled(
                format!(
                    "t{:>6}  {}",
                    record.tick.0,
                    super::feed::description(record, state)
                ),
                Style::default().fg(HISTORICAL),
            )
        })
        .collect()
}

pub(super) fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let block = panel(" LIFE ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(human) = state.selected_human() else {
        frame.render_widget(Paragraph::new("No living human selected"), inner);
        return;
    };

    let infection = human.infection.as_ref().map_or_else(
        || String::from("clear"),
        |infection| format!("{:?} · strain {}", infection.phase, infection.strain.0),
    );
    let memory = if human.skills.recall_learned() {
        "Recall online — this life can accumulate a history."
    } else {
        "Recall not learned — experience cannot become a history."
    };
    let parents = format!(
        "parents {:?}/{:?} · children {:>2} · residence {}",
        human.lineage.mother,
        human.lineage.father,
        human.lineage.children.len(),
        human.residence.id.0,
    );
    let mut lines = vec![
        Line::styled(
            format!(
                "H{} · {:?} · {:?} · age {:>5} · generation {:>2}",
                human.id.0,
                human.phenotype.sex,
                human.body.life_stage,
                human.body.age_ticks,
                human.lineage.generation,
            ),
            Style::default().fg(LIVE).add_modifier(Modifier::BOLD),
        ),
        labelled(
            "BODY",
            format!(
                "{} {:>3}/{:<3} · fertility {:>3} · {infection}",
                health_bar(human),
                human.body.health,
                human.body.max_health,
                human.body.fertility,
            ),
        ),
    ];
    if state.is_divinely_touched(human.id) {
        lines.push(Line::styled(
            "DIVINE TOUCH · this life was changed by a decree this tick",
            Style::default()
                .fg(DIVINE_AMBER)
                .add_modifier(Modifier::BOLD),
        ));
    }
    lines.extend([
        labelled("MEMORY", String::from(memory)),
        labelled("KNOWLEDGE", knowledge(human)),
        labelled("", learned_among(human)),
        labelled("ATTACHMENTS", attachments(human)),
        labelled("ORIGIN", parents),
        labelled(
            "DISPOSITION",
            format!(
                "survival {:>3} · social {:>3} · fear {:>3} · novelty {:>4}‰",
                human.instincts.survival,
                human.instincts.social,
                human.instincts.fear,
                human.phenotype.novelty_tolerance.value(),
            ),
        ),
        labelled(
            "MIND",
            format!(
                "awareness {:>3} · focus {:>3} · memory {:>4}",
                human.consciousness.awareness,
                human.consciousness.focus,
                human.consciousness.memory_capacity,
            ),
        ),
        Line::styled(
            "LIFE EVENTS",
            Style::default().fg(LIVE).add_modifier(Modifier::BOLD),
        ),
    ]);
    lines.extend(life_events(state, human));
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
