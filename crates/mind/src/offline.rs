use anana_core::{LifeStage, SkillId};

use crate::{
    AiEffect, AiEventBatch, AiEventKind, AiEventProposal, AiField, AiOp, LifeHistory, LifeStory,
    Mind, MindError, WorldContext,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct OfflineMind;

fn history_number(history: &LifeHistory) -> u64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&history.subject.0.to_le_bytes());
    hasher.update(&history.age_ticks.to_le_bytes());
    hasher.update(&history.generation.to_le_bytes());
    hasher.update(&[history.life_stage as u8, u8::from(history.recall_learned)]);
    for (skill, level) in &history.skills {
        hasher.update(&[*skill as u8, *level]);
    }
    for event in &history.events {
        hasher.update(&event.tick.0.to_le_bytes());
        hasher.update(&event.seq.0.to_le_bytes());
        hasher.update(event.summary.as_bytes());
    }
    hash_number(hasher.finalize())
}

fn context_number(context: &WorldContext) -> u64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&context.tick.0.to_le_bytes());
    for human in &context.humans {
        hasher.update(&human.id.0.to_le_bytes());
        hasher.update(&human.age_ticks.to_le_bytes());
        hasher.update(&human.health.to_le_bytes());
        hasher.update(&[human.life_stage as u8]);
    }
    for virus in &context.viruses {
        hasher.update(&virus.id.0.to_le_bytes());
        hasher.update(&[virus.spreadscore, virus.virulence]);
    }
    hash_number(hasher.finalize())
}

fn hash_number(hash: blake3::Hash) -> u64 {
    hash.as_bytes()
        .iter()
        .take(8)
        .enumerate()
        .fold(0_u64, |number, (shift, byte)| {
            number | (u64::from(*byte) << shift.saturating_mul(8))
        })
}

fn stage_name(stage: LifeStage) -> &'static str {
    match stage {
        LifeStage::Infant => "infant",
        LifeStage::Child => "child",
        LifeStage::Adolescent => "adolescent",
        LifeStage::Adult => "adult",
        LifeStage::Elder => "elder",
    }
}

fn skill_names(history: &LifeHistory) -> String {
    let names = history
        .skills
        .iter()
        .filter(|(_, level)| *level > 0)
        .map(|(skill, level)| format!("{skill:?} level {level}"))
        .collect::<Vec<_>>();
    if names.is_empty() {
        String::from("no practiced skill yet")
    } else {
        names.join(", ")
    }
}

