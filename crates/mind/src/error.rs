use thiserror::Error;

#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum MindError {
    #[error("model transport failed: {0}")]
    Transport(String),
    #[error("model returned non-success status {0}")]
    NonSuccessStatus(u16),
    #[error("model response body was malformed: {0}")]
    MalformedResponse(String),
    #[error("model response violated the schema: {0}")]
    SchemaViolation(String),
    #[error("model named unknown human {0}")]
    UnknownSubject(u64),
    #[error("model batch was rejected: {0}")]
    BatchRejected(String),
}
