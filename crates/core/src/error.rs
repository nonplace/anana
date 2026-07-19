use thiserror::Error;

#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum CoreError {
    #[error("allele dose must be zero or one, got {0}")]
    BadDose(u8),
}
