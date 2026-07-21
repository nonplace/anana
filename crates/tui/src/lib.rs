//! Terminal presentation layer for AnanA.

mod app_state;
mod input;
mod palette;
mod widgets;

pub use app_state::*;
pub use input::*;
pub use palette::{ANSI_DIVINE, ANSI_LIVE, ANSI_RESET, ANSI_STRUCTURE, DIVINE_AMBER};
pub use ratatui;
pub use widgets::render;

#[cfg(test)]
mod tests {
    //! Dashboard rendering exposes the selected human, canonical log, population map, and Recall gate without mutating state.

    use std::collections::BTreeMap;

    use anana_core::{
        Bane, Body, Bond, ChanceTemplate, Consciousness, DeadHuman, DeterministicKind,
        DiseaseAllele, EffectSummary, EventAuthor, EventOutcome, EventPayload, EventRecord,
        EyeAllele, GenePair, Genome, God, GodId, GoshKind, GoshTarget, HandAllele, HumanId,
        HumanState, Infection, InfectionPhase, Instincts, Lineage, Permille, PolySublocus,
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
            threat_salience: GenePair {
                maternal: anana_core::ThreatSalienceAllele::Median,
                paternal: anana_core::ThreatSalienceAllele::Median,
            },
            novelty_tolerance: GenePair {
                maternal: anana_core::NoveltyToleranceAllele::Median,
                paternal: anana_core::NoveltyToleranceAllele::Median,
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
            residence: anana_core::Residence {
                id: anana_core::ResidenceId(1),
            },
            social_bonds: anana_core::SocialBonds::default(),
            positions: anana_core::Positions::default(),
            infection: None,
        }
    }

