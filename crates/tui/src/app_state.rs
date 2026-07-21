use std::collections::{BTreeMap, BTreeSet};

use anana_core::{
    DeterministicKind, EventOutcome, EventPayload, EventRecord, GoshKind, HumanId, HumanState,
    SkillId, Tick, VirusId, WorldSnapshot,
};

const SPLASH_FRAMES: u8 = 24;
const MOMENT_LIMIT: usize = 128;

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum PresentationMoment {
    RecallLearned {
        tick: Tick,
        human: HumanId,
    },
    KnowledgeLost {
        tick: Tick,
        human: HumanId,
        skills: Vec<SkillId>,
    },
    Recovered {
        tick: Tick,
        human: HumanId,
        virus: VirusId,
    },
    BondFormed {
        tick: Tick,
        first: HumanId,
        second: HumanId,
    },
}

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
    splash_frames_remaining: u8,
    pub(crate) moments: Vec<PresentationMoment>,
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
            splash_frames_remaining: SPLASH_FRAMES,
            moments: Vec::new(),
        }
    }

    pub fn update_snapshot(&mut self, snapshot: WorldSnapshot, counters: StatusCounters) {
        self.capture_recall_transitions(&snapshot);
        self.capture_knowledge_loss(&snapshot);
        self.capture_recoveries(&snapshot);
        self.capture_new_bonds(&snapshot);
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

    fn capture_recall_transitions(&mut self, next: &WorldSnapshot) {
        for (id, human) in &next.humans {
            let learned_before = self
                .snapshot
                .humans
                .get(id)
                .is_some_and(|previous| previous.skills.recall_learned());
            if !learned_before && human.skills.recall_learned() {
                self.moments.push(PresentationMoment::RecallLearned {
                    tick: next.tick,
                    human: *id,
                });
            }
        }
        self.trim_moments();
    }

    fn capture_knowledge_loss(&mut self, next: &WorldSnapshot) {
        let removed = self
            .snapshot
            .humans
            .iter()
            .filter(|(id, _)| !next.humans.contains_key(id))
            .collect::<BTreeMap<_, _>>();
        let potentially_lost = removed
            .values()
            .flat_map(|human| {
                human
                    .skills
                    .levels
                    .iter()
                    .filter_map(|(skill, state)| state.learned.then_some(*skill))
            })
            .filter(|skill| {
                !next.humans.values().any(|survivor| {
                    survivor
                        .skills
                        .levels
                        .get(skill)
                        .is_some_and(|state| state.learned)
                })
            })
            .collect::<BTreeSet<_>>();
        let death_sequence = |human: HumanId| {
            next.event_log
                .iter()
                .filter(|record| {
                    record.tick == next.tick
                        && matches!(
                            record.payload,
                            EventPayload::Deterministic(DeterministicKind::HealthTick)
                        )
                        && record.subjects.contains(&human)
                })
                .map(|record| record.seq.0)
                .max()
                .unwrap_or(0)
        };
        let mut losses = BTreeMap::<HumanId, Vec<SkillId>>::new();
        for skill in potentially_lost {
            let last_holder = removed
                .iter()
                .filter(|(_, human)| {
                    human
                        .skills
                        .levels
                        .get(&skill)
                        .is_some_and(|state| state.learned)
                })
                .max_by_key(|(id, _)| (death_sequence(***id), ***id));
            if let Some((id, _)) = last_holder {
                losses.entry(**id).or_default().push(skill);
            }
        }
        for (human, skills) in losses {
            self.moments.push(PresentationMoment::KnowledgeLost {
                tick: next.tick,
                human,
                skills,
            });
        }
        self.trim_moments();
    }

    fn capture_recoveries(&mut self, next: &WorldSnapshot) {
        for (id, human) in &next.humans {
            let Some(previous_infection) = self
                .snapshot
                .humans
                .get(id)
                .and_then(|previous| previous.infection.as_ref())
            else {
                continue;
            };
            if human.infection.is_none()
                && human.body.immunities.contains(&previous_infection.strain)
            {
                self.moments.push(PresentationMoment::Recovered {
                    tick: next.tick,
                    human: *id,
                    virus: previous_infection.strain,
                });
            }
        }
        self.trim_moments();
    }

    fn capture_new_bonds(&mut self, next: &WorldSnapshot) {
        let mut formed = BTreeSet::new();
        for (observer, human) in &next.humans {
            for (model, bond) in &human.social_bonds.bonds {
                if observer == model || bond.positive_interactions == 0 {
                    continue;
                }
                let (first, second) = if observer < model {
                    (*observer, *model)
                } else {
                    (*model, *observer)
                };
                let existed_before =
                    self.snapshot.humans.get(&first).is_some_and(|previous| {
                        previous
                            .social_bonds
                            .bonds
                            .get(&second)
                            .is_some_and(|old| old.positive_interactions > 0)
                    }) || self.snapshot.humans.get(&second).is_some_and(|previous| {
                        previous
                            .social_bonds
                            .bonds
                            .get(&first)
                            .is_some_and(|old| old.positive_interactions > 0)
                    });
                if !existed_before {
                    formed.insert((first, second));
                }
            }
        }
        self.moments
            .extend(
                formed
                    .into_iter()
                    .map(|(first, second)| PresentationMoment::BondFormed {
                        tick: next.tick,
                        first,
                        second,
                    }),
            );
        self.trim_moments();
    }

    fn trim_moments(&mut self) {
        let excess = self.moments.len().saturating_sub(MOMENT_LIMIT);
        if excess > 0 {
            self.moments.drain(..excess);
        }
    }

    #[must_use]
    pub fn splash_visible(&self) -> bool {
        self.splash_frames_remaining > 0
    }

    pub fn dismiss_splash(&mut self) {
        self.splash_frames_remaining = 0;
    }

    pub fn advance_splash(&mut self) {
        self.splash_frames_remaining = self.splash_frames_remaining.saturating_sub(1);
    }

    #[must_use]
    pub fn is_divinely_touched(&self, human: HumanId) -> bool {
        self.snapshot.event_log.iter().rev().any(|record| {
            record.tick == self.snapshot.tick
                && record.author == anana_core::EventAuthor::God
                && (record.subjects.contains(&human)
                    || matches!(
                        &record.outcome,
                        EventOutcome::Occurred(effects) if effects.contains_key(&human)
                    ))
        })
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
            self.feed_scroll = self.feed_scroll.saturating_add(amount);
        } else {
            let amount = u16::try_from(delta).map_or(u16::MAX, |value| value);
            self.feed_scroll = self.feed_scroll.saturating_sub(amount);
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
