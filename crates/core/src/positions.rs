use serde::{Deserialize, Serialize};

use crate::Permille;

pub const POSITION_SLOT_COUNT: usize = 8;
pub const MAX_POSITION_STEP: u16 = 100;

/// One deliberately anonymous position slot.
///
/// The index is not a topic. Real positions have content and logical relations to one another;
/// eight unrelated numbers do not. Naming a slot would turn an approximation into a script.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct PositionState {
    pub value: i16,
    pub conviction: Permille,
    pub lifetime_movement: u32,
}

impl Default for PositionState {
    fn default() -> Self {
        Self {
            value: 0,
            conviction: Permille::ZERO,
            lifetime_movement: 0,
        }
    }
}

/// A small anonymous position repertoire plus public lifetime aggregates used for validation.
///
/// Genes are intentionally absent from every position update. Inherited perceptual gains may
/// shape what is retained before this boundary, but no locus may write a position.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ecs", derive(bevy_ecs::component::Component))]
pub struct Positions {
    pub slots: [PositionState; POSITION_SLOT_COUNT],
    pub social_exposure_ticks: u32,
    pub relationship_count_sum: u64,
}

impl Default for Positions {
    fn default() -> Self {
        Self {
            slots: [PositionState::default(); POSITION_SLOT_COUNT],
            social_exposure_ticks: 0,
            relationship_count_sum: 0,
        }
    }
}

impl Positions {
    pub fn record_relationship_count(&mut self, count: usize) {
        self.social_exposure_ticks = self.social_exposure_ticks.saturating_add(1);
        self.relationship_count_sum = self
            .relationship_count_sum
            .saturating_add(u64::try_from(count).unwrap_or(u64::MAX));
    }

