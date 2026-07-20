use serde::{Deserialize, Serialize};

use crate::{Permille, VirusId};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum InfectionPhase {
    Incubating,
    Infectious,
    Recovered,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ecs", derive(bevy_ecs::component::Component))]
pub struct Infection {
    pub strain: VirusId,
    pub ticks: u32,
    pub severity: u8,
    pub phase: InfectionPhase,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ecs", derive(bevy_ecs::component::Component))]
pub struct Virus {
    pub id: VirusId,
    pub spreadscore: u8,
    pub virulence: u8,
    pub incubation_ticks: u32,
    pub mutation_rate: Permille,
}

#[must_use]
pub fn p_infect(
    virus: &Virus,
    resistance: Permille,
    fear_avoidance: Permille,
    contact: Permille,
    medicine: Permille,
) -> Permille {
    if virus.spreadscore == 0 {
        return Permille::ZERO;
    }
    if virus.spreadscore >= 100 {
        return Permille::ONE;
    }
    Permille(u16::from(virus.spreadscore).saturating_mul(10))
        .and(resistance.complement())
        .and(fear_avoidance.complement())
        .and(Permille(contact.0.min(1000)))
        .and(medicine.complement())
}

#[cfg(test)]
mod tests {
    //! Viral spread has absolute endpoints and monotonic integer-only resistance modifiers.

    use super::*;
    use crate::{Permille, VirusId};

    fn virus(spreadscore: u8) -> Virus {
        Virus {
            id: VirusId(1),
            spreadscore,
            virulence: 40,
            incubation_ticks: 12,
            mutation_rate: Permille(3),
        }
    }

    #[test]
    fn a_dormant_virus_never_spreads_under_maximal_contact() {
        assert_eq!(
            p_infect(
                &virus(0),
                Permille::ZERO,
                Permille::ZERO,
                Permille::ONE,
                Permille::ZERO,
            ),
            Permille::ZERO
        );
    }

    #[test]
    fn an_unresistable_virus_always_spreads_through_maximal_defences() {
        assert_eq!(
            p_infect(
                &virus(100),
                Permille::ONE,
                Permille::ONE,
                Permille::ZERO,
                Permille::ONE,
            ),
            Permille::ONE
        );
        assert_eq!(
            p_infect(
                &virus(u8::MAX),
                Permille::ONE,
                Permille::ONE,
                Permille::ZERO,
                Permille::ONE,
            ),
            Permille::ONE
        );
    }

    #[test]
    fn infection_probability_never_decreases_as_spreadscore_rises() {
        let probabilities = (0..=100)
            .map(|spreadscore| {
                p_infect(
                    &virus(spreadscore),
                    Permille(150),
                    Permille(200),
                    Permille(700),
                    Permille(250),
                )
                .0
            })
            .collect::<Vec<_>>();
        assert!(probabilities.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn resistance_fear_and_medicine_each_reduce_midrange_spread() {
        let baseline = p_infect(
            &virus(50),
            Permille::ZERO,
            Permille::ZERO,
            Permille::ONE,
            Permille::ZERO,
        );
        assert_eq!(baseline, Permille(500));
        assert!(
            p_infect(
                &virus(50),
                Permille(100),
                Permille::ZERO,
                Permille::ONE,
                Permille::ZERO,
            ) < baseline
        );
        assert!(
            p_infect(
                &virus(50),
                Permille::ZERO,
                Permille(100),
                Permille::ONE,
                Permille::ZERO,
            ) < baseline
        );
        assert!(
            p_infect(
                &virus(50),
                Permille::ZERO,
                Permille::ZERO,
                Permille::ONE,
                Permille(100),
            ) < baseline
        );
    }
}
