//! Terminal presentation layer for AnanA.

mod app_state;
mod input;
mod widgets;

pub use app_state::*;
pub use input::*;
pub use widgets::render;

#[cfg(test)]
mod tests {
    //! Dashboard rendering exposes the selected human, canonical log, population map, and Recall gate without mutating state.

    use std::collections::BTreeMap;

    use anana_core::{
        Bane, Body, Consciousness, DeterministicKind, DiseaseAllele, EventAuthor, EventOutcome,
        EventPayload, EventRecord, EyeAllele, GenePair, Genome, God, GodId, GoshKind, GoshTarget,
        HandAllele, HumanId, HumanState, Instincts, Lineage, Permille, PolySublocus,
        PolygenicLocus, Rng, Seq, SexAllele, SkillId, SkillState, Skills, Tick, Virus, VirusId,
        WorldSnapshot, express,
    };
    use ratatui::{
        Terminal,
        backend::TestBackend,
        crossterm::event::{KeyCode, KeyEvent},
    };

    use super::*;

    fn human(id: HumanId, recall: bool) -> HumanState {
        let locus = PolygenicLocus {
            subloci: [PolySublocus {
                maternal: 0,
                paternal: 1,
            }; 4],
        };
        let genome = Genome {
            eye: GenePair {
                maternal: EyeAllele::Brown,
                paternal: EyeAllele::Blue,
            },
            hand: GenePair {
                maternal: HandAllele::Right,
                paternal: HandAllele::Left,
            },
            disease_x: GenePair {
                maternal: DiseaseAllele::Healthy,
                paternal: DiseaseAllele::Risk,
            },
            sex: GenePair {
                maternal: SexAllele::X,
                paternal: if id.0.is_multiple_of(2) {
                    SexAllele::Y
                } else {
                    SexAllele::X
                },
            },
            robustness: locus,
            aptitude: locus,
        };
        let phenotype = express(&genome, &Rng::new(42), Tick(0), id);
        let mut body = Body::at_birth(&phenotype);
        body.age_ticks = 8_000;
        body.life_stage = anana_core::LifeStage::Adult;
        body.fertility = 70;
        let mut skills = Skills::default();
        if recall {
            skills.levels.insert(
                SkillId::Recall,
                SkillState {
                    xp: 100,
                    learned: true,
                },
            );
        }
        skills.levels.insert(
            SkillId::Motor,
            SkillState {
                xp: 300,
                learned: recall,
            },
        );
        HumanState {
            id,
            genome,
            phenotype,
            instincts: Instincts {
                survival: 60,
                reproduction: 70,
                hunger: 40,
                fear: 30,
                social: 80,
            },
            consciousness: Consciousness {
                awareness: 70,
                focus: 75,
                memory_capacity: 900,
            },
            body,
            skills,
            lineage: Lineage::new(id, None, None, id.0 as u32 - 1, Tick(0)),
            infection: None,
        }
    }

    fn state(recall: bool) -> AppState {
        let record = EventRecord {
            tick: Tick(4),
            seq: Seq(0),
            author: EventAuthor::Engine,
            subjects: vec![HumanId(1)],
            payload: EventPayload::Deterministic(DeterministicKind::Maturation),
            outcome: EventOutcome::NoOp,
            narration: Some(String::from("the known event line")),
        };
        AppState::new(
            WorldSnapshot {
                seed: 42,
                tick: Tick(5),
                next_human_id: HumanId(3),
                humans: BTreeMap::from([
                    (HumanId(1), human(HumanId(1), recall)),
                    (HumanId(2), human(HumanId(2), true)),
                ]),
                viruses: BTreeMap::from([(
                    VirusId(1),
                    Virus {
                        id: VirusId(1),
                        spreadscore: 30,
                        virulence: 10,
                        incubation_ticks: 5,
                        mutation_rate: Permille::ZERO,
                    },
                )]),
                gods: BTreeMap::from([(
                    GodId(1),
                    God {
                        id: GodId(1),
                        goshes_spoken: 0,
                    },
                )]),
                event_log: vec![record],
            },
            StatusCounters {
                births: 1,
                deaths: 0,
                infections: 2,
                living: 2,
            },
        )
    }