    fn state_with_splash(recall: bool) -> AppState {
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
                next_residence_id: anana_core::ResidenceId(2),
                humans: BTreeMap::from([
                    (HumanId(1), human(HumanId(1), recall)),
                    (HumanId(2), human(HumanId(2), true)),
                ]),
                dead: BTreeMap::new(),
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
                coalitions: BTreeMap::new(),
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

    fn state(recall: bool) -> AppState {
        let mut state = state_with_splash(recall);
        state.dismiss_splash();
        state
    }

    fn rendered_at(state: &AppState, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
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

    fn rendered(state: &AppState) -> String {
        rendered_at(state, 120, 42)
    }

    fn colors(state: &AppState) -> Vec<ratatui::style::Color> {
        let backend = TestBackend::new(120, 42);
        let mut terminal = Terminal::new(backend).expect("the test terminal starts");
        terminal
            .draw(|frame| render(frame, state))
            .expect("the dashboard renders");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.fg)
            .collect()
    }

    #[test]
    fn the_opening_frame_centres_the_palindrome_its_axis_and_the_seed() {
        let output = rendered(&state_with_splash(true));
        assert!(output.contains("A n a n A"));
        assert!(output.contains("---------|---------"));
        assert!(output.contains("seed 42"));
        assert!(output.contains("a world where every life runs once, unless you run it twice."));
        assert!(!output.contains("World / population map"));
    }

    #[test]
    fn any_key_dismisses_the_splash_without_triggering_its_normal_action() {
        let mut state = state_with_splash(true);
        let intent = handle_key(
            &mut state,
            KeyEvent::new(
                KeyCode::Char('g'),
                ratatui::crossterm::event::KeyModifiers::NONE,
            ),
        );
        assert_eq!(intent, UiIntent::None);
        assert!(!state.splash_visible());
        assert!(state.gosh_form.is_none());
    }

    #[test]
    fn a_terminal_too_narrow_for_the_wordmark_skips_the_splash_cleanly() {
        let output = rendered_at(&state_with_splash(true), 30, 16);
        assert!(!output.contains("A n a n A"));
        assert!(output.contains("WORLD"));
    }

    #[test]
    fn the_dashboard_renders_the_selected_human_event_feed_and_one_map_glyph_per_human() {
        let output = rendered(&state(true));
        assert!(output.contains("H1 ·"));
        assert!(output.contains("the known event line"));
        assert_eq!(output.matches("● H").count(), 2);
    }

    #[test]
    fn routine_no_op_events_do_not_drown_the_feed_in_engine_internals() {
        let mut state = state(true);
        state.snapshot.event_log.push(EventRecord {
            tick: Tick(5),
            seq: Seq(1),
            author: EventAuthor::Engine,
            subjects: vec![HumanId(1), HumanId(2)],
            payload: EventPayload::Chance {
                template: ChanceTemplate::Conflict,
                base_prob: Permille(20),
                skill_modifier: Some(SkillId::Planning),
                modifier_strength: Permille(10),
            },
            outcome: EventOutcome::NoOp,
            narration: None,
        });
        let output = rendered(&state);
        assert!(!output.contains("Engine Conflict"));
        assert!(!output.contains("Conflict"));
    }

    #[test]
    fn births_illness_death_and_divine_acts_read_as_sentences_about_people() {
        let mut state = state(true);
        let genome = state
            .snapshot
            .humans
            .get(&HumanId(1))
            .expect("the fixture parent exists")
            .genome
            .clone();
        let mut child = human(HumanId(3), true);
        child.lineage = Lineage::new(HumanId(3), Some(HumanId(1)), Some(HumanId(2)), 1, Tick(5));
        state.snapshot.humans.insert(HumanId(3), child);
        state.snapshot.event_log.extend([
            EventRecord {
                tick: Tick(5),
                seq: Seq(2),
                author: EventAuthor::Engine,
                subjects: vec![HumanId(1), HumanId(2), HumanId(3)],
                payload: EventPayload::Deterministic(DeterministicKind::Maturation),
                outcome: EventOutcome::Occurred(BTreeMap::from([(
                    HumanId(3),
                    EffectSummary {
                        seeded_genome: Some(genome),
                        ..EffectSummary::default()
                    },
                )])),
                narration: None,
            },
            EventRecord {
                tick: Tick(5),
                seq: Seq(3),
                author: EventAuthor::Engine,
                subjects: vec![HumanId(2), HumanId(1)],
                payload: EventPayload::Chance {
                    template: ChanceTemplate::Conflict,
                    base_prob: Permille::ONE,
                    skill_modifier: Some(SkillId::Medicine),
                    modifier_strength: Permille::ZERO,
                },
                outcome: EventOutcome::Occurred(BTreeMap::from([(
                    HumanId(1),
                    EffectSummary {
                        infection: Some(VirusId(1)),
                        ..EffectSummary::default()
                    },
                )])),
                narration: None,
            },
            EventRecord {
                tick: Tick(5),
                seq: Seq(4),
                author: EventAuthor::God,
                subjects: vec![HumanId(2)],
                payload: EventPayload::Gosh(GoshKind::Bless {
                    subject: HumanId(2),
                    boon: anana_core::Boon::Heal(10),
                }),
                outcome: EventOutcome::NoOp,
                narration: None,
            },
        ]);
        let output = rendered(&state);
        assert!(output.contains("H3 was born to H1 and H2"));
        assert!(output.contains("H1 fell ill after contact with H2"));
        assert!(output.contains("God healed H2"));
        assert!(!output.contains("Maturation"));
        assert!(!output.contains("Bless {"));
    }

    #[test]
    fn every_displayed_percentage_is_clamped_to_a_human_scale() {
        let mut state = state(true);
        let selected = state
            .snapshot
            .humans
            .get_mut(&HumanId(1))
            .expect("the selected fixture human exists");
        selected.phenotype.novelty_tolerance = anana_core::PerceptualGain::HIGH;
        selected.instincts.survival = u8::MAX;
        selected.consciousness.awareness = u8::MAX;
        selected.consciousness.focus = u8::MAX;
        selected.consciousness.memory_capacity = u16::MAX;
        selected.body.fertility = u8::MAX;
        selected.social_bonds.bonds.insert(
            HumanId(2),
            Bond {
                strength: Permille::ONE,
                last_interaction: Tick(5),
                last_decay_tick: Tick(5),
                positive_interactions: 8,
                defections: 0,
            },
        );
        let output = rendered(&state);
        assert!(output.contains("novelty 100/100"));
        assert!(output.contains("H2 ██████████ 100/100"));
        assert!(output.contains("survival 100/100"));
        assert!(output.contains("awareness 100/100"));
        assert!(output.contains("memory 1000/1000"));
        assert!(output.contains("fertility 100"));
        for token in output.split_whitespace() {
            if let Some(number) = token.strip_suffix('%') {
                let value = number
                    .parse::<u16>()
                    .expect("a displayed percentage is numeric");
                assert!(value <= 100, "screen leaked {value}%");
            }
        }
        assert!(!output.contains("1000%"));
        assert!(!output.contains("1500%"));
        assert!(!output.contains("1000‰"));
        assert!(!output.contains("1500‰"));
    }

    #[test]
    fn the_world_map_explains_every_glyph_and_abbreviation_it_uses() {
        let output = rendered(&state(true));
        assert!(output.contains("· infant"));
        assert!(output.contains("○ child"));
        assert!(output.contains("◌ adolescent"));
        assert!(output.contains("● adult"));
        assert!(output.contains("◍ elder"));
        assert!(output.contains("i incubating"));
        assert!(output.contains("X infectious"));
        assert!(output.contains("H = human"));
        assert!(output.contains("g = generation"));
    }

    #[test]
    fn the_gosh_modal_explains_the_one_power_target_effect_and_decision() {
        let mut state = state(true);
        state.gosh_form = Some(GoshForm {
            draft: GoshKind::Bless {
                subject: HumanId(1),
                boon: anana_core::Boon::Heal(10),
            },
        });
        let output = rendered(&state);
        assert!(output.contains("Your one power over this world"));
        assert!(output.contains("Blessing"));
        assert!(output.contains("TARGET"));
        assert!(output.contains("H1"));
        assert!(output.contains("Restore 10 health"));
        assert!(output.contains("Enter cast"));
        assert!(output.contains("Esc cancel"));
        assert!(!output.contains("Bless {"));
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
    fn amber_never_appears_when_no_divine_path_is_visible() {
        assert!(!colors(&state(true)).contains(&DIVINE_AMBER));
    }

    #[test]
    fn amber_marks_a_divine_record_and_the_gosh_form() {
        let mut state = state(true);
        state.snapshot.event_log.push(EventRecord {
            tick: state.snapshot.tick,
            seq: Seq(1),
            author: EventAuthor::God,
            subjects: vec![HumanId(1)],
            payload: EventPayload::Gosh(GoshKind::Bless {
                subject: HumanId(1),
                boon: anana_core::Boon::Heal(10),
            }),
            outcome: EventOutcome::NoOp,
            narration: None,
        });
        state.gosh_form = Some(GoshForm {
            draft: GoshKind::Bless {
                subject: HumanId(1),
                boon: anana_core::Boon::Heal(10),
            },
        });
        let amber_cells = colors(&state)
            .into_iter()
            .filter(|color| *color == DIVINE_AMBER)
            .count();
        assert!(amber_cells > 0);
    }

    #[test]
    fn a_worldwide_decree_marks_every_affected_human_as_divinely_touched() {
        let mut state = state(true);
        state.snapshot.event_log.push(EventRecord {
            tick: state.snapshot.tick,
            seq: Seq(1),
            author: EventAuthor::God,
            subjects: Vec::new(),
            payload: EventPayload::Gosh(GoshKind::Afflict {
                target: GoshTarget::All,
                bane: Bane::Harm(10),
            }),
            outcome: EventOutcome::Occurred(BTreeMap::from([
                (
                    HumanId(1),
                    EffectSummary {
                        health_delta: -10,
                        ..EffectSummary::default()
                    },
                ),
                (
                    HumanId(2),
                    EffectSummary {
                        health_delta: -10,
                        ..EffectSummary::default()
                    },
                ),
            ])),
            narration: None,
        });
        assert!(state.is_divinely_touched(HumanId(1)));
        assert!(state.is_divinely_touched(HumanId(2)));
    }

    #[test]
    fn learning_recall_becomes_the_strongest_non_divine_moment_in_the_feed() {
        let mut state = state(false);
        let mut next = state.snapshot.clone();
        let learner = next
            .humans
            .get_mut(&HumanId(1))
            .expect("the fixture learner exists");
        learner.skills.levels.insert(
            SkillId::Recall,
            SkillState {
                xp: 100,
                learned: true,
            },
        );
        next.tick = Tick(6);
        state.update_snapshot(next, state.counters.clone());
        assert!(
            rendered(&state).contains("H1 learned Recall and can now remember"),
            "the feed should name the moment memory becomes possible"
        );
    }

    #[test]
    fn recovery_becomes_a_plain_sentence_when_the_infection_clears() {
        let mut state = state(true);
        state
            .snapshot
            .humans
            .get_mut(&HumanId(1))
            .expect("the fixture patient exists")
            .infection = Some(Infection {
            strain: VirusId(1),
            ticks: 20,
            severity: 10,
            phase: InfectionPhase::Infectious,
        });
        let mut next = state.snapshot.clone();
        let recovered = next
            .humans
            .get_mut(&HumanId(1))
            .expect("the fixture patient survives");
        recovered.infection = None;
        recovered.body.immunities.insert(VirusId(1));
        next.tick = Tick(6);
        state.update_snapshot(next, state.counters.clone());
        assert!(rendered(&state).contains("H1 recovered from illness V1"));
    }

    #[test]
    fn a_first_one_way_attachment_is_described_once_as_a_shared_bond() {
        let mut state = state(true);
        let mut next = state.snapshot.clone();
        next.humans
            .get_mut(&HumanId(2))
            .expect("the second fixture human exists")
            .social_bonds
            .bonds
            .insert(
                HumanId(1),
                Bond {
                    strength: Permille(200),
                    last_interaction: Tick(6),
                    last_decay_tick: Tick(6),
                    positive_interactions: 1,
                    defections: 0,
                },
            );
        next.tick = Tick(6);
        state.update_snapshot(next, state.counters.clone());
        let output = rendered(&state);
        assert_eq!(output.matches("H1 and H2 formed a bond").count(), 1);
    }

    #[test]
    fn derived_life_moments_remain_in_tick_order_with_the_canonical_feed() {
        let mut state = state(false);
        let mut next = state.snapshot.clone();
        next.humans
            .get_mut(&HumanId(1))
            .expect("the fixture learner exists")
            .skills
            .levels
            .insert(
                SkillId::Recall,
                SkillState {
                    xp: 100,
                    learned: true,
                },
            );
        next.tick = Tick(6);
        state.update_snapshot(next, state.counters.clone());
        state.snapshot.event_log.push(EventRecord {
            tick: Tick(7),
            seq: Seq(8),
            author: EventAuthor::Engine,
            subjects: vec![HumanId(1)],
            payload: EventPayload::Deterministic(DeterministicKind::Maturation),
            outcome: EventOutcome::NoOp,
            narration: Some(String::from("a later event")),
        });
        state.focus = Panel::Feed;
        let output = rendered_at(&state, 100, 18);
        let recall = output
            .find("learned Recall")
            .expect("the Recall moment appears");
        let later = output
            .find("a later event")
            .expect("the later event appears");
        assert!(recall < later);
    }

    #[test]
    fn a_death_that_removes_the_last_holder_names_the_lost_knowledge() {
        let mut state = state(true);
        let holder = state
            .snapshot
            .humans
            .get_mut(&HumanId(1))
            .expect("the fixture holder exists");
        holder.skills.levels.insert(
            SkillId::Medicine,
            SkillState {
                xp: 300,
                learned: true,
            },
        );
        let mut next = state.snapshot.clone();
        let dead = next
            .humans
            .remove(&HumanId(1))
            .expect("the holder can die in the fixture");
        next.dead.insert(
            HumanId(1),
            DeadHuman {
                id: dead.id,
                generation: dead.lineage.generation,
                birth_tick: dead.lineage.birth_tick,
                death_tick: Tick(6),
                lineage: dead.lineage,
                skills: dead.skills,
                positions: dead.positions,
                social_bonds: dead.social_bonds,
            },
        );
        next.tick = Tick(6);
        state.update_snapshot(next, state.counters.clone());
        assert!(rendered(&state).contains("H1 died; Medicine was lost"));
    }

    #[test]
    fn simultaneous_deaths_name_only_the_final_holder_as_the_loss_of_knowledge() {
        let mut state = state(false);
        for human in state.snapshot.humans.values_mut() {
            human.skills.levels.clear();
            human.skills.memories.clear();
            human.skills.levels.insert(
                SkillId::Medicine,
                SkillState {
                    xp: 300,
                    learned: true,
                },
            );
        }
        let mut next = state.snapshot.clone();
        for (seq, id) in [(Seq(1), HumanId(1)), (Seq(2), HumanId(2))] {
            let dead = next
                .humans
                .remove(&id)
                .expect("each fixture holder can die");
            next.dead.insert(
                id,
                DeadHuman {
                    id,
                    generation: dead.lineage.generation,
                    birth_tick: dead.lineage.birth_tick,
                    death_tick: Tick(6),
                    lineage: dead.lineage,
                    skills: dead.skills,
                    positions: dead.positions,
                    social_bonds: dead.social_bonds,
                },
            );
            next.event_log.push(EventRecord {
                tick: Tick(6),
                seq,
                author: EventAuthor::Engine,
                subjects: vec![id],
                payload: EventPayload::Deterministic(DeterministicKind::HealthTick),
                outcome: EventOutcome::NoOp,
                narration: None,
            });
        }
        next.tick = Tick(6);
        state.update_snapshot(next, state.counters.clone());
        let losses = state
            .moments
            .iter()
            .filter_map(|moment| match moment {
                app_state::PresentationMoment::KnowledgeLost { human, skills, .. } => {
                    Some((*human, skills.clone()))
                }
                app_state::PresentationMoment::RecallLearned { .. }
                | app_state::PresentationMoment::Recovered { .. }
                | app_state::PresentationMoment::BondFormed { .. } => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(losses, vec![(HumanId(2), vec![SkillId::Medicine])]);
    }

    #[test]
    fn births_say_whether_they_begin_or_continue_a_lineage() {
        let mut state = state(true);
        let genome = state
            .snapshot
            .humans
            .get(&HumanId(1))
            .expect("the fixture parent exists")
            .genome
            .clone();
        let mut founder = human(HumanId(3), true);
        founder.lineage = Lineage::new(HumanId(3), None, None, 0, Tick(5));
        let mut descendant = human(HumanId(4), true);
        descendant.lineage =
            Lineage::new(HumanId(4), Some(HumanId(1)), Some(HumanId(2)), 1, Tick(5));
        state.snapshot.humans.insert(HumanId(3), founder);
        state.snapshot.humans.insert(HumanId(4), descendant);
        for (seq, child) in [(Seq(2), HumanId(3)), (Seq(3), HumanId(4))] {
            state.snapshot.event_log.push(EventRecord {
                tick: Tick(5),
                seq,
                author: EventAuthor::Engine,
                subjects: vec![child],
                payload: EventPayload::Deterministic(DeterministicKind::Maturation),
                outcome: EventOutcome::Occurred(BTreeMap::from([(
                    child,
                    EffectSummary {
                        seeded_genome: Some(genome.clone()),
                        ..EffectSummary::default()
                    },
                )])),
                narration: None,
            });
        }
        let output = rendered(&state);
        assert!(output.contains("H3 began a new lineage"));
        assert!(output.contains("H4 was born to H1 and H2"));
    }

    #[test]
    fn the_inspector_reads_from_identity_through_memory_knowledge_bonds_and_events() {
        let mut state = state(true);
        state
            .snapshot
            .humans
            .get_mut(&HumanId(1))
            .expect("the selected human exists")
            .social_bonds
            .observed_competence
            .insert(HumanId(2), 700);
        let output = rendered(&state);
        let identity = output.find("H1 ·").expect("identity is shown");
        let memory = output.find("MEMORY").expect("memory status is shown");
        let knowledge = output.find("KNOWLEDGE").expect("knowledge is shown");
        let attachments = output.find("ATTACHMENTS").expect("attachments are shown");
        let events = output.find("LIFE EVENTS").expect("life events are shown");
        assert!(identity < memory);
        assert!(memory < knowledge);
        assert!(knowledge < attachments);
        assert!(attachments < events);
        assert!(output.contains("Learned around H2"));
    }

    #[test]
    fn an_amnesic_inspector_says_there_is_no_history_yet() {
        assert!(rendered(&state(false)).contains("No history yet — Recall has not been learned."));
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
