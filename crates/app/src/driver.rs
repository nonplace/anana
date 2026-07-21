use std::time::Duration;

use anana_mind::{Mind, build_life_history, build_world_context, validate};
use anana_sim::{
    App, Config, EventAuthor, EventIntake, HashHistory, PendingEvent, SimulationFaults,
    SimulationStats, WorldClock, build_headless_app, replay_for_ticks, snapshot, step, world_hash,
};
use anana_tui::{
    AppState, Narrative, StatusCounters, UiIntent, handle_key, ratatui::crossterm::event::KeyCode,
};
use anyhow::{Result, anyhow};

use crate::terminal::TerminalGuard;

const AI_INTERVAL: u64 = 25;
const INPUT_POLL_MILLIS: u64 = 50;

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct HeadlessResult {
    pub final_hash: [u8; 32],
    pub stats: SimulationStats,
    pub faults: Vec<anana_sim::SimError>,
    pub tick: u64,
}

fn counters(app: &App) -> StatusCounters {
    let stats = app.world().resource::<SimulationStats>();
    StatusCounters {
        births: stats.births,
        deaths: stats.deaths,
        infections: stats.infections,
        living: stats.living,
    }
}

async fn enqueue_ai_events<M: Mind>(app: &mut App, mind: &M) -> Result<()> {
    let tick = app.world().resource::<WorldClock>().0;
    if tick.0 == 0 || !tick.0.is_multiple_of(AI_INTERVAL) {
        return Ok(());
    }
    let current = snapshot(app);
    let context = build_world_context(&current, 12);
    let Ok(batch) = mind.author_events(&context).await else {
        return Ok(());
    };
    let Ok(events) = validate(&batch, &context) else {
        return Ok(());
    };
    for event in events {
        app.world().resource::<EventIntake>().push(PendingEvent {
            author: EventAuthor::Ai,
            tick,
            subjects: event.subjects,
            payload: event.payload,
        })?;
    }
    Ok(())
}

async fn advance_one<M: Mind>(app: &mut App, mind: &M) -> Result<()> {
    enqueue_ai_events(app, mind).await?;
    step(app);
    Ok(())
}

pub(crate) async fn run_headless<M: Mind>(
    seed: u64,
    config: Config,
    ticks: u64,
    mind: &M,
) -> Result<HeadlessResult> {
    let mut app = build_headless_app(seed, config);
    for _ in 0..ticks {
        advance_one(&mut app, mind).await?;
    }
    let current = snapshot(&mut app);
    let final_hash = app
        .world()
        .resource::<HashHistory>()
        .0
        .last()
        .copied()
        .unwrap_or_else(|| world_hash(&current));
    Ok(HeadlessResult {
        final_hash,
        stats: *app.world().resource::<SimulationStats>(),
        faults: app.world().resource::<SimulationFaults>().0.clone(),
        tick: current.tick.0,
    })
}

async fn request_narration<M: Mind>(state: &mut AppState, mind: &M) {
    let Some(subject) = state.selected else {
        return;
    };
    let Some(human) = state.snapshot.humans.get(&subject) else {
        return;
    };
    let history = build_life_history(human, &state.snapshot.event_log);
    if let Ok(story) = mind.narrate(&history).await {
        state.narrative = Some(Narrative {
            title: story.title,
            story: story.story,
            epitaph: story.epitaph,
        });
    }
}

pub(crate) async fn run_live<M: Mind>(
    seed: u64,
    config: Config,
    tick_limit: Option<u64>,
    mind: &M,
) -> Result<()> {
    let mut app = build_headless_app(seed, config);
    let initial = snapshot(&mut app);
    let mut state = AppState::new(initial, counters(&app));
    let mut terminal = TerminalGuard::enter()?;
    loop {
        terminal.draw(&state)?;
        state.advance_splash();
        if tick_limit.is_some_and(|limit| state.snapshot.tick.0 >= limit) {
            break;
        }
        let mut stepped = false;
        if let Some(key) = TerminalGuard::poll_key(Duration::from_millis(INPUT_POLL_MILLIS))? {
            match handle_key(&mut state, key) {
                UiIntent::Quit => break,
                UiIntent::CastGosh(gosh) => {
                    app.world()
                        .resource::<EventIntake>()
                        .cast_gosh(app.world().resource::<WorldClock>().0, gosh)?;
                }
                UiIntent::RequestNarration(_) => request_narration(&mut state, mind).await,
                UiIntent::StepOnce => {
                    advance_one(&mut app, mind).await?;
                    stepped = true;
                }
                UiIntent::None
                | UiIntent::Select(_)
                | UiIntent::ScrollFeed(_)
                | UiIntent::FocusPanel(_)
                | UiIntent::TogglePause => {}
            }
        }
        if !state.paused && !stepped {
            advance_one(&mut app, mind).await?;
        }
        let current = snapshot(&mut app);
        state.update_snapshot(current, counters(&app));
    }
    Ok(())
}

fn replay_matches_prefix(expected: &[[u8; 32]], app: &App, ticks: u64) -> bool {
    let count = usize::try_from(ticks).map_or(usize::MAX, |value| value);
    let actual = &app.world().resource::<HashHistory>().0;
    actual.len() == expected.len().min(count) && actual.iter().eq(expected.iter().take(count))
}

pub(crate) fn run_replay(seed: u64, config: Config, ticks: u64) -> Result<()> {
    let mut original = build_headless_app(seed, config.clone());
    for _ in 0..ticks {
        step(&mut original);
    }
    let expected = original.world().resource::<HashHistory>().0.clone();
    let records = original
        .world()
        .resource::<anana_sim::EventLog>()
        .records()
        .to_vec();
    let mut cursor = ticks;
    let mut replayed = replay_for_ticks(seed, config.clone(), records.clone(), cursor);
    if !replay_matches_prefix(&expected, &replayed, cursor) {
        return Err(anyhow!("replay diverged before the dashboard opened"));
    }
    let current = snapshot(&mut replayed);
    let mut state = AppState::new(current, counters(&replayed));
    state.mode = String::from("replay");
    state.paused = true;
    let mut terminal = TerminalGuard::enter()?;
    loop {
        terminal.draw(&state)?;
        state.advance_splash();
        let Some(key) = TerminalGuard::poll_key(Duration::from_millis(INPUT_POLL_MILLIS))? else {
            continue;
        };
        let mut scrubbed = false;
        match key.code {
            KeyCode::PageUp => {
                cursor = cursor.saturating_sub(1);
                scrubbed = true;
            }
            KeyCode::PageDown => {
                cursor = cursor.saturating_add(1).min(ticks);
                scrubbed = true;
            }
            _ => {
                if handle_key(&mut state, key) == UiIntent::Quit {
                    break;
                }
            }
        }
        if scrubbed {
            replayed = replay_for_ticks(seed, config.clone(), records.clone(), cursor);
            if !replay_matches_prefix(&expected, &replayed, cursor) {
                return Err(anyhow!("replay diverged at tick {cursor}"));
            }
            let current = snapshot(&mut replayed);
            state.update_snapshot(current, counters(&replayed));
        }
    }
    Ok(())
}

#[must_use]
pub(crate) fn hash_hex(hash: [u8; 32]) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}
