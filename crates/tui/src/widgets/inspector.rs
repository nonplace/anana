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
            format!(
                "{} {:>1} {marker}",
                super::skill_name(*skill),
                human.skills.level_of(*skill)
            )
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

fn models_seen(human: &HumanState) -> String {
    let teachers = human
        .social_bonds
        .observed_competence
        .keys()
        .take(5)
        .map(|id| format!("H{}", id.0))
        .collect::<Vec<_>>();
    if teachers.is_empty() {
        String::from("No teacher or model has been recorded")
    } else {
        format!("Learned around {}", teachers.join(", "))
    }
}

fn score_bar(score: u16) -> String {
    let score = score.min(100);
    let filled = usize::from(score / 10);
    format!(
        "{}{}",
        "█".repeat(filled),
        "·".repeat(10_usize.saturating_sub(filled))
    )
}

fn perceptual_score(value: u16) -> u16 {
    value.clamp(500, 1_500).saturating_sub(500) / 10
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
        .take(3)
        .map(|(id, strength)| {
            let score = strength.0.min(1_000) / 10;
            format!("H{} {} {:>3}/100", id.0, score_bar(score), score)
        })
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
        .filter(|record| super::feed::is_visible(record, state))
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
        || String::from("healthy"),
        |infection| {
            let phase = match infection.phase {
                anana_core::InfectionPhase::Incubating => "incubating",
                anana_core::InfectionPhase::Infectious => "infectious",
                anana_core::InfectionPhase::Recovered => "recovered",
            };
            format!("{phase} · virus V{}", infection.strain.0)
        },
    );
    let memory = if human.skills.recall_learned() {
        "Recall online — this life can accumulate a history."
    } else {
        "Recall not learned — experience cannot become a history."
    };
    let parent = |id: Option<anana_core::HumanId>| {
        id.map_or_else(|| String::from("—"), |id| format!("H{}", id.0))
    };
    let parents = format!(
        "parents {}/{} · children {:>2} · home R{}",
        parent(human.lineage.mother),
        parent(human.lineage.father),
        human.lineage.children.len(),
        human.residence.id.0,
    );
    let sex = match human.phenotype.sex {
        anana_core::Sex::Female => "female",
        anana_core::Sex::Male => "male",
    };
    let stage = match human.body.life_stage {
        anana_core::LifeStage::Infant => "infant",
        anana_core::LifeStage::Child => "child",
        anana_core::LifeStage::Adolescent => "adolescent",
        anana_core::LifeStage::Adult => "adult",
        anana_core::LifeStage::Elder => "elder",
    };
    let mut lines = vec![
        Line::styled(
            format!(
                "H{} · {sex} · {stage} · age {:>5} · generation {:>2}",
                human.id.0, human.body.age_ticks, human.lineage.generation,
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
                human.body.fertility.min(100),
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
        labelled("MODELS SEEN", models_seen(human)),
        labelled("ATTACHMENTS", attachments(human)),
        labelled("ORIGIN", parents),
        labelled(
            "DISPOSITION",
            format!(
                "survival {:>3}/100 · social {:>3}/100 · fear {:>3}/100",
                human.instincts.survival.min(100),
                human.instincts.social.min(100),
                human.instincts.fear.min(100),
            ),
        ),
        labelled(
            "PERCEPTION",
            format!(
                "threat {:>3}/100 · novelty {:>3}/100",
                perceptual_score(human.phenotype.threat_salience.value()),
                perceptual_score(human.phenotype.novelty_tolerance.value()),
            ),
        ),
        labelled(
            "MIND",
            format!(
                "awareness {:>3}/100 · focus {:>3}/100 · memory {:>4}/1000",
                human.consciousness.awareness.min(100),
                human.consciousness.focus.min(100),
                human.consciousness.memory_capacity.min(1_000),
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
