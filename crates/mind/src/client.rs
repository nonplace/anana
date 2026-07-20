use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::{AiEventBatch, LifeHistory, LifeStory, Mind, MindError, OfflineMind, WorldContext};

pub const GPT_MODEL: &str = "gpt-5.6";
pub const DEFAULT_API_ENDPOINT: &str = "https://api.openai.com/v1/responses";
const REQUEST_TIMEOUT_SECONDS: u64 = 30;

pub struct GptMind {
    client: reqwest::Client,
    api_key: String,
    endpoint: String,
}

impl GptMind {
    pub fn new(api_key: impl Into<String>) -> Result<Self, MindError> {
        Self::with_endpoint(api_key, DEFAULT_API_ENDPOINT)
    }

    pub fn with_endpoint(
        api_key: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Result<Self, MindError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
            .build()
            .map_err(|error| MindError::Transport(error.to_string()))?;
        Ok(Self {
            client,
            api_key: api_key.into(),
            endpoint: endpoint.into(),
        })
    }

    async fn send(&self, request: Value) -> Result<String, MindError> {
        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|error| MindError::Transport(error.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            return Err(MindError::NonSuccessStatus(status.as_u16()));
        }
        response
            .text()
            .await
            .map_err(|error| MindError::Transport(error.to_string()))
    }
}

pub enum AnyMind {
    Gpt(GptMind),
    Offline(OfflineMind),
}

impl Mind for AnyMind {
    async fn narrate(&self, history: &LifeHistory) -> Result<LifeStory, MindError> {
        match self {
            Self::Gpt(mind) => mind.narrate(history).await,
            Self::Offline(mind) => mind.narrate(history).await,
        }
    }

    async fn author_events(&self, context: &WorldContext) -> Result<AiEventBatch, MindError> {
        match self {
            Self::Gpt(mind) => mind.author_events(context).await,
            Self::Offline(mind) => mind.author_events(context).await,
        }
    }
}

fn skill_schema() -> Value {
    json!({
        "anyOf": [
            {"type": "string", "enum": ["Recall", "Motor", "Language", "Foraging", "ToolUse", "SocialBond", "Farming", "Medicine", "Planning"]},
            {"type": "null"}
        ]
    })
}

fn author_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["events"],
        "properties": {
            "events": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["subject_id", "kind", "title", "description", "base_prob", "skill_modifier", "modifier_strength", "effects", "seed_salt"],
                    "properties": {
                        "subject_id": {"type": "integer"},
                        "kind": {"type": "string", "enum": ["Chance", "Deterministic"]},
                        "title": {"type": "string"},
                        "description": {"type": "string"},
                        "base_prob": {"anyOf": [{"type": "integer"}, {"type": "null"}]},
                        "skill_modifier": skill_schema(),
                        "modifier_strength": {"anyOf": [{"type": "integer"}, {"type": "null"}]},
                        "effects": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "additionalProperties": false,
                                "required": ["target", "op", "field", "value"],
                                "properties": {
                                    "target": {"type": "integer"},
                                    "op": {"type": "string", "enum": ["Add", "Set"]},
                                    "field": {"type": "string", "enum": ["Health", "Fertility", "AgeTicks", "SkillXp"]},
                                    "value": {"type": "integer"}
                                }
                            }
                        },
                        "seed_salt": {"type": "integer"}
                    }
                }
            }
        }
    })
}

fn narration_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["title", "story", "epitaph"],
        "properties": {
            "title": {"type": "string"},
            "story": {"type": "string"},
            "epitaph": {"type": "string"}
        }
    })
}

fn serialized<T: serde::Serialize>(value: &T) -> Value {
    match serde_json::to_value(value) {
        Ok(value) => value,
        Err(_) => Value::Null,
    }
}

#[must_use]
pub fn build_author_request(context: &WorldContext) -> Value {
    json!({
        "model": GPT_MODEL,
        "input": [{
            "role": "system",
            "content": [{"type": "input_text", "text": "Author a small batch of integer-only world events. Name only humans in the supplied context."}]
        }, {
            "role": "user",
            "content": [{"type": "input_text", "text": serialized(context).to_string()}]
        }],
        "text": {
            "format": {
                "type": "json_schema",
                "name": "anana_event_batch",
                "strict": true,
                "schema": author_schema()
            }
        }
    })
}

#[must_use]
pub fn build_narrate_request(history: &LifeHistory) -> Value {
    json!({
        "model": GPT_MODEL,
        "input": [{
            "role": "system",
            "content": [{"type": "input_text", "text": "Tell the supplied life faithfully. Never invent memories that are absent."}]
        }, {
            "role": "user",
            "content": [{"type": "input_text", "text": serialized(history).to_string()}]
        }],
        "text": {
            "format": {
                "type": "json_schema",
                "name": "anana_life_story",
                "strict": true,
                "schema": narration_schema()
            }
        }
    })
}

fn parse_response<T: DeserializeOwned>(body: &str) -> Result<T, MindError> {
    let value = serde_json::from_str::<Value>(body)
        .map_err(|error| MindError::MalformedResponse(error.to_string()))?;
    match serde_json::from_value::<T>(value.clone()) {
        Ok(parsed) => Ok(parsed),
        Err(direct_error) => {
            let Some(text) = value
                .pointer("/output/0/content/0/text")
                .and_then(Value::as_str)
            else {
                return Err(MindError::SchemaViolation(direct_error.to_string()));
            };
            serde_json::from_str::<T>(text)
                .map_err(|error| MindError::SchemaViolation(error.to_string()))
        }
    }
}

