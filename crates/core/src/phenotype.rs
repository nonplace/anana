use serde::{Deserialize, Serialize};

use crate::{PerceptualGain, Permille};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
pub enum Sex {
    Female,
    Male,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
pub enum EyeColor {
    Brown,
    Blue,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
pub enum Handedness {
    Right,
    Left,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
pub enum DiseaseStatus {
    Clear,
    Carrier,
    Affected,
}

/// The immutable phenotype expressed once and stored at birth.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ecs", derive(bevy_ecs::component::Component))]
pub struct Phenotype {
    pub sex: Sex,
    pub eye_color: EyeColor,
    pub handedness: Handedness,
    pub disease_x: DiseaseStatus,
    /// An inherited perceptual gain, not an opinion or preference.
    pub threat_salience: PerceptualGain,
    /// An inherited perceptual gain, not an opinion or preference.
    pub novelty_tolerance: PerceptualGain,
    pub robustness: u8,
    pub aptitude: u8,
    pub base_max_health: u16,
    pub learning_rate: Permille,
    pub lifespan_ticks: u32,
}

#[cfg(test)]
mod tests {
    //! A stored phenotype is a serializable birth record rather than a live genetic calculation.

    use super::*;
    use crate::Permille;

    #[test]
    fn a_birth_phenotype_round_trips_without_reexpression() {
        let phenotype = Phenotype {
            sex: Sex::Female,
            eye_color: EyeColor::Brown,
            handedness: Handedness::Right,
            disease_x: DiseaseStatus::Carrier,
            threat_salience: PerceptualGain::MEDIAN,
            novelty_tolerance: PerceptualGain::MEDIAN,
            robustness: 4,
            aptitude: 5,
            base_max_health: 100,
            learning_rate: Permille(750),
            lifespan_ticks: 22_000,
        };
        let bytes = postcard::to_allocvec(&phenotype).expect("phenotype serializes");
        assert_eq!(
            postcard::from_bytes::<Phenotype>(&bytes).expect("phenotype deserializes"),
            phenotype
        );
    }
}
