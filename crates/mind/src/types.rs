use std::future::Future;

use anana_core::{
    DiseaseStatus, EventAuthor, EventPayload, EventRecord, EyeColor, Handedness, HumanId,
    HumanState, LifeStage, Sex, SkillId, Tick, VirusId, WorldSnapshot,
};
use serde::{Deserialize, Serialize};

use crate::MindError;

const ALL_SKILLS: [SkillId; 9] = [
    SkillId::Recall,
    SkillId::Motor,
    SkillId::Language,
    SkillId::Foraging,
    SkillId::ToolUse,
    SkillId::SocialBond,
    SkillId::Farming,
    SkillId::Medicine,
    SkillId::Planning,
];

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct TraitSummary {
    pub eye_color: EyeColor,
    pub handedness: Handedness,
    pub disease_status: DiseaseStatus,
    pub robustness: u8,
    pub aptitude: u8,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct HistoryEntry {
    pub tick: Tick,
    pub seq: anana_core::Seq,
    pub author: EventAuthor,
    pub summary: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct LifeHistory {
    pub subject: HumanId,
    pub sex: Sex,
    pub life_stage: LifeStage,
    pub age_ticks: u32,
    pub generation: u32,
    pub parents: (Option<HumanId>, Option<HumanId>),
    pub children: Vec<HumanId>,
    pub traits: TraitSummary,
    pub skills: Vec<(SkillId, u8)>,
    pub recall_learned: bool,
    pub events: Vec<HistoryEntry>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct LifeStory {
    pub title: String,
    pub story: String,
    pub epitaph: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct HumanBrief {
    pub id: HumanId,
    pub sex: Sex,
    pub life_stage: LifeStage,
    pub age_ticks: u32,
    pub health: u16,
    pub max_health: u16,
    pub notable_traits: TraitSummary,
    pub top_skills: Vec<(SkillId, u8)>,
    pub infected: Option<VirusId>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct VirusBrief {
    pub id: VirusId,
    pub spreadscore: u8,
    pub virulence: u8,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct WorldContext {
    pub tick: Tick,
    pub humans: Vec<HumanBrief>,
    pub viruses: Vec<VirusBrief>,
    pub recent: Vec<HistoryEntry>,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub enum AiEventKind {
    Chance,
    Deterministic,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub enum AiOp {
    Add,
    Set,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub enum AiField {
    Health,
    Fertility,
    AgeTicks,
    SkillXp,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct AiEffect {
    pub target: u64,
    pub op: AiOp,
    pub field: AiField,
    pub value: i64,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct AiEventProposal {
    pub subject_id: u64,
    pub kind: AiEventKind,
    pub title: String,
    pub description: String,
    pub base_prob: Option<u16>,
    pub skill_modifier: Option<SkillId>,
    pub modifier_strength: Option<u16>,
    pub effects: Vec<AiEffect>,
    pub seed_salt: u32,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct AiEventBatch {
    pub events: Vec<AiEventProposal>,
}

pub trait Mind {
    fn narrate(
        &self,
        history: &LifeHistory,
    ) -> impl Future<Output = Result<LifeStory, MindError>> + Send;
    fn author_events(
        &self,
        context: &WorldContext,
    ) -> impl Future<Output = Result<AiEventBatch, MindError>> + Send;
}

fn trait_summary(human: &HumanState) -> TraitSummary {
    TraitSummary {
        eye_color: human.phenotype.eye_color,
        handedness: human.phenotype.handedness,
        disease_status: human.phenotype.disease_x,
        robustness: human.phenotype.robustness,
        aptitude: human.phenotype.aptitude,
    }
}

fn event_summary(record: &EventRecord) -> String {
    if let Some(narration) = &record.narration {
        return narration.clone();
    }
    match &record.payload {
        EventPayload::Chance { template, .. } => {
            format!("A {template:?} chance event was resolved")
        }
        EventPayload::Deterministic(kind) => format!("The world applied {kind:?}"),
        EventPayload::Gosh(kind) => format!("A divine {kind:?} decree was spoken"),
    }
}

fn history_entry(record: &EventRecord) -> HistoryEntry {
    HistoryEntry {
        tick: record.tick,
        seq: record.seq,
        author: record.author,
        summary: event_summary(record),
    }
}

#[must_use]
pub fn build_life_history(human: &HumanState, log: &[EventRecord]) -> LifeHistory {
    let recall_learned = human.skills.recall_learned();
    let mut events = if recall_learned {
        log.iter()
            .filter(|record| record.subjects.contains(&human.id))
            .map(history_entry)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    events.sort_by_key(|entry| (entry.tick, entry.seq));
    LifeHistory {
        subject: human.id,
        sex: human.phenotype.sex,
        life_stage: human.body.life_stage,
        age_ticks: human.body.age_ticks,
        generation: human.lineage.generation,
        parents: (human.lineage.mother, human.lineage.father),
        children: human.lineage.children.clone(),
        traits: trait_summary(human),
        skills: ALL_SKILLS
            .into_iter()
            .map(|skill| (skill, human.skills.level_of(skill)))
            .collect(),
        recall_learned,
        events,
    }
}

#[must_use]
pub fn build_world_context(snapshot: &WorldSnapshot, recent_limit: usize) -> WorldContext {
    let humans = snapshot
        .humans
        .values()
        .map(|human| {
            let mut top_skills = ALL_SKILLS
                .into_iter()
                .map(|skill| (skill, human.skills.level_of(skill)))
                .filter(|(_, level)| *level > 0)
                .collect::<Vec<_>>();
            top_skills.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));
            top_skills.truncate(3);
            HumanBrief {
                id: human.id,
                sex: human.phenotype.sex,
                life_stage: human.body.life_stage,
                age_ticks: human.body.age_ticks,
                health: human.body.health,
                max_health: human.body.max_health,
                notable_traits: trait_summary(human),
                top_skills,
                infected: human.infection.as_ref().map(|infection| infection.strain),
            }
        })
        .collect();
    let recent_start = snapshot.event_log.len().saturating_sub(recent_limit);
    let recent = snapshot
        .event_log
        .iter()
        .skip(recent_start)
        .map(history_entry)
        .collect();
    let viruses = snapshot
        .viruses
        .values()
        .map(|virus| VirusBrief {
            id: virus.id,
            spreadscore: virus.spreadscore,
            virulence: virus.virulence,
        })
        .collect();
    WorldContext {
        tick: snapshot.tick,
        humans,
        viruses,
        recent,
    }
}

#[cfg(test)]
mod tests {
    //! Prompt inputs preserve the recall gate and canonical ordering, while JSON rejects invented fields.

    use std::collections::BTreeMap;

    use anana_core::{
        Body, Consciousness, DeterministicKind, DiseaseAllele, EventAuthor, EventOutcome,
        EventPayload, EventRecord, EyeAllele, GenePair, Genome, God, GodId, HandAllele, HumanId,
        HumanState, Instincts, Lineage, Permille, PolySublocus, PolygenicLocus, Seq, SexAllele,
        SkillId, SkillState, Skills, Tick, Virus, VirusId, WorldSnapshot, express,
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
                paternal: SexAllele::Y,
            },
            robustness: locus,
            aptitude: locus,
        };
        let phenotype = express(&genome, &anana_core::Rng::new(42), Tick(0), id);
        let mut skills = Skills::default();
        skills.levels.insert(
            SkillId::Motor,
            SkillState {
                xp: 300,
                learned: recall,
            },
        );
        if recall {
            skills.levels.insert(
                SkillId::Recall,
                SkillState {
                    xp: 100,
                    learned: true,
                },
            );
        }
        HumanState {
            id,
            genome,
            phenotype: phenotype.clone(),
            instincts: Instincts {
                survival: 50,
                reproduction: 50,
                hunger: 50,
                fear: 50,
                social: 50,
            },
            consciousness: Consciousness {
                awareness: 80,
                focus: 70,
                memory_capacity: 800,
            },
            body: Body::at_birth(&phenotype),
            skills,
            lineage: Lineage::new(id, None, None, 2, Tick(0)),
            infection: None,
        }
    }

    fn record(tick: u64, seq: u32, subject: HumanId) -> EventRecord {
        EventRecord {
            tick: Tick(tick),
            seq: Seq(seq),
            author: EventAuthor::Engine,
            subjects: vec![subject],
            payload: EventPayload::Deterministic(DeterministicKind::Maturation),
            outcome: EventOutcome::NoOp,
            narration: None,
        }
    }

    #[test]
    fn an_amnesic_life_history_contains_no_event_memories() {
        let history = build_life_history(&human(HumanId(1), false), &[record(1, 0, HumanId(1))]);
        assert!(!history.recall_learned);
        assert!(history.events.is_empty());
    }

    #[test]
    fn a_remembering_life_history_orders_skills_and_named_events_canonically() {
        let history = build_life_history(
            &human(HumanId(1), true),
            &[
                record(2, 1, HumanId(1)),
                record(1, 0, HumanId(1)),
                record(0, 0, HumanId(2)),
            ],
        );
        assert_eq!(history.skills[0].0, SkillId::Recall);
        assert_eq!(history.events.len(), 2);
        assert_eq!(history.events[0].tick, Tick(1));
        assert_eq!(history.events[1].tick, Tick(2));
    }

    #[test]
    fn a_world_context_sorts_humans_viruses_and_keeps_the_requested_recent_tail() {
        let snapshot = WorldSnapshot {
            seed: 42,
            tick: Tick(9),
            next_human_id: HumanId(3),
            humans: BTreeMap::from([
                (HumanId(2), human(HumanId(2), true)),
                (HumanId(1), human(HumanId(1), false)),
            ]),
            viruses: BTreeMap::from([
                (
                    VirusId(2),
                    Virus {
                        id: VirusId(2),
                        spreadscore: 20,
                        virulence: 10,
                        incubation_ticks: 5,
                        mutation_rate: Permille::ZERO,
                    },
                ),
                (
                    VirusId(1),
                    Virus {
                        id: VirusId(1),
                        spreadscore: 10,
                        virulence: 5,
                        incubation_ticks: 4,
                        mutation_rate: Permille::ZERO,
                    },
                ),
            ]),
            gods: BTreeMap::from([(
                GodId(1),
                God {
                    id: GodId(1),
                    goshes_spoken: 0,
                },
            )]),
            event_log: vec![
                record(1, 0, HumanId(1)),
                record(2, 1, HumanId(2)),
                record(3, 2, HumanId(1)),
            ],
        };
        let context = build_world_context(&snapshot, 2);
        assert_eq!(
            context
                .humans
                .iter()
                .map(|human| human.id)
                .collect::<Vec<_>>(),
            vec![HumanId(1), HumanId(2)]
        );
        assert_eq!(
            context
                .viruses
                .iter()
                .map(|virus| virus.id)
                .collect::<Vec<_>>(),
            vec![VirusId(1), VirusId(2)]
        );
        assert_eq!(
            context
                .recent
                .iter()
                .map(|entry| entry.tick)
                .collect::<Vec<_>>(),
            vec![Tick(2), Tick(3)]
        );
    }

    #[test]
    fn authored_event_json_rejects_a_field_that_the_boundary_does_not_define() {
        let json = r#"{"events":[],"invented":true}"#;
        assert!(serde_json::from_str::<AiEventBatch>(json).is_err());
    }
}
