use std::io::{Stdout, stdout};

use anana_tui::{AppState, ratatui};
use anyhow::{Result, anyhow};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        cursor::{Hide, Show},
        event::{self, Event, KeyEvent},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};

pub(crate) struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    pub(crate) fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut output = stdout();
        if let Err(error) = execute!(output, EnterAlternateScreen, Hide) {
            let _ = disable_raw_mode();
            return Err(anyhow!(error));
        }
        match Terminal::new(CrosstermBackend::new(output)) {
            Ok(terminal) => Ok(Self { terminal }),
            Err(error) => {
                let _ = disable_raw_mode();
                let mut output = stdout();
                let _ = execute!(output, LeaveAlternateScreen, Show);
                Err(anyhow!(error))
            }
        }
    }

    pub(crate) fn draw(&mut self, state: &AppState) -> Result<()> {
        self.terminal
            .draw(|frame| anana_tui::render(frame, state))?;
        Ok(())
    }

    pub(crate) fn poll_key(timeout: std::time::Duration) -> Result<Option<KeyEvent>> {
        if !event::poll(timeout)? {
            return Ok(None);
        }
        match event::read()? {
            Event::Key(key) => Ok(Some(key)),
            _ => Ok(None),
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen, Show);
        let _ = self.terminal.show_cursor();
    }
}
