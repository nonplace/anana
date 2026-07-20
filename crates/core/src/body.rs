use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{Phenotype, VirusId};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
pub enum LifeStage {
    Infant,
    Child,
    Adolescent,
    Adult,
    Elder,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ecs", derive(bevy_ecs::component::Component))]
pub struct Body {
    pub age_ticks: u32,
    pub life_stage: LifeStage,
    pub health: u16,
    pub max_health: u16,
    pub fertility: u8,
    pub immunities: BTreeSet<VirusId>,
    pub alive: bool,
}

impl Body {
    #[must_use]
    pub fn at_birth(phenotype: &Phenotype) -> Self {
        Self {
            age_ticks: 0,
            life_stage: LifeStage::Infant,
            health: phenotype.base_max_health,
            max_health: phenotype.base_max_health,
            fertility: 0,
            immunities: BTreeSet::new(),
            alive: true,
        }
    }

    #[must_use]
    pub fn life_stage_for(age_ticks: u32, lifespan_ticks: u32) -> LifeStage {
        if lifespan_ticks == 0 {
            return LifeStage::Elder;
        }
        let elapsed = u64::from(age_ticks).saturating_mul(1000) / u64::from(lifespan_ticks);
        match elapsed {
            0..=49 => LifeStage::Infant,
            50..=199 => LifeStage::Child,
            200..=349 => LifeStage::Adolescent,
            350..=749 => LifeStage::Adult,
            _ => LifeStage::Elder,
        }
    }
}

#[cfg(test)]
mod tests {
    //! Bodies begin from the expressed phenotype and age through exact integer life-stage boundaries.

    use super::*;
    use crate::{DiseaseStatus, EyeColor, Handedness, Permille, Phenotype, Sex};

    fn phenotype() -> Phenotype {
        Phenotype {
            sex: Sex::Female,
            eye_color: EyeColor::Brown,
            handedness: Handedness::Right,
            disease_x: DiseaseStatus::Clear,
            robustness: 4,
            aptitude: 4,
            base_max_health: 100,
            learning_rate: Permille(700),
            lifespan_ticks: 22_000,
        }
    }

    #[test]
    fn a_newborn_body_takes_its_health_from_the_birth_phenotype() {
        let body = Body::at_birth(&phenotype());
        assert_eq!((body.health, body.max_health), (100, 100));
        assert_eq!(
            (body.age_ticks, body.life_stage, body.fertility),
            (0, LifeStage::Infant, 0)
        );
        assert!(body.immunities.is_empty());
        assert!(body.alive);
    }

    #[test]
    fn every_life_stage_changes_at_its_exact_permille_boundary() {
        let cases = [
            (0, LifeStage::Infant),
            (49, LifeStage::Infant),
            (50, LifeStage::Child),
            (199, LifeStage::Child),
            (200, LifeStage::Adolescent),
            (349, LifeStage::Adolescent),
            (350, LifeStage::Adult),
            (749, LifeStage::Adult),
            (750, LifeStage::Elder),
            (u32::MAX, LifeStage::Elder),
        ];
        for (age, expected) in cases {
            assert_eq!(Body::life_stage_for(age, 1000), expected);
        }
    }

    #[test]
    fn a_zero_lifespan_is_already_elder_rather_than_dividing_by_zero() {
        assert_eq!(Body::life_stage_for(0, 0), LifeStage::Elder);
    }
}
