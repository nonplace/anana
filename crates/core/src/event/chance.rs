use serde::{Deserialize, Serialize};

use crate::SkillId;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum ChanceTemplate {
    Accident,
    Discovery,
    Conflict,
    Windfall,
}

/// Names the capability exercised by living through each kind of chance event.
#[must_use]
pub const fn exercised_skill(template: ChanceTemplate) -> SkillId {
    match template {
        ChanceTemplate::Accident => SkillId::Motor,
        ChanceTemplate::Discovery => SkillId::ToolUse,
        ChanceTemplate::Conflict => SkillId::SocialBond,
        ChanceTemplate::Windfall => SkillId::Foraging,
    }
}

#[cfg(test)]
mod tests {
    //! Lived events name the capability their participants actually exercise.

    use super::*;

    #[test]
    fn each_chance_event_exercises_a_distinct_domain_skill() {
        assert_eq!(exercised_skill(ChanceTemplate::Accident), SkillId::Motor);
        assert_eq!(exercised_skill(ChanceTemplate::Discovery), SkillId::ToolUse);
        assert_eq!(
            exercised_skill(ChanceTemplate::Conflict),
            SkillId::SocialBond
        );
        assert_eq!(exercised_skill(ChanceTemplate::Windfall), SkillId::Foraging);
    }
}
