use anana_core::{Bane, Boon, GoshKind, GoshTarget, HumanId, SkillId};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use crate::{AppState, GoshForm, Panel};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum UiIntent {
    None,
    Quit,
    Select(HumanId),
    ScrollFeed(i32),
    FocusPanel(Panel),
    RequestNarration(HumanId),
    CastGosh(GoshKind),
    TogglePause,
    StepOnce,
}

fn next_panel(panel: Panel) -> Panel {
    match panel {
        Panel::World => Panel::Inspector,
        Panel::Inspector => Panel::Feed,
        Panel::Feed => Panel::Narrative,
        Panel::Narrative => Panel::World,
    }
}

fn set_gosh_kind(state: &mut AppState, code: char) {
    let Some(subject) = state.selected else {
        return;
    };
    let draft = match code {
        'b' => Some(GoshKind::Bless {
            subject,
            boon: Boon::Heal(10),
        }),
        'f' => Some(GoshKind::Bless {
            subject,
            boon: Boon::Fertility(10),
        }),
        'i' => {
            let virus = state.snapshot.viruses.keys().next().copied();
            virus.map(|virus| GoshKind::Bless {
                subject,
                boon: Boon::GrantImmunity(virus),
            })
        }
        'a' => Some(GoshKind::Afflict {
            target: GoshTarget::One(subject),
            bane: Bane::Harm(10),
        }),
        't' => Some(GoshKind::Teach {
            subject,
            skill: SkillId::Recall,
            xp: 100,
        }),
        's' => state
            .snapshot
            .humans
            .get(&subject)
            .map(|human| GoshKind::Seed {
                genome: human.genome.clone(),
            }),
        _ => None,
    };
    if let (Some(form), Some(draft)) = (state.gosh_form.as_mut(), draft) {
        form.draft = draft;
    }
}

fn adjust_magnitude(form: &mut GoshForm, increase: bool) {
    match &mut form.draft {
        GoshKind::Bless {
            boon: Boon::Heal(amount),
            ..
        }
        | GoshKind::Afflict {
            bane: Bane::Harm(amount),
            ..
        } => {
            *amount = if increase {
                amount.saturating_add(5)
            } else {
                amount.saturating_sub(5)
            };
        }
        GoshKind::Bless {
            boon: Boon::Fertility(amount),
            ..
        } => {
            *amount = if increase {
                amount.saturating_add(5).min(100)
            } else {
                amount.saturating_sub(5)
            };
        }
        GoshKind::Teach { xp, .. } => {
            *xp = if increase {
                xp.saturating_add(50)
            } else {
                xp.saturating_sub(50)
            };
        }
        GoshKind::Bless {
            boon: Boon::GrantImmunity(_),
            ..
        }
        | GoshKind::Afflict {
            bane: Bane::Infect(_),
            ..
        }
        | GoshKind::Seed { .. } => {}
    }
}

fn cycle_affliction_target(form: &mut GoshForm, selected: HumanId) {
    if let GoshKind::Afflict { target, .. } = &mut form.draft {
        *target = match *target {
            GoshTarget::One(subject) => GoshTarget::Lineage(subject),
            GoshTarget::Lineage(_) => GoshTarget::All,
            GoshTarget::All => GoshTarget::One(selected),
        };
    }
}

fn handle_gosh_key(state: &mut AppState, code: KeyCode) -> UiIntent {
    match code {
        KeyCode::Esc => {
            state.gosh_form = None;
            UiIntent::None
        }
        KeyCode::Enter => state
            .gosh_form
            .take()
            .map_or(UiIntent::None, |form| UiIntent::CastGosh(form.draft)),
        KeyCode::Char('+') | KeyCode::Char('=') => {
            if let Some(form) = state.gosh_form.as_mut() {
                adjust_magnitude(form, true);
            }
            UiIntent::None
        }
        KeyCode::Char('-') => {
            if let Some(form) = state.gosh_form.as_mut() {
                adjust_magnitude(form, false);
            }
            UiIntent::None
        }
        KeyCode::Char('l') => {
            if let (Some(form), Some(selected)) = (state.gosh_form.as_mut(), state.selected) {
                cycle_affliction_target(form, selected);
            }
            UiIntent::None
        }
        KeyCode::Char(code @ ('a' | 'b' | 'f' | 'i' | 's' | 't')) => {
            set_gosh_kind(state, code);
            UiIntent::None
        }
        _ => UiIntent::None,
    }
}

pub fn handle_key(state: &mut AppState, key: KeyEvent) -> UiIntent {
    if key.kind == KeyEventKind::Release {
        return UiIntent::None;
    }
    if state.splash_visible() {
        state.dismiss_splash();
        return UiIntent::None;
    }
    if state.gosh_form.is_some() {
        return handle_gosh_key(state, key.code);
    }
    match key.code {
        KeyCode::Char('q') => UiIntent::Quit,
        KeyCode::Right | KeyCode::Char('j') => {
            state.select_next().map_or(UiIntent::None, UiIntent::Select)
        }
        KeyCode::Left | KeyCode::Char('k') => {
            state.select_prev().map_or(UiIntent::None, UiIntent::Select)
        }
        KeyCode::Down => {
            state.scroll_feed(1);
            UiIntent::ScrollFeed(1)
        }
        KeyCode::Up => {
            state.scroll_feed(-1);
            UiIntent::ScrollFeed(-1)
        }
        KeyCode::Tab => {
            state.focus = next_panel(state.focus);
            UiIntent::FocusPanel(state.focus)
        }
        KeyCode::Char('n') => state
            .selected
            .map_or(UiIntent::None, UiIntent::RequestNarration),
        KeyCode::Char('g') => {
            if let Some(subject) = state.selected {
                state.gosh_form = Some(GoshForm {
                    draft: GoshKind::Bless {
                        subject,
                        boon: Boon::Heal(10),
                    },
                });
            }
            UiIntent::None
        }
        KeyCode::Char('f') => {
            state.feed_selected_only = !state.feed_selected_only;
            UiIntent::None
        }
        KeyCode::Char(' ') => {
            state.paused = !state.paused;
            UiIntent::TogglePause
        }
        KeyCode::Char('.') => UiIntent::StepOnce,
        _ => UiIntent::None,
    }
}
