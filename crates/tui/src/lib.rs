//! Terminal presentation layer for AnanA.

mod app_state;
mod widgets;

pub use app_state::*;
pub use widgets::render;

#[cfg(test)]
mod tests {
    //! Dashboard rendering exposes the selected human, canonical log, population map, and Recall gate without mutating state.

    use std::collections::BTreeMap;

    use anana_core::{
        Body, Consciousness, DeterministicKind, DiseaseAllele, EventAuthor, EventOutcome,
        EventPayload, EventRecord, EyeAllele, GenePair, Genome, God, GodId, HandAllele, HumanId,
        HumanState, Instincts, Lineage, Permille, PolySublocus, PolygenicLocus, Rng, Seq,
        SexAllele, SkillId, SkillState, Skills, Tick, Virus, VirusId, WorldSnapshot, express,
    };
    use ratatui::{Terminal, backend::TestBackend};

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
}
