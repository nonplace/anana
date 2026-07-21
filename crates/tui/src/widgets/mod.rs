mod feed;
mod inspector;
mod narrative;
mod worldmap;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

use crate::{
    AppState, Panel,
    palette::{BACKGROUND, DIVINE_AMBER, HISTORICAL, LIVE, STRUCTURE, divine_panel},
};

const SPLASH_MIN_WIDTH: u16 = 64;
const SPLASH_MIN_HEIGHT: u16 = 9;
const COMPACT_WIDTH: u16 = 72;
const COMPACT_HEIGHT: u16 = 20;

fn render_splash(frame: &mut Frame<'_>, state: &AppState) -> bool {
    let area = frame.area();
    if !state.splash_visible() || area.width < SPLASH_MIN_WIDTH || area.height < SPLASH_MIN_HEIGHT {
        return false;
    }
    frame.render_widget(
        Block::default().style(Style::default().bg(BACKGROUND)),
        area,
    );
    let height = 6;
    let card = Rect {
        x: area.x,
        y: area
            .y
            .saturating_add(area.height.saturating_sub(height) / 2),
        width: area.width,
        height,
    };
    let lines = vec![
        Line::styled(
            "A n a n A",
            Style::default().fg(LIVE).add_modifier(Modifier::BOLD),
        ),
        Line::styled("---------|---------", Style::default().fg(STRUCTURE)),
        Line::raw(""),
        Line::styled(
            format!("seed {}", state.snapshot.seed),
            Style::default().fg(HISTORICAL),
        ),
        Line::styled(
            "a world where every life runs once, unless you run it twice.",
            Style::default().fg(HISTORICAL),
        ),
    ];
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), card);
    true
}

fn render_status(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let status = if area.width < COMPACT_WIDTH {
        format!(
            " t{:>6}  living {:>4}  {}{} ",
            state.snapshot.tick.0,
            state.counters.living,
            state.mode,
            if state.paused { " · PAUSED" } else { "" },
        )
    } else {
        format!(
            " tick {:>6}  living {:>4}  births {:>4}  deaths {:>4}  infections {:>4}  mode {:<8}{} ",
            state.snapshot.tick.0,
            state.counters.living,
            state.counters.births,
            state.counters.deaths,
            state.counters.infections,
            state.mode,
            if state.paused { "PAUSED" } else { "" },
        )
    };
    frame.render_widget(
        Paragraph::new(status).style(Style::default().fg(LIVE).bg(BACKGROUND)),
        area,
    );
}

fn render_compact(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    match state.focus {
        Panel::World => worldmap::render(frame, area, state),
        Panel::Inspector => inspector::render(frame, area, state),
        Panel::Feed => feed::render(frame, area, state),
        Panel::Narrative => narrative::render(frame, area, state),
    }
}

pub fn render(frame: &mut Frame<'_>, state: &AppState) {
    if render_splash(frame, state) {
        return;
    }
    frame.render_widget(
        Block::default().style(Style::default().fg(LIVE).bg(BACKGROUND)),
        frame.area(),
    );
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
    render_status(frame, *status_area, state);

    if frame.area().width < COMPACT_WIDTH || frame.area().height < COMPACT_HEIGHT {
        render_compact(frame, *body_area, state);
    } else {
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
        worldmap::render(frame, *world_area, state);
        feed::render(frame, *feed_area, state);
        inspector::render(frame, *inspector_area, state);
        narrative::render(frame, *narrative_area, state);
    }

    let hint = if state.mode == "replay" {
        " PgUp/PgDn scrub · ←/→ select · ↑/↓ feed · q quit "
    } else {
        " ←/→ select · ↑/↓ feed · tab focus · n narrate · g gosh · space pause · . step · q quit "
    };
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().fg(STRUCTURE).bg(BACKGROUND)),
        *hint_area,
    );

    if let Some(form) = &state.gosh_form {
        let full = frame.area();
        let width = full.width.saturating_sub(4).min(76);
        let height = 9_u16.min(full.height.saturating_sub(2));
        let modal = Rect {
            x: full.x.saturating_add(full.width.saturating_sub(width) / 2),
            y: full
                .y
                .saturating_add(full.height.saturating_sub(height) / 2),
            width,
            height,
        };
        frame.render_widget(Clear, modal);
        let body = vec![
            Line::from(vec![
                Span::styled("DECREE  ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{:?}", form.draft)),
            ]),
            Line::raw(""),
            Line::raw("b bless · a afflict · t teach · s seed · f fertility · i immunity"),
            Line::raw("+/- magnitude · l target · Enter confirm · Esc cancel"),
        ];
        frame.render_widget(
            Paragraph::new(body)
                .style(Style::default().fg(DIVINE_AMBER).bg(BACKGROUND))
                .block(divine_panel(" GOSH ")),
            modal,
        );
    }
}