pub fn parse_author_response(body: &str) -> Result<AiEventBatch, MindError> {
    parse_response(body)
}

pub fn parse_narrate_response(body: &str) -> Result<LifeStory, MindError> {
    parse_response(body)
}

impl Mind for GptMind {
    async fn narrate(&self, history: &LifeHistory) -> Result<LifeStory, MindError> {
        let body = self.send(build_narrate_request(history)).await?;
        parse_narrate_response(&body)
    }

    async fn author_events(&self, context: &WorldContext) -> Result<AiEventBatch, MindError> {
        let body = self.send(build_author_request(context)).await?;
        parse_author_response(&body)
    }
}

#[cfg(test)]
mod tests {
    //! Wire construction is stable and canned response parsing reports every schema failure without network access.

    use anana_core::{DiseaseStatus, EyeColor, Handedness, HumanId, LifeStage, Sex, SkillId, Tick};

    use crate::{HumanBrief, LifeHistory, MindError, TraitSummary, WorldContext};

    use super::*;

    fn traits() -> TraitSummary {
        TraitSummary {
            eye_color: EyeColor::Brown,
            handedness: Handedness::Right,
            disease_status: DiseaseStatus::Carrier,
            robustness: 5,
            aptitude: 6,
        }
    }

    fn context() -> WorldContext {
        WorldContext {
            tick: Tick(9),
            humans: vec![HumanBrief {
                id: HumanId(1),
                sex: Sex::Female,
                life_stage: LifeStage::Adult,
                age_ticks: 8_000,
                health: 90,
                max_health: 110,
                notable_traits: traits(),
                top_skills: vec![(SkillId::Planning, 2)],
                infected: None,
            }],
            viruses: Vec::new(),
            recent: Vec::new(),
        }
    }

    fn history() -> LifeHistory {
        LifeHistory {
            subject: HumanId(1),
            sex: Sex::Female,
            life_stage: LifeStage::Adult,
            age_ticks: 8_000,
            generation: 2,
            parents: (None, None),
            children: Vec::new(),
            traits: traits(),
            skills: vec![(SkillId::Recall, 1)],
            recall_learned: true,
            events: Vec::new(),
        }
    }

    #[test]
    fn a_well_formed_canned_author_response_parses_without_network_access() {
        let body = r#"{"events":[{"subject_id":1,"kind":"Chance","title":"Discovery","description":"A lesson","base_prob":200,"skill_modifier":"Planning","modifier_strength":50,"effects":[{"target":1,"op":"Add","field":"SkillXp","value":100}],"seed_salt":7}]}"#;
        let parsed = parse_author_response(body).expect("the canned response follows the schema");
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(parsed.events[0].subject_id, 1);
    }

    #[test]
    fn a_well_formed_canned_narration_response_parses_without_network_access() {
        let body = r#"{"title":"A Life","story":"A remembered life.","epitaph":"Remembered."}"#;
        let parsed = parse_narrate_response(body).expect("the canned response follows the schema");
        assert_eq!(parsed.title, "A Life");
    }

    #[test]
    fn a_responses_envelope_extracts_its_structured_text_before_schema_validation() {
        let body = r#"{"output":[{"content":[{"text":"{\"title\":\"A Life\",\"story\":\"Remembered.\",\"epitaph\":\"Enduring.\"}"}]}]}"#;
        let parsed = parse_narrate_response(body).expect("the envelope contains a valid story");
        assert_eq!(parsed.epitaph, "Enduring.");
    }

    #[test]
    fn malformed_missing_and_wrong_typed_author_bodies_return_errors_instead_of_panicking() {
        assert!(matches!(
            parse_author_response("{"),
            Err(MindError::MalformedResponse(_))
        ));
        for body in [r#"{"events":[{"subject_id":1}]}"#, r#"{"events":"many"}"#] {
            assert!(matches!(
                parse_author_response(body),
                Err(MindError::SchemaViolation(_))
            ));
        }
    }

    #[test]
    fn malformed_missing_and_wrong_typed_narration_bodies_return_errors_instead_of_panicking() {
        assert!(matches!(
            parse_narrate_response("{"),
            Err(MindError::MalformedResponse(_))
        ));
        for body in [
            r#"{"title":"Only a title"}"#,
            r#"{"title":1,"story":"x","epitaph":"y"}"#,
        ] {
            assert!(matches!(
                parse_narrate_response(body),
                Err(MindError::SchemaViolation(_))
            ));
        }
    }

    #[test]
    fn author_and_narration_requests_are_stable_for_identical_inputs() {
        assert_eq!(
            build_author_request(&context()),
            build_author_request(&context())
        );
        assert_eq!(
            build_narrate_request(&history()),
            build_narrate_request(&history())
        );
        assert_eq!(build_author_request(&context())["model"], GPT_MODEL);
        assert_eq!(build_narrate_request(&history())["model"], GPT_MODEL);
    }

    #[tokio::test]
    async fn the_runtime_mind_enum_preserves_the_offline_implementation_contract() {
        let mind = AnyMind::Offline(OfflineMind);
        let first = mind
            .narrate(&history())
            .await
            .expect("offline selection narrates");
        let second = mind
            .narrate(&history())
            .await
            .expect("offline selection repeats");
        assert_eq!(first, second);
    }
}
