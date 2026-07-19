use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub enum SkillId {
    Recall,
    Motor,
    Language,
    Foraging,
    ToolUse,
    SocialBond,
    Farming,
    Medicine,
    Planning,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
pub struct SkillState {
    pub xp: u32,
    pub learned: bool,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
#[cfg_attr(feature = "ecs", derive(bevy_ecs::component::Component))]
pub struct Skills {
    pub levels: BTreeMap<SkillId, SkillState>,
}

impl Skills {
    #[must_use]
    pub fn recall_learned(&self) -> bool {
        self.levels
            .get(&SkillId::Recall)
            .is_some_and(|state| state.learned)
    }

    #[must_use]
    pub fn level_of(&self, id: SkillId) -> u8 {
        let xp = self.levels.get(&id).map_or(0, |state| state.xp);
        match xp {
            0..=99 => 0,
            100..=299 => 1,
            300..=599 => 2,
            600..=999 => 3,
            1000..=1499 => 4,
            _ => 5,
        }
    }
}

#[cfg(test)]
mod tests {
    //! Skill levels are derived from experience, and Recall is derived from its one canonical entry.

    use super::*;

    #[test]
    fn the_full_skill_curve_includes_every_boundary_and_saturates() {
        let cases = [
            (0, 0),
            (99, 0),
            (100, 1),
            (299, 1),
            (300, 2),
            (599, 2),
            (600, 3),
            (999, 3),
            (1000, 4),
            (1499, 4),
            (1500, 5),
            (u32::MAX, 5),
        ];
        for (xp, expected) in cases {
            let mut skills = Skills::default();
            skills
                .levels
                .insert(SkillId::Motor, SkillState { xp, learned: false });
            assert_eq!(skills.level_of(SkillId::Motor), expected);
        }
    }

    #[test]
    fn an_absent_skill_has_level_zero_and_recall_is_derived() {
        let mut skills = Skills::default();
        assert_eq!(skills.level_of(SkillId::Planning), 0);
        assert!(!skills.recall_learned());
        skills.levels.insert(
            SkillId::Recall,
            SkillState {
                xp: 100,
                learned: true,
            },
        );
        assert!(skills.recall_learned());
    }
}
