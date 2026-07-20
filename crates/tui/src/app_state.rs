use anana_core::{EventRecord, GoshKind, HumanId, HumanState, WorldSnapshot};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Panel {
    #[default]
    World,
    Inspector,
    Feed,
    Narrative,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct StatusCounters {
    pub births: u64,
    pub deaths: u64,
    pub infections: u64,
    pub living: u64,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Narrative {
    pub title: String,
    pub story: String,
    pub epitaph: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GoshForm {
    pub draft: GoshKind,
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub snapshot: WorldSnapshot,
    pub selected: Option<HumanId>,
    pub feed_scroll: u16,
    pub feed_selected_only: bool,
    pub focus: Panel,
    pub gosh_form: Option<GoshForm>,
    pub narrative: Option<Narrative>,
    pub counters: StatusCounters,
    pub paused: bool,
    pub mode: String,
}

impl AppState {
    #[must_use]
    pub fn new(snapshot: WorldSnapshot, counters: StatusCounters) -> Self {
        let selected = snapshot.humans.keys().next().copied();
        Self {
            snapshot,
            selected,
            feed_scroll: 0,
            feed_selected_only: false,
            focus: Panel::World,
            gosh_form: None,
            narrative: None,
            counters,
            paused: false,
            mode: String::from("live"),
        }
    }

    pub fn update_snapshot(&mut self, snapshot: WorldSnapshot, counters: StatusCounters) {
        self.snapshot = snapshot;
        self.counters = counters;
        if self
            .selected
            .is_none_or(|selected| !self.snapshot.humans.contains_key(&selected))
        {
            self.selected = self.snapshot.humans.keys().next().copied();
            self.narrative = None;
        }
    }

    #[must_use]
    pub fn selected_human(&self) -> Option<&HumanState> {
        self.selected
            .and_then(|selected| self.snapshot.humans.get(&selected))
    }

    pub fn select_next(&mut self) -> Option<HumanId> {
        let next = match self.selected {
            Some(selected) => self
                .snapshot
                .humans
                .range((
                    std::ops::Bound::Excluded(selected),
                    std::ops::Bound::Unbounded,
                ))
                .next()
                .map(|(id, _)| *id)
                .or_else(|| self.snapshot.humans.keys().next().copied()),
            None => self.snapshot.humans.keys().next().copied(),
        };
        self.set_selected(next);
        next
    }

    pub fn select_prev(&mut self) -> Option<HumanId> {
        let previous = match self.selected {
            Some(selected) => self
                .snapshot
                .humans
                .range((
                    std::ops::Bound::Unbounded,
                    std::ops::Bound::Excluded(selected),
                ))
                .next_back()
                .map(|(id, _)| *id)
                .or_else(|| self.snapshot.humans.keys().next_back().copied()),
            None => self.snapshot.humans.keys().next_back().copied(),
        };
        self.set_selected(previous);
        previous
    }

    pub fn set_selected(&mut self, selected: Option<HumanId>) {
        if self.selected != selected {
            self.selected = selected;
            self.narrative = None;
        }
    }

    pub fn scroll_feed(&mut self, delta: i32) {
        if delta < 0 {
            let amount = u16::try_from(delta.unsigned_abs()).map_or(u16::MAX, |value| value);
            self.feed_scroll = self.feed_scroll.saturating_sub(amount);
        } else {
            let amount = u16::try_from(delta).map_or(u16::MAX, |value| value);
            self.feed_scroll = self.feed_scroll.saturating_add(amount);
        }
    }

    #[must_use]
    pub fn visible_events(&self) -> Vec<&EventRecord> {
        self.snapshot
            .event_log
            .iter()
            .filter(|record| {
                !self.feed_selected_only
                    || self
                        .selected
                        .is_some_and(|selected| record.subjects.contains(&selected))
            })
            .collect()
    }
}
