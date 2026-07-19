use thiserror::Error;

use crate::SkillId;

#[derive(Clone, PartialEq, Eq, Debug, Error)]
pub enum CoreError {
    #[error("allele dose must be zero or one, got {0}")]
    BadDose(u8),
    #[error("skill {0:?} is locked below its awareness threshold")]
    SkillLocked(SkillId),
}
