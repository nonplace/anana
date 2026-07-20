use anana_core::{EventAuthor, EventRecord};

use crate::{
    App, Config, EventIntake, PendingEvent, SimulationFaults, WorldClock, build_headless_app, step,
};

/// Rebuilds a world from its seed and re-enqueues recorded authored events at their capture ticks.
pub fn replay(seed: u64, config: Config, records: Vec<EventRecord>) -> App {
    let recorded_ticks = records
        .iter()
        .map(|record| record.tick.0)
        .fold(0_u64, u64::max);
    let authored_at_last_tick = records.iter().any(|record| {
        record.tick.0 == recorded_ticks
            && matches!(record.author, EventAuthor::God | EventAuthor::Ai)
    });
    let total_ticks = if records.is_empty() {
        0
    } else if authored_at_last_tick {
        recorded_ticks.saturating_add(1)
    } else {
        recorded_ticks.max(1)
    };
    let mut authored = records
        .into_iter()
        .filter(|record| matches!(record.author, EventAuthor::God | EventAuthor::Ai))
        .collect::<Vec<_>>();
    authored.sort_by_key(|record| (record.tick, record.seq));
    let mut authored = authored.into_iter().peekable();
    let mut app = build_headless_app(seed, config);

    for _ in 0..total_ticks {
        let now = app.world().resource::<WorldClock>().0;
        while let Some(next) = authored.peek() {
            if next.tick != now {
                break;
            }
            let Some(record) = authored.next() else {
                break;
            };
            let result = app.world().resource::<EventIntake>().push(PendingEvent {
                author: record.author,
                tick: record.tick,
                subjects: record.subjects,
                payload: record.payload,
            });
            if let Err(error) = result {
                app.world_mut()
                    .resource_mut::<SimulationFaults>()
                    .0
                    .push(error);
            }
        }
        step(&mut app);
    }
    app
}