    #[must_use]
    pub fn mean_relationships_permille(&self) -> u64 {
        if self.social_exposure_ticks == 0 {
            return 0;
        }
        self.relationship_count_sum.saturating_mul(1_000) / u64::from(self.social_exposure_ticks)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct PositionSignal {
    pub slot: u8,
    pub value: i16,
    pub retention: Permille,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct AttachedPosition {
    pub value: i16,
    pub attachment: Permille,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
pub struct PositionChange {
    pub evidence_pressure: u32,
    pub coalition_cost: u32,
    pub moved_toward: bool,
    pub moved_away: bool,
    pub step: u16,
}

fn agreement(left: i16, right: i16) -> u32 {
    let distance = i32::from(left).abs_diff(i32::from(right)).min(2_000);
    1_000_u32.saturating_sub(distance / 2)
}

fn coalition_cost(current: &PositionState, attached: &[AttachedPosition]) -> u32 {
    if current.conviction == Permille::ZERO {
        return 0;
    }
    attached.iter().fold(0_u32, |sum, neighbour| {
        let cost = u32::from(neighbour.attachment.0.min(1_000))
            .saturating_mul(agreement(current.value, neighbour.value))
            .saturating_div(1_000);
        sum.saturating_add(cost)
    })
}

/// Applies retained information to one anonymous slot.
///
/// Contradiction is attractive when evidence pressure exceeds the attachment cost of leaving
/// agreeing companions. When that cost is larger, the sign reverses and the human moves away.
pub fn receive_position(
    positions: &mut Positions,
    signal: PositionSignal,
    attached: &[AttachedPosition],
    include_coalition_cost: bool,
) -> PositionChange {
    let Some(current) = positions.slots.get_mut(usize::from(signal.slot)) else {
        return PositionChange::default();
    };
    let retention = u32::from(signal.retention.0.min(1_000));
    if retention == 0 {
        return PositionChange::default();
    }
    let incoming = signal.value.clamp(-1_000, 1_000);
    let difference = i32::from(incoming).saturating_sub(i32::from(current.value));
    if difference == 0 {
        current.conviction = Permille::clamp1000(
            i64::from(current.conviction.0).saturating_add(i64::from(retention / 20)),
        );
        return PositionChange::default();
    }
    let evidence_pressure = difference.unsigned_abs().saturating_mul(retention) / 1_000;
    let social_cost = if include_coalition_cost {
        coalition_cost(current, attached)
    } else {
        0
    };
    let net = i64::from(evidence_pressure).saturating_sub(i64::from(social_cost));
    if net == 0 {
        return PositionChange {
            evidence_pressure,
            coalition_cost: social_cost,
            ..PositionChange::default()
        };
    }
    let step = u16::try_from(net.unsigned_abs().min(u64::from(MAX_POSITION_STEP)))
        .unwrap_or(MAX_POSITION_STEP)
        .max(1);
    let toward = difference.signum();
    let direction = if net > 0 { toward } else { -toward };
    let previous = current.value;
    let next = i32::from(current.value)
        .saturating_add(direction.saturating_mul(i32::from(step)))
        .clamp(-1_000, 1_000);
    current.value = i16::try_from(next).unwrap_or(if next < 0 { -1_000 } else { 1_000 });
    current.lifetime_movement = current
        .lifetime_movement
        .saturating_add(u32::from(previous.abs_diff(current.value)));
    current.conviction = Permille::clamp1000(
        i64::from(current.conviction.0).saturating_add(i64::from(step) / 2 + 1),
    );
    PositionChange {
        evidence_pressure,
        coalition_cost: social_cost,
        moved_toward: net > 0,
        moved_away: net < 0,
        step,
    }
}

/// People infer what others do from close contacts, so weak attachments contribute weak evidence.
#[must_use]
pub fn attachment_weighted_observation(observed: u16, attachment: Permille) -> u16 {
    u32::from(observed)
        .saturating_mul(u32::from(attachment.0.min(1_000)))
        .saturating_div(1_000)
        .min(u32::from(u16::MAX)) as u16
}

#[cfg(test)]
mod tests {
    //! Anonymous positions move toward retained evidence when social ties are cheap and away from
    //! the same evidence when agreement with attached people makes changing sides expensive.

    use crate::{
        AttachedPosition, Permille, PositionSignal, Positions, attachment_weighted_observation,
        receive_position,
    };

    #[test]
    fn retained_contradiction_moves_a_weakly_attached_human_toward_it() {
        let mut positions = Positions::default();
        positions.slots[0].value = -600;
        positions.slots[0].conviction = Permille(500);
        let change = receive_position(
            &mut positions,
            PositionSignal {
                slot: 0,
                value: 600,
                retention: Permille::ONE,
            },
            &[],
            true,
        );
        assert!(change.moved_toward);
        assert!(!change.moved_away);
        assert!(positions.slots[0].value > -600);
    }

    #[test]
    fn costly_contradiction_pushes_a_strongly_attached_human_away_from_it() {
        let mut positions = Positions::default();
        positions.slots[0].value = -600;
        positions.slots[0].conviction = Permille(800);
        let allies = [
            AttachedPosition {
                value: -650,
                attachment: Permille(900),
            },
            AttachedPosition {
                value: -700,
                attachment: Permille(900),
            },
        ];
        let change = receive_position(
            &mut positions,
            PositionSignal {
                slot: 0,
                value: 600,
                retention: Permille(500),
            },
            &allies,
            true,
        );
        assert!(change.moved_away);
        assert!(!change.moved_toward);
        assert!(positions.slots[0].value < -600);
    }

    #[test]
    fn a_human_who_retains_nothing_acquires_no_position() {
        let mut positions = Positions::default();
        receive_position(
            &mut positions,
            PositionSignal {
                slot: 3,
                value: 900,
                retention: Permille::ZERO,
            },
            &[],
            true,
        );
        assert_eq!(positions, Positions::default());
    }

    #[test]
    fn disabling_coalition_cost_turns_the_same_social_contradiction_into_attraction() {
        let mut positions = Positions::default();
        positions.slots[0].value = -600;
        positions.slots[0].conviction = Permille(800);
        let allies = [AttachedPosition {
            value: -650,
            attachment: Permille::ONE,
        }];
        let change = receive_position(
            &mut positions,
            PositionSignal {
                slot: 0,
                value: 600,
                retention: Permille::ONE,
            },
            &allies,
            false,
        );
        assert!(change.moved_toward);
    }

    #[test]
    fn one_encounter_cannot_move_a_position_more_than_the_declared_limit() {
        let mut positions = Positions::default();
        positions.slots[0].value = -1_000;
        receive_position(
            &mut positions,
            PositionSignal {
                slot: 0,
                value: 1_000,
                retention: Permille::ONE,
            },
            &[],
            true,
        );
        assert_eq!(positions.slots[0].value, -900);
    }

    #[test]
    fn beliefs_about_other_people_weight_close_observations_more_heavily() {
        assert_eq!(attachment_weighted_observation(800, Permille(900)), 720);
        assert_eq!(attachment_weighted_observation(800, Permille(100)), 80);
    }
}
