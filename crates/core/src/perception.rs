use serde::{Deserialize, Serialize};

use crate::Permille;

const CLOSE_ATTACHMENT: Permille = Permille(500);

/// A deliberately coarse inherited perceptual gain, expressed in parts per thousand.
///
/// This is an approximation of inherited variation in perception, not a calibrated physiological
/// effect size. Genes may scale perception through this type; they must never write a position,
/// preference, or opinion.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
pub struct PerceptualGain(u16);

impl PerceptualGain {
    pub const LOW: Self = Self(500);
    pub const MEDIAN: Self = Self(1_000);
    pub const HIGH: Self = Self(1_500);

    #[must_use]
    pub const fn new(value: u16) -> Self {
        if value < 500 {
            Self::LOW
        } else if value > 1_500 {
            Self::HIGH
        } else {
            Self(value)
        }
    }

    #[must_use]
    pub const fn value(self) -> u16 {
        self.0
    }
}

#[must_use]
pub fn encode_experience_magnitude(
    magnitude: u32,
    is_bad: bool,
    threat_salience: PerceptualGain,
) -> u32 {
    if !is_bad {
        return magnitude;
    }
    u64::from(magnitude)
        .saturating_mul(u64::from(threat_salience.value()))
        .saturating_div(1_000)
        .min(u64::from(u32::MAX)) as u32
}

#[must_use]
pub fn unfamiliar_attention(
    attention: Permille,
    is_kin: bool,
    attachment: Permille,
    novelty_tolerance: PerceptualGain,
) -> Permille {
    if is_kin || attachment >= CLOSE_ATTACHMENT {
        return Permille(attention.0.min(1_000));
    }
    Permille::clamp1000(
        i64::from(attention.0.min(1_000)).saturating_mul(i64::from(novelty_tolerance.value()))
            / 1_000,
    )
}

#[cfg(test)]
mod tests {
    //! Perceptual gains alter only how experience and unfamiliar models are perceived.

    use crate::{PerceptualGain, Permille};

    use super::{encode_experience_magnitude, unfamiliar_attention};

    #[test]
    fn threat_salience_scales_bad_experience_before_memory_but_leaves_good_experience_alone() {
        assert_eq!(
            encode_experience_magnitude(200, true, PerceptualGain::new(1_500)),
            300
        );
        assert_eq!(
            encode_experience_magnitude(200, false, PerceptualGain::new(1_500)),
            200
        );
    }

    #[test]
    fn novelty_tolerance_scales_attention_only_for_unfamiliar_non_kin() {
        let tolerant = PerceptualGain::new(1_500);
        assert_eq!(
            unfamiliar_attention(Permille(400), false, Permille(200), tolerant),
            Permille(600)
        );
        assert_eq!(
            unfamiliar_attention(Permille(400), true, Permille(200), tolerant),
            Permille(400)
        );
        assert_eq!(
            unfamiliar_attention(Permille(400), false, Permille(800), tolerant),
            Permille(400)
        );
    }

    #[test]
    fn perceptual_gains_clamp_authored_values_to_the_approximation_range() {
        assert_eq!(PerceptualGain::new(0).value(), 500);
        assert_eq!(PerceptualGain::new(1_000).value(), 1_000);
        assert_eq!(PerceptualGain::new(u16::MAX).value(), 1_500);
    }
}
