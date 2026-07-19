use serde::{Deserialize, Serialize};

/// Heritable animal drives that weight later stochastic choices.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ecs", derive(bevy_ecs::component::Component))]
pub struct Instincts {
    pub survival: u8,
    pub reproduction: u8,
    pub hunger: u8,
    pub fear: u8,
    pub social: u8,
}

#[cfg(test)]
mod tests {
    //! Instincts preserve the five independent inherited drives in canonical state.

    use super::*;

    #[test]
    fn every_animal_drive_round_trips_without_becoming_domain_logic() {
        let instincts = Instincts {
            survival: 91,
            reproduction: 72,
            hunger: 53,
            fear: 34,
            social: 15,
        };
        let bytes = postcard::to_allocvec(&instincts).expect("instincts serialize");
        assert_eq!(
            postcard::from_bytes::<Instincts>(&bytes).expect("instincts deserialize"),
            instincts
        );
    }
}