    fn rendered(state: &AppState) -> String {
        let backend = TestBackend::new(120, 42);
        let mut terminal = Terminal::new(backend).expect("the test terminal starts");
        terminal
            .draw(|frame| render(frame, state))
            .expect("the dashboard renders");
        let buffer = terminal.backend().buffer();
        buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }

    #[test]
    fn the_dashboard_renders_the_selected_human_event_feed_and_one_map_glyph_per_human() {
        let output = rendered(&state(true));
        assert!(output.contains("Human 1"));
        assert!(output.contains("the known event line"));
        assert_eq!(output.matches('●').count(), 2);
    }

    #[test]
    fn an_amnesic_human_shows_amnesia_instead_of_a_remembered_history() {
        let output = rendered(&state(false));
        assert!(output.contains("AMNESIA"));
        assert!(!output.contains("Remembered: the known event line"));
    }

    #[test]
    fn a_human_with_recall_can_read_their_remembered_history() {
        let output = rendered(&state(true));
        assert!(output.contains("Remembered: the known event line"));
    }

    #[test]
    fn rendering_the_same_presentation_state_twice_produces_identical_buffers() {
        let state = state(true);
        assert_eq!(rendered(&state), rendered(&state));
    }

    #[test]
    fn navigation_and_scrolling_return_view_intents_and_never_cast_a_gosh() {
        let mut state = state(true);
        let selected = handle_key(
            &mut state,
            KeyEvent::new(
                KeyCode::Right,
                ratatui::crossterm::event::KeyModifiers::NONE,
            ),
        );
        let scrolled = handle_key(
            &mut state,
            KeyEvent::new(KeyCode::Down, ratatui::crossterm::event::KeyModifiers::NONE),
        );
        assert_eq!(selected, UiIntent::Select(HumanId(2)));
        assert_eq!(scrolled, UiIntent::ScrollFeed(1));
        assert!(!matches!(selected, UiIntent::CastGosh(_)));
        assert!(!matches!(scrolled, UiIntent::CastGosh(_)));
    }

    #[test]
    fn confirming_a_completed_gosh_form_returns_exactly_the_decree_that_was_built() {
        let mut state = state(true);
        assert_eq!(
            handle_key(
                &mut state,
                KeyEvent::new(
                    KeyCode::Char('g'),
                    ratatui::crossterm::event::KeyModifiers::NONE
                )
            ),
            UiIntent::None
        );
        handle_key(
            &mut state,
            KeyEvent::new(
                KeyCode::Char('a'),
                ratatui::crossterm::event::KeyModifiers::NONE,
            ),
        );
        handle_key(
            &mut state,
            KeyEvent::new(
                KeyCode::Char('+'),
                ratatui::crossterm::event::KeyModifiers::NONE,
            ),
        );
        assert_eq!(
            handle_key(
                &mut state,
                KeyEvent::new(
                    KeyCode::Enter,
                    ratatui::crossterm::event::KeyModifiers::NONE
                )
            ),
            UiIntent::CastGosh(GoshKind::Afflict {
                target: GoshTarget::One(HumanId(1)),
                bane: Bane::Harm(15),
            })
        );
        assert!(state.gosh_form.is_none());
    }

    #[test]
    fn escaping_a_gosh_form_cancels_it_without_returning_a_decree() {
        let mut state = state(true);
        handle_key(
            &mut state,
            KeyEvent::new(
                KeyCode::Char('g'),
                ratatui::crossterm::event::KeyModifiers::NONE,
            ),
        );
        assert_eq!(
            handle_key(
                &mut state,
                KeyEvent::new(KeyCode::Esc, ratatui::crossterm::event::KeyModifiers::NONE)
            ),
            UiIntent::None
        );
        assert!(state.gosh_form.is_none());
    }
}
