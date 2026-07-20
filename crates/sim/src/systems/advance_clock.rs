use bevy::prelude::ResMut;

use crate::WorldClock;

pub(crate) fn advance_clock(mut clock: ResMut<'_, WorldClock>) {
    clock.0.0 = clock.0.0.saturating_add(1);
}
