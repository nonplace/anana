use anana_core::{InfectionPhase, LifeStage};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use crate::AppState;

fn stage_color(stage: LifeStage) -> Color {
    match stage {
        LifeStage::Infant => Color::LightMagenta,
        LifeStage::Child => Color::LightBlue,
        LifeStage::Adolescent => Color::LightGreen,
        LifeStage::Adult => Color::Yellow,
        LifeStage::Elder => Color::White,
    }
}

pub(super) fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let entries = state
        .snapshot
        .humans
        .values()
        .map(|human| {
            let infection = match human.infection.as_ref().map(|infection| infection.phase) {
                Some(InfectionPhase::Incubating) => "i",
                Some(InfectionPhase::Infectious) => "X",
                Some(InfectionPhase::Recovered) | None => "",
            };
            let low_health = if human.body.health.saturating_mul(4) < human.body.max_health {
                "!"
            } else {
                ""
            };
            let mut style = Style::default().fg(stage_color(human.body.life_stage));
            if state.selected == Some(human.id) {
                style = style.add_modifier(Modifier::BOLD | Modifier::REVERSED);
            }
            Span::styled(
                format!(
                    "●{}g{}{}{} ",
                    human.id.0, human.lineage.generation, infection, low_health
                ),
                style,
            )
        })
        .collect::<Vec<_>>();
    let lines = entries
        .chunks(4)
        .map(|row| Line::from(row.to_vec()))
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(" World / population map ")),
        area,
    );
}
