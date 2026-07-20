use serde::{Deserialize, Serialize};

use crate::Permille;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct ObservationFactors {
    pub attention: Permille,
    pub retention: Permille,
    pub reproduction: Permille,
    pub motivation: Permille,
}

#[must_use]
pub fn observational_gain(
    direct_gain: u32,
    model_competence: u16,
    observer_competence: u16,
    factors: ObservationFactors,
) -> u32 {
    if direct_gain == 0 || model_competence <= observer_competence {
        return 0;
    }
    let stages = factors
        .attention
        .and(factors.retention)
        .and(factors.reproduction)
        .and(factors.motivation);
    let competence_gap = model_competence.saturating_sub(observer_competence);
    let model_value = 500_u64.saturating_add(u64::from(competence_gap).saturating_mul(5).min(500));
    u64::from(direct_gain)
        .saturating_mul(u64::from(stages.0.min(1000)))
        .saturating_mul(model_value)
        .saturating_div(2_000_000)
        .min(u64::from(direct_gain.saturating_sub(1))) as u32
}

#[must_use]
pub fn optimal_teaching_gap(learner_competence: u16) -> u16 {
    10_u16.saturating_add(learner_competence.min(100).saturating_mul(3) / 10)
}

#[must_use]
pub fn teaching_gain(learner_competence: u16, teacher_competence: u16, base_gain: u32) -> u32 {
    if teacher_competence <= learner_competence || base_gain == 0 {
        return 0;
    }
    let gap = teacher_competence
        .saturating_sub(learner_competence)
        .min(100);
    if gap >= 100 {
        return 0;
    }
    let peak = optimal_teaching_gap(learner_competence).min(99);
    let factor = if gap <= peak {
        u64::from(gap).saturating_mul(1000) / u64::from(peak.max(1))
    } else {
        u64::from(100_u16.saturating_sub(gap)).saturating_mul(1000)
            / u64::from(100_u16.saturating_sub(peak).max(1))
    };
    u64::from(base_gain)
        .saturating_mul(factor)
        .saturating_div(1000)
        .min(u64::from(u32::MAX)) as u32
}

#[cfg(test)]
mod tests {
    //! Observation follows Bandura's four stages, while teaching peaks inside a moving zone of proximal development.

    use super::*;

    fn complete_observation() -> ObservationFactors {
        ObservationFactors {
            attention: Permille::ONE,
            retention: Permille::ONE,
            reproduction: Permille::ONE,
            motivation: Permille::ONE,
        }
    }

    #[test]
    fn failure_at_any_observational_stage_prevents_learning() {
        for missing in 0..4 {
            let mut factors = complete_observation();
            match missing {
                0 => factors.attention = Permille::ZERO,
                1 => factors.retention = Permille::ZERO,
                2 => factors.reproduction = Permille::ZERO,
                _ => factors.motivation = Permille::ZERO,
            }
            assert_eq!(observational_gain(100, 80, 20, factors), 0);
        }
    }

    #[test]
    fn observation_is_weaker_than_direct_practice_and_grows_with_model_competence() {
        let modest = observational_gain(100, 40, 20, complete_observation());
        let expert = observational_gain(100, 80, 20, complete_observation());
        assert!(modest > 0);
        assert!(expert > modest);
        assert!(expert < 100);
    }

    #[test]
    fn teaching_peaks_at_a_moderate_gap_for_beginners_and_competent_learners() {
        for learner in [0, 60] {
            let peak = optimal_teaching_gap(learner);
            let at_peak = teaching_gain(learner, learner.saturating_add(peak), 1_000);
            assert_eq!(teaching_gain(learner, learner, 1_000), 0);
            assert!(at_peak > teaching_gain(learner, learner.saturating_add(peak - 1), 1_000));
            assert!(at_peak > teaching_gain(learner, learner.saturating_add(peak + 1), 1_000));
            assert_eq!(
                teaching_gain(learner, learner.saturating_add(100), 1_000),
                0
            );
        }
    }

    #[test]
    fn the_best_teaching_gap_moves_outward_as_the_learner_becomes_competent() {
        assert!(optimal_teaching_gap(80) > optimal_teaching_gap(10));
    }
}
