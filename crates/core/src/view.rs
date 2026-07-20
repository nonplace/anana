use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    Body, Consciousness, Genome, HumanId, Infection, Instincts, Lineage, Phenotype, Skills,
};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct HumanState {
    pub id: HumanId,
    pub genome: Genome,
    pub phenotype: Phenotype,
    pub instincts: Instincts,
    pub consciousness: Consciousness,
    pub body: Body,
    pub skills: Skills,
    pub lineage: Lineage,
    pub infection: Option<Infection>,
}

#[derive(Clone, Copy, Debug)]
pub struct WorldView<'a> {
    pub humans: &'a BTreeMap<HumanId, HumanState>,
    pub subjects: &'a [HumanId],
    pub next_human_id: HumanId,
}

#[cfg(test)]
pub(crate) fn fixture_human(id: HumanId) -> HumanState {
    use crate::{
        DiseaseAllele, EyeAllele, GenePair, HandAllele, PolySublocus, PolygenicLocus, Rng,
        SexAllele, Tick, express,
    };

    let zero = PolySublocus::new(0, 0).expect("fixture doses are valid");
    let genome = Genome {
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
            paternal: SexAllele::X,
        },
        robustness: PolygenicLocus { subloci: [zero; 4] },
        aptitude: PolygenicLocus { subloci: [zero; 4] },
    };
    let phenotype = express(&genome, &Rng::new(1), Tick(0), id);
    HumanState {
        id,
        genome,
        body: Body::at_birth(&phenotype),
        phenotype,
        instincts: Instincts {
            survival: 50,
            reproduction: 50,
            hunger: 50,
            fear: 50,
            social: 50,
        },
        consciousness: Consciousness {
            awareness: 100,
            focus: 100,
            memory_capacity: 1000,
        },
        skills: Skills::default(),
        lineage: Lineage::new(id, None, None, 0, Tick(0)),
        infection: None,
    }
}

#[cfg(test)]
mod tests {
    //! World views borrow canonical humans without exposing a mutation path to event resolution.

    use std::collections::BTreeMap;

    use super::*;
    use crate::HumanId;

    #[test]
    fn a_world_view_preserves_sorted_humans_subjects_and_the_next_identifier() {
        let humans = BTreeMap::from([(HumanId(2), fixture_human(HumanId(2)))]);
        let subjects = [HumanId(2)];
        let view = WorldView {
            humans: &humans,
            subjects: &subjects,
            next_human_id: HumanId(3),
        };
        assert_eq!(
            view.humans.keys().copied().collect::<Vec<_>>(),
            vec![HumanId(2)]
        );
        assert_eq!(view.subjects, &subjects);
        assert_eq!(view.next_human_id, HumanId(3));
    }
}
