use ratatui::{
    style::{Color, Style},
    widgets::{Block, Padding},
};

pub const BACKGROUND: Color = Color::Black;
pub const LIVE: Color = Color::LightGreen;
pub const STRUCTURE: Color = Color::Green;
pub const HISTORICAL: Color = Color::Green;

// Amber is reserved exclusively for divine acts. Never spend it on warnings, selection,
// infection, age, or any other non-divine state: one color must always mean "the god did this."
pub const DIVINE_AMBER: Color = Color::Yellow;

pub const ANSI_LIVE: &str = "\x1b[92m";
pub const ANSI_STRUCTURE: &str = "\x1b[32m";
pub const ANSI_DIVINE: &str = "\x1b[33m";
pub const ANSI_RESET: &str = "\x1b[0m";

pub fn panel<'a>(title: impl Into<ratatui::text::Line<'a>>) -> Block<'a> {
    Block::bordered()
        .title(title)
        .title_style(Style::default().fg(LIVE))
        .border_style(Style::default().fg(STRUCTURE))
        .style(Style::default().fg(LIVE).bg(BACKGROUND))
        .padding(Padding::uniform(1))
}

pub fn divine_panel<'a>(title: impl Into<ratatui::text::Line<'a>>) -> Block<'a> {
    Block::bordered()
        .title(title)
        .title_style(Style::default().fg(DIVINE_AMBER))
        .border_style(Style::default().fg(DIVINE_AMBER))
        .style(Style::default().fg(DIVINE_AMBER).bg(BACKGROUND))
        .padding(Padding::uniform(1))
}
