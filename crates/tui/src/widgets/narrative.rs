use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Paragraph, Wrap},
};

use crate::{
    AppState,
    palette::{HISTORICAL, LIVE, panel},
};

pub(super) fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let lines = match state.selected_human() {
        None => vec![Line::from("No living human selected")],
        Some(human) if !human.skills.recall_learned() => vec![
            Line::styled(
                "AMNESIA",
                Style::default().fg(LIVE).add_modifier(Modifier::BOLD),
            ),
            Line::styled(
                "Recall has not been learned. No personal history is accessible.",
                Style::default().fg(HISTORICAL),
            ),
        ],
        Some(human) => {
            if let Some(story) = &state.narrative {
                vec![
                    Line::styled(
                        story.title.clone(),
                        Style::default().fg(LIVE).add_modifier(Modifier::BOLD),
                    ),
                    Line::from(story.story.clone()),
                    Line::styled(story.epitaph.clone(), Style::default().fg(HISTORICAL)),
                ]
            } else {
                let remembered = state
                    .snapshot
                    .event_log
                    .iter()
                    .filter(|record| record.subjects.contains(&human.id))
                    .filter_map(|record| record.narration.as_deref())
                    .take(3)
                    .collect::<Vec<_>>()
                    .join("; ");
                let text = if remembered.is_empty() {
                    String::from("No narration fetched. Recall is online; no named memory yet.")
                } else {
                    format!("Remembered: {remembered}")
                };
                vec![Line::from(text)]
            }
        }
    };
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel(" STORY ")),
        area,
    );
}
