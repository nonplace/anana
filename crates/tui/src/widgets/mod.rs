mod feed;
mod inspector;
mod narrative;
mod worldmap;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Clear, Paragraph},
};

use crate::AppState;

pub fn render(frame: &mut Frame<'_>, state: &AppState) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(8),
            Constraint::Length(1),
        ])
        .split(frame.area());
    let (Some(status_area), Some(body_area), Some(hint_area)) =
        (outer.first(), outer.get(1), outer.get(2))
    else {
        return;
    };
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(*body_area);
    let (Some(left_column), Some(right_column)) = (columns.first(), columns.get(1)) else {
        return;
    };
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(*left_column);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(*right_column);
    let (Some(world_area), Some(feed_area), Some(inspector_area), Some(narrative_area)) =
        (left.first(), left.get(1), right.first(), right.get(1))
    else {
        return;
    };

    let status = Line::from(format!(
        " tick {}  living {}  births {}  deaths {}  infections {}  mode {}{} ",
        state.snapshot.tick.0,
        state.counters.living,
        state.counters.births,
        state.counters.deaths,
        state.counters.infections,
        state.mode,
        if state.paused { " [PAUSED]" } else { "" },
    ));
    frame.render_widget(
        Paragraph::new(status).style(Style::default().fg(Color::Black).bg(Color::Cyan)),
        *status_area,
    );
    worldmap::render(frame, *world_area, state);
    feed::render(frame, *feed_area, state);
    inspector::render(frame, *inspector_area, state);
    narrative::render(frame, *narrative_area, state);
    let hint = if state.mode == "replay" {
        " PgUp/PgDn scrub ticks  ←/→ select  ↑/↓ feed  q quit "
    } else {
        " ←/→ select  ↑/↓ feed  tab focus  n narrate  g gosh  space pause  . step  q quit "
    };
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::DarkGray)),
        *hint_area,
    );
    if let Some(form) = &state.gosh_form {
        let full = frame.area();
        let width = full.width.saturating_mul(2) / 3;
        let height = 7_u16.min(full.height);
        let modal = ratatui::layout::Rect {
            x: full.x.saturating_add(full.width.saturating_sub(width) / 2),
            y: full
                .y
                .saturating_add(full.height.saturating_sub(height) / 2),
            width,
            height,
        };
        frame.render_widget(Clear, modal);
        frame.render_widget(
            Paragraph::new(format!(
                "Draft: {:?}\n[b]less [a]fflict [t]each [s]eed [f]ertility [i]mmunity\n[+/-] magnitude [l] target  Enter confirm  Esc cancel",
                form.draft
            ))
            .block(Block::bordered().title(" Speak a gosh ")),
            modal,
        );
    }
}
