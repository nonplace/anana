use anana_core::{EventAuthor, EventPayload};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use crate::AppState;

fn author_color(author: EventAuthor) -> Color {
    match author {
        EventAuthor::Engine => Color::Gray,
        EventAuthor::Ai => Color::LightBlue,
        EventAuthor::God => Color::LightMagenta,
    }
}

fn description(record: &anana_core::EventRecord) -> String {
    if let Some(narration) = &record.narration {
        return narration.clone();
    }
    match &record.payload {
        EventPayload::Chance { template, .. } => format!("{template:?}"),
        EventPayload::Deterministic(kind) => format!("{kind:?}"),
        EventPayload::Gosh(kind) => format!("{kind:?}"),
    }
}

pub(super) fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let lines = state
        .visible_events()
        .into_iter()
        .map(|record| {
            Line::from(vec![
                Span::styled(
                    format!("t{} s{} {:?} ", record.tick.0, record.seq.0, record.author),
                    Style::default().fg(author_color(record.author)),
                ),
                Span::raw(description(record)),
            ])
        })
        .collect::<Vec<_>>();
    let filter = if state.feed_selected_only {
        " selected"
    } else {
        " all"
    };
    frame.render_widget(
        Paragraph::new(lines)
            .scroll((state.feed_scroll, 0))
            .block(Block::bordered().title(format!(" Event history —{filter} "))),
        area,
    );
}
