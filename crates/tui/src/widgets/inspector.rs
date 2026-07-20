use anana_core::{SkillId, min_awareness};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Gauge, Paragraph, Wrap},
};

use crate::AppState;

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

fn bar(value: u8) -> String {
    let filled = usize::from(value.min(100) / 20);
    format!(
        "{}{}",
        "█".repeat(filled),
        "·".repeat(5_usize.saturating_sub(filled))
    )
}

pub(super) fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let block = Block::bordered().title(" Human inspector ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(human) = state.selected_human() else {
        frame.render_widget(Paragraph::new("No living human selected"), inner);
        return;
    };
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(3)])
        .split(inner);
    let (Some(health_area), Some(details_area)) = (sections.first(), sections.get(1)) else {
        return;
    };
    let ratio = if human.body.max_health == 0 {
        0.0
    } else {
        f64::from(human.body.health) / f64::from(human.body.max_health)
    };
    frame.render_widget(
        Gauge::default()
            .ratio(ratio.clamp(0.0, 1.0))
            .label(format!(
                "health {} / {}",
                human.body.health, human.body.max_health
            ))
            .gauge_style(Style::default().fg(Color::Green)),
        *health_area,
    );
    let infection = human.infection.as_ref().map_or_else(
        || String::from("none"),
        |infection| format!("{:?} strain {}", infection.phase, infection.strain.0),
    );
    let skills = SKILLS
        .iter()
        .map(|skill| {
            let state = human.skills.levels.get(skill);
            let status = if human.consciousness.awareness < min_awareness(*skill) {
                "LOCKED"
            } else if state.is_some_and(|state| state.learned) {
                "learned"
            } else {
                "open"
            };
            format!("{skill:?}: L{} {status}", human.skills.level_of(*skill))
        })
        .collect::<Vec<_>>()
        .join("  ");
    let lines = vec![
        Line::from(format!(
            "Human {}  {:?}  {:?}  age {}  generation {}  fertility {}",
            human.id.0,
            human.phenotype.sex,
            human.body.life_stage,
            human.body.age_ticks,
            human.lineage.generation,
            human.body.fertility,
        )),
        Line::from(format!(
            "infection {infection}  immunities {:?}",
            human.body.immunities
        )),
        Line::from(format!(
            "traits: {:?} eyes, {:?} hand, disease {:?}  robustness {} aptitude {}",
            human.phenotype.eye_color,
            human.phenotype.handedness,
            human.phenotype.disease_x,
            human.phenotype.robustness,
            human.phenotype.aptitude,
        )),
        Line::from(format!(
            "genes: eye {:?}/{:?}, hand {:?}/{:?}, disease {:?}/{:?}, sex {:?}/{:?}; poly {}/{}",
            human.genome.eye.maternal,
            human.genome.eye.paternal,
            human.genome.hand.maternal,
            human.genome.hand.paternal,
            human.genome.disease_x.maternal,
            human.genome.disease_x.paternal,
            human.genome.sex.maternal,
            human.genome.sex.paternal,
            human.genome.robustness.value(),
            human.genome.aptitude.value(),
        )),
        Line::from(format!(
            "instincts: survival {} {}  reproduction {} {}  hunger {} {}  fear {} {}  social {} {}",
            human.instincts.survival,
            bar(human.instincts.survival),
            human.instincts.reproduction,
            bar(human.instincts.reproduction),
            human.instincts.hunger,
            bar(human.instincts.hunger),
            human.instincts.fear,
            bar(human.instincts.fear),
            human.instincts.social,
            bar(human.instincts.social),
        )),
        Line::from(format!(
            "mind: awareness {} focus {} memory capacity {}",
            human.consciousness.awareness,
            human.consciousness.focus,
            human.consciousness.memory_capacity,
        )),
        Line::from(format!("skills: {skills}")),
        Line::from(format!(
            "lineage: mother {:?}, father {:?}, children {:?}",
            human.lineage.mother, human.lineage.father, human.lineage.children
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        *details_area,
    );
}
