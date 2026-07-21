use anana_core::{InfectionPhase, LifeStage};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{
    AppState,
    palette::{DIVINE_AMBER, LIVE, STRUCTURE, panel},
};

const CELL_WIDTH: u16 = 18;

fn stage_glyph(stage: LifeStage) -> &'static str {
    match stage {
        LifeStage::Infant => "·",
        LifeStage::Child => "○",
        LifeStage::Adolescent => "◌",
        LifeStage::Adult => "●",
        LifeStage::Elder => "◍",
    }
}

pub(super) fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let block = panel(" WORLD ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(inner);
    let (Some(map_area), Some(legend_area)) = (sections.first(), sections.get(1)) else {
        return;
    };
    let columns = usize::from((map_area.width / CELL_WIDTH).max(1));
    let entries = state
        .snapshot
        .humans
        .values()
        .map(|human| {
            let infection = match human.infection.as_ref().map(|infection| infection.phase) {
                Some(InfectionPhase::Incubating) => " i",
                Some(InfectionPhase::Infectious) => " X",
                Some(InfectionPhase::Recovered) | None => "  ",
            };
            let low_health = if human.body.health.saturating_mul(4) < human.body.max_health {
                "!"
            } else {
                " "
            };
            let divine = state.is_divinely_touched(human.id);
            let mut style = Style::default().fg(if divine { DIVINE_AMBER } else { LIVE });
            if state.selected == Some(human.id) {
                style = style.add_modifier(Modifier::BOLD | Modifier::REVERSED);
            } else if matches!(human.body.life_stage, LifeStage::Infant | LifeStage::Elder) {
                style = style.fg(if divine { DIVINE_AMBER } else { STRUCTURE });
            }
            Span::styled(
                format!(
                    "{} H{:>4} g{:>2}{infection}{low_health} ",
                    stage_glyph(human.body.life_stage),
                    human.id.0,
                    human.lineage.generation,
                ),
                style,
            )
        })
        .collect::<Vec<_>>();
    let lines = entries
        .chunks(columns)
        .map(|row| Line::from(row.to_vec()))
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), *map_area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                "· infant  ○ child  ◌ adolescent  ● adult  ◍ elder",
                Style::default().fg(STRUCTURE),
            ),
            Line::styled(
                "i incubating  X infectious  ! low health",
                Style::default().fg(STRUCTURE),
            ),
            Line::styled("H = human   g = generation", Style::default().fg(STRUCTURE)),
        ]),
        *legend_area,
    );
}
