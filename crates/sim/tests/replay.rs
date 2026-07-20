//! These scenarios prove that recorded authored influence can rebuild the exact offline trajectory.

use anana_core::{Boon, GoshKind, HumanId};
use anana_sim::{
    Config, EventIntake, EventLog, HashHistory, WorldClock, build_headless_app, replay, step,
};

#[test]
fn recorded_goshes_replay_to_the_identical_per_tick_hash_history() {
    let config = Config::default();
    let mut original = build_headless_app(84, config.clone());
    original
        .world()
        .resource::<EventIntake>()
        .cast_gosh(
            original.world().resource::<WorldClock>().0,
            GoshKind::Bless {
                subject: HumanId(2),
                boon: Boon::GrantImmunity(config.initial_virus.id),
            },
        )
        .expect("the event intake accepts the first recorded gosh");
    for completed in 0..100 {
        if completed == 50 {
            original
                .world()
                .resource::<EventIntake>()
                .cast_gosh(
                    original.world().resource::<WorldClock>().0,
                    GoshKind::Bless {
                        subject: HumanId(3),
                        boon: Boon::Heal(7),
                    },
                )
                .expect("the event intake accepts the second recorded gosh");
        }
        step(&mut original);
    }

    let history = original.world().resource::<HashHistory>().0.clone();
    let records = original.world().resource::<EventLog>().records().to_vec();
    let replayed = replay(84, config, records);

    assert_eq!(replayed.world().resource::<HashHistory>().0, history);
}
