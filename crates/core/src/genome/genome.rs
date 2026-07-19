use serde::{Deserialize, Serialize};

use crate::{CoreError, DiseaseAllele, EyeAllele, GenePair, HandAllele, SexAllele};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct PolySublocus {
    pub maternal: u8,
    pub paternal: u8,
}

impl PolySublocus {
    pub fn new(maternal: u8, paternal: u8) -> Result<Self, CoreError> {
        if maternal > 1 {
            return Err(CoreError::BadDose(maternal));
        }
        if paternal > 1 {
            return Err(CoreError::BadDose(paternal));
        }
        Ok(Self { maternal, paternal })
    }

    #[must_use]
    pub fn dose(self) -> u8 {
        self.maternal.min(1).saturating_add(self.paternal.min(1))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct PolygenicLocus {
    pub subloci: [PolySublocus; 4],
}

impl PolygenicLocus {
    #[must_use]
    pub fn value(self) -> u8 {
        self.subloci
            .iter()
            .fold(0_u8, |value, sublocus| {
                value.saturating_add(sublocus.dose())
            })
            .min(8)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ecs", derive(bevy_ecs::component::Component))]
pub struct Genome {
    pub eye: GenePair<EyeAllele>,
    pub hand: GenePair<HandAllele>,
    pub disease_x: GenePair<DiseaseAllele>,
    pub sex: GenePair<SexAllele>,
    pub robustness: PolygenicLocus,
    pub aptitude: PolygenicLocus,
}

impl Genome {
    pub fn validate(&self) -> Result<(), CoreError> {
        for sublocus in self
            .robustness
            .subloci
            .iter()
            .chain(self.aptitude.subloci.iter())
        {
            if sublocus.maternal > 1 {
                return Err(CoreError::BadDose(sublocus.maternal));
            }
            if sublocus.paternal > 1 {
                return Err(CoreError::BadDose(sublocus.paternal));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! Polygenic doses stay within their biological range and authored genomes are validated.

    use super::*;
    use crate::{CoreError, EyeAllele, GenePair, HandAllele, SexAllele};

    fn valid_locus() -> PolygenicLocus {
        PolygenicLocus {
            subloci: [PolySublocus::new(0, 1).expect("valid dose"); 4],
        }
    }

    fn valid_genome() -> Genome {
        Genome {
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
            robustness: valid_locus(),
            aptitude: valid_locus(),
        }
    }

    #[test]
    fn a_polygenic_sublocus_accepts_only_zero_or_one() {
        assert_eq!(PolySublocus::new(0, 1).expect("valid dose").dose(), 1);
        assert_eq!(PolySublocus::new(2, 0), Err(CoreError::BadDose(2)));
        assert_eq!(PolySublocus::new(0, 9), Err(CoreError::BadDose(9)));
    }

    #[test]
    fn hostile_polygenic_values_are_defensively_clamped() {
        let hostile = PolySublocus {
            maternal: u8::MAX,
            paternal: u8::MAX,
        };
        assert_eq!(hostile.dose(), 2);
        assert_eq!(
            PolygenicLocus {
                subloci: [hostile; 4]
            }
            .value(),
            8
        );
    }

    #[test]
    fn validation_accepts_a_well_formed_conceived_genome() {
        assert_eq!(valid_genome().validate(), Ok(()));
    }

    #[test]
    fn validation_rejects_an_out_of_range_authored_dose() {
        let mut genome = valid_genome();
        genome.aptitude.subloci[2].paternal = 7;
        assert_eq!(genome.validate(), Err(CoreError::BadDose(7)));
    }
}