impl Mind for OfflineMind {
    async fn narrate(&self, history: &LifeHistory) -> Result<LifeStory, MindError> {
        let number = history_number(history);
        let title = if history.recall_learned {
            format!("The Remembered Life of Human {}", history.subject.0)
        } else {
            format!("Human {} Before Memory", history.subject.0)
        };
        let foundation = format!(
            "Generation {} reached the {} stage with {:?} eyes, {:?} handedness, {:?} disease status, robustness {}, aptitude {}, and {}.",
            history.generation,
            stage_name(history.life_stage),
            history.traits.eye_color,
            history.traits.handedness,
            history.traits.disease_status,
            history.traits.robustness,
            history.traits.aptitude,
            skill_names(history),
        );
        let memory = if history.recall_learned {
            let remembered = history
                .events
                .iter()
                .take(2)
                .map(|entry| entry.summary.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            if remembered.is_empty() {
                String::from(" Recall was learned, though no named event has yet entered memory.")
            } else {
                format!(" Their remembered history includes: {remembered}.")
            }
        } else {
            String::from(
                " Recall has not been learned, so experience leaves no accessible personal history.",
            )
        };
        let epitaph = match number % 3 {
            0 => "What endured became part of the world.",
            1 => "A life measured in choices, kin, and memory.",
            _ => "The seed remembers what the mind could hold.",
        };
        Ok(LifeStory {
            title,
            story: foundation + &memory,
            epitaph: String::from(epitaph),
        })
    }

    async fn author_events(&self, context: &WorldContext) -> Result<AiEventBatch, MindError> {
        let number = context_number(context);
        let count = u64::try_from(context.humans.len()).map_or(u64::MAX, |value| value);
        if count == 0 {
            return Ok(AiEventBatch::default());
        }
        let index = usize::try_from(number % count).map_or(0, |value| value);
        let Some(human) = context.humans.get(index) else {
            return Ok(AiEventBatch::default());
        };
        let (title, description) = match number % 3 {
            0 => (
                "A narrow escape",
                "A sudden danger tests practiced judgment",
            ),
            1 => (
                "An unexpected lesson",
                "Ordinary work reveals a useful pattern",
            ),
            _ => (
                "A difficult crossing",
                "Circumstance puts health and planning under strain",
            ),
        };
        let field = if number.is_multiple_of(3) {
            AiField::SkillXp
        } else {
            AiField::Health
        };
        let value = if field == AiField::SkillXp { 100 } else { -5 };
        Ok(AiEventBatch {
            events: vec![AiEventProposal {
                subject_id: human.id.0,
                kind: AiEventKind::Chance,
                title: String::from(title),
                description: String::from(description),
                base_prob: Some(200 + i64::try_from(number % 200).map_or(0, |value| value)),
                skill_modifier: Some(SkillId::Planning),
                modifier_strength: Some(25),
                effects: vec![AiEffect {
                    target: human.id.0,
                    op: AiOp::Add,
                    field,
                    value,
                }],
                seed_salt: (number & u64::from(u32::MAX)) as u32,
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    //! The offline mind produces repeatable stories and safe authored events without network, clock, or randomness.

    use anana_core::{
        DiseaseStatus, EventAuthor, EyeColor, Handedness, HumanId, LifeStage, Seq, Sex, SkillId,
        Tick,
    };

    use crate::{
        HistoryEntry, HumanBrief, LifeHistory, Mind, TraitSummary, VirusBrief, WorldContext,
        validate,
    };

    use super::*;

    fn history(recall_learned: bool) -> LifeHistory {
        LifeHistory {
            subject: HumanId(7),
            sex: Sex::Female,
            life_stage: LifeStage::Adult,
            age_ticks: 8_000,
            generation: 3,
            parents: (Some(HumanId(2)), Some(HumanId(3))),
            children: vec![HumanId(9)],
            traits: TraitSummary {
                eye_color: EyeColor::Brown,
                handedness: Handedness::Left,
                disease_status: DiseaseStatus::Carrier,
                robustness: 6,
                aptitude: 5,
            },
            skills: vec![(SkillId::Recall, 1), (SkillId::Medicine, 2)],
            recall_learned,
            events: vec![HistoryEntry {
                tick: Tick(44),
                seq: Seq(2),
                author: EventAuthor::Engine,
                summary: String::from("The unmistakable remembered discovery"),
            }],
        }
    }

    fn context() -> WorldContext {
        WorldContext {
            tick: Tick(90),
            humans: vec![HumanBrief {
                id: HumanId(7),
                sex: Sex::Female,
                life_stage: LifeStage::Adult,
                age_ticks: 8_000,
                health: 70,
                max_health: 110,
                notable_traits: history(true).traits,
                top_skills: vec![(SkillId::Medicine, 2)],
                infected: None,
            }],
            viruses: vec![VirusBrief {
                id: anana_core::VirusId(1),
                spreadscore: 30,
                virulence: 10,
            }],
            recent: Vec::new(),
        }
    }

    #[tokio::test]
    async fn offline_narration_is_identical_across_calls_and_fresh_instances() {
        let first = OfflineMind
            .narrate(&history(true))
            .await
            .expect("offline narration succeeds");
        let repeated = OfflineMind
            .narrate(&history(true))
            .await
            .expect("offline narration repeats");
        assert_eq!(first, repeated);
    }

    #[tokio::test]
    async fn an_amnesic_offline_story_names_the_missing_recall_without_inventing_events() {
        let story = OfflineMind
            .narrate(&history(false))
            .await
            .expect("offline narration succeeds");
        assert!(story.story.contains("Recall"));
        assert!(!story.story.contains("unmistakable remembered discovery"));
    }

    #[tokio::test]
    async fn offline_authorship_is_deterministic_and_every_proposal_validates() {
        let world = context();
        let first = OfflineMind
            .author_events(&world)
            .await
            .expect("offline authorship succeeds");
        let repeated = OfflineMind
            .author_events(&world)
            .await
            .expect("offline authorship repeats");
        assert_eq!(first, repeated);
        assert_eq!(
            validate(&first, &world)
                .expect("offline proposals are valid")
                .len(),
            first.events.len()
        );
    }
}
