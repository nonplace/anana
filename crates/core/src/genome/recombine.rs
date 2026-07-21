use crate::{
    DiseaseAllele, EyeAllele, GenePair, Genome, HandAllele, HumanId, Permille, PolySublocus,
    PolygenicLocus, Rng, RngDomain, SexAllele, Tick,
};

const MUTATION_RATE: Permille = Permille(5);

#[derive(Clone, Copy)]
struct ConceptionKey<'a> {
    rng: &'a Rng,
    tick: Tick,
    child_id: HumanId,
}

#[must_use]
pub fn gamete_allele<A: Copy>(pair: GenePair<A>, pick_maternal: bool) -> A {
    if pick_maternal {
        pair.maternal
    } else {
        pair.paternal
    }
}

fn inherited_allele<A: Copy>(
    pair: GenePair<A>,
    key: ConceptionKey<'_>,
    domain: RngDomain,
    salt: u64,
    flip: fn(A) -> A,
) -> A {
    let inherited = gamete_allele(pair, key.rng.coin(domain, key.tick, key.child_id, salt));
    if key.rng.gate(
        RngDomain::Mutation,
        key.tick,
        key.child_id,
        salt.saturating_add(1),
        MUTATION_RATE,
    ) {
        flip(inherited)
    } else {
        inherited
    }
}

fn flip_eye(allele: EyeAllele) -> EyeAllele {
    match allele {
        EyeAllele::Brown => EyeAllele::Blue,
        EyeAllele::Blue => EyeAllele::Brown,
    }
}

fn flip_hand(allele: HandAllele) -> HandAllele {
    match allele {
        HandAllele::Right => HandAllele::Left,
        HandAllele::Left => HandAllele::Right,
    }
}

fn flip_disease(allele: DiseaseAllele) -> DiseaseAllele {
    match allele {
        DiseaseAllele::Healthy => DiseaseAllele::Risk,
        DiseaseAllele::Risk => DiseaseAllele::Healthy,
    }
}

fn flip_sex(allele: SexAllele) -> SexAllele {
    match allele {
        SexAllele::X => SexAllele::Y,
        SexAllele::Y => SexAllele::X,
    }
}

fn inherit_pair<A: Copy>(
    mother: GenePair<A>,
    father: GenePair<A>,
    key: ConceptionKey<'_>,
    base_salt: u64,
    paternal_domain: RngDomain,
    flip: fn(A) -> A,
) -> GenePair<A> {
    GenePair {
        maternal: inherited_allele(mother, key, RngDomain::Meiosis, base_salt, flip),
        paternal: inherited_allele(
            father,
            key,
            paternal_domain,
            base_salt.saturating_add(2),
            flip,
        ),
    }
}

fn inherit_unmutated_pair<A: Copy>(
    mother: GenePair<A>,
    father: GenePair<A>,
    key: ConceptionKey<'_>,
    base_salt: u64,
) -> GenePair<A> {
    GenePair {
        maternal: gamete_allele(
            mother,
            key.rng
                .coin(RngDomain::Meiosis, key.tick, key.child_id, base_salt),
        ),
        paternal: gamete_allele(
            father,
            key.rng.coin(
                RngDomain::Meiosis,
                key.tick,
                key.child_id,
                base_salt.saturating_add(1),
            ),
        ),
    }
}

fn inherit_sublocus(
    mother: PolySublocus,
    father: PolySublocus,
    key: ConceptionKey<'_>,
    salt: u64,
) -> PolySublocus {
    let maternal_pair = GenePair {
        maternal: mother.maternal.min(1),
        paternal: mother.paternal.min(1),
    };
    let paternal_pair = GenePair {
        maternal: father.maternal.min(1),
        paternal: father.paternal.min(1),
    };
    let flip = |dose: u8| 1_u8.saturating_sub(dose.min(1));
    PolySublocus {
        maternal: inherited_allele(maternal_pair, key, RngDomain::Meiosis, salt, flip),
        paternal: inherited_allele(
            paternal_pair,
            key,
            RngDomain::Meiosis,
            salt.saturating_add(2),
            flip,
        ),
    }
}

fn inherit_polygenic(
    mother: PolygenicLocus,
    father: PolygenicLocus,
    key: ConceptionKey<'_>,
    base_salt: u64,
) -> PolygenicLocus {
    let [m0, m1, m2, m3] = mother.subloci;
    let [f0, f1, f2, f3] = father.subloci;
    PolygenicLocus {
        subloci: [
            inherit_sublocus(m0, f0, key, base_salt),
            inherit_sublocus(m1, f1, key, base_salt + 10),
            inherit_sublocus(m2, f2, key, base_salt + 20),
            inherit_sublocus(m3, f3, key, base_salt + 30),
        ],
    }
}

#[must_use]
pub fn conceive(
    mother: &Genome,
    father: &Genome,
    rng: &Rng,
    tick: Tick,
    child_id: HumanId,
) -> Genome {
    let key = ConceptionKey {
        rng,
        tick,
        child_id,
    };
    Genome {
        eye: inherit_pair(mother.eye, father.eye, key, 0, RngDomain::Meiosis, flip_eye),
        hand: inherit_pair(
            mother.hand,
            father.hand,
            key,
            10,
            RngDomain::Meiosis,
            flip_hand,
        ),
        disease_x: inherit_pair(
            mother.disease_x,
            father.disease_x,
            key,
            20,
            RngDomain::Meiosis,
            flip_disease,
        ),
        sex: inherit_pair(
            mother.sex,
            father.sex,
            key,
            30,
            RngDomain::SexDetermination,
            flip_sex,
        ),
        threat_salience: inherit_unmutated_pair(
            mother.threat_salience,
            father.threat_salience,
            key,
            300,
        ),
        novelty_tolerance: inherit_unmutated_pair(
            mother.novelty_tolerance,
            father.novelty_tolerance,
            key,
            310,
        ),
        robustness: inherit_polygenic(mother.robustness, father.robustness, key, 100),
        aptitude: inherit_polygenic(mother.aptitude, father.aptitude, key, 200),
    }
}

#[cfg(test)]
mod tests {
    //! Conception independently inherits, mutates, and reproduces every parental contribution.

    use super::*;
    use crate::{
        DiseaseAllele, EyeAllele, GenePair, Genome, HandAllele, HumanId, PolySublocus,
        PolygenicLocus, Rng, SexAllele, Tick,
    };

    fn locus(dose: u8) -> PolygenicLocus {
        PolygenicLocus {
            subloci: [PolySublocus::new(dose, dose).expect("valid dose"); 4],
        }
    }

    fn parents() -> (Genome, Genome) {
        let mother = Genome {
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
            threat_salience: GenePair {
                maternal: crate::ThreatSalienceAllele::Low,
                paternal: crate::ThreatSalienceAllele::High,
            },
            novelty_tolerance: GenePair {
                maternal: crate::NoveltyToleranceAllele::Low,
                paternal: crate::NoveltyToleranceAllele::High,
            },
            robustness: PolygenicLocus {
                subloci: [PolySublocus::new(0, 1).expect("valid dose"); 4],
            },
            aptitude: PolygenicLocus {
                subloci: [PolySublocus::new(1, 0).expect("valid dose"); 4],
            },
        };
        let father = Genome {
            eye: GenePair {
                maternal: EyeAllele::Blue,
                paternal: EyeAllele::Blue,
            },
            hand: GenePair {
                maternal: HandAllele::Left,
                paternal: HandAllele::Left,
            },
            disease_x: GenePair {
                maternal: DiseaseAllele::Risk,
                paternal: DiseaseAllele::Risk,
            },
            sex: GenePair {
                maternal: SexAllele::X,
                paternal: SexAllele::Y,
            },
            threat_salience: GenePair {
                maternal: crate::ThreatSalienceAllele::Median,
                paternal: crate::ThreatSalienceAllele::High,
            },
            novelty_tolerance: GenePair {
                maternal: crate::NoveltyToleranceAllele::Median,
                paternal: crate::NoveltyToleranceAllele::High,
            },
            robustness: locus(0),
            aptitude: locus(1),
        };
        (mother, father)
    }

    #[test]
    fn the_gamete_helper_selects_the_requested_parental_side() {
        let pair = GenePair {
            maternal: EyeAllele::Brown,
            paternal: EyeAllele::Blue,
        };
        assert_eq!(gamete_allele(pair, true), EyeAllele::Brown);
        assert_eq!(gamete_allele(pair, false), EyeAllele::Blue);
    }

    #[test]
    fn conception_is_reproducible_and_every_dose_stays_in_range() {
        let (mother, father) = parents();
        let first = conceive(&mother, &father, &Rng::new(41), Tick(8), HumanId(3));
        let second = conceive(&mother, &father, &Rng::new(41), Tick(8), HumanId(3));
        assert_eq!(first, second);
        assert_eq!(first.validate(), Ok(()));
    }

    #[test]
    fn independent_assortment_can_mix_parental_sides_within_one_child() {
        let (mother, father) = parents();
        let mixed = (0..500).any(|seed| {
            let child = conceive(&mother, &father, &Rng::new(seed), Tick(1), HumanId(1));
            let sides = [
                child.eye.maternal == EyeAllele::Brown,
                child.hand.maternal == HandAllele::Right,
                child.disease_x.maternal == DiseaseAllele::Healthy,
            ];
            sides.iter().any(|side| *side) && sides.iter().any(|side| !*side)
        });
        assert!(mixed);
    }

    #[test]
    fn each_perceptual_locus_inherits_one_actual_allele_from_each_parent() {
        let (mother, father) = parents();
        for seed in 0..100 {
            let child = conceive(&mother, &father, &Rng::new(seed), Tick(2), HumanId(4));
            assert!(
                child.threat_salience.maternal == mother.threat_salience.maternal
                    || child.threat_salience.maternal == mother.threat_salience.paternal
            );
            assert!(
                child.threat_salience.paternal == father.threat_salience.maternal
                    || child.threat_salience.paternal == father.threat_salience.paternal
            );
            assert!(
                child.novelty_tolerance.maternal == mother.novelty_tolerance.maternal
                    || child.novelty_tolerance.maternal == mother.novelty_tolerance.paternal
            );
            assert!(
                child.novelty_tolerance.paternal == father.novelty_tolerance.maternal
                    || child.novelty_tolerance.paternal == father.novelty_tolerance.paternal
            );
        }
    }

    #[test]
    fn sex_determination_produces_both_xx_and_xy_near_half_the_time() {
        let (mother, father) = parents();
        let male_count = (0..1000)
            .filter(|seed| {
                let child = conceive(&mother, &father, &Rng::new(*seed), Tick(1), HumanId(1));
                child.sex.maternal == SexAllele::Y || child.sex.paternal == SexAllele::Y
            })
            .count();
        assert!((350..=650).contains(&male_count));
    }

    #[test]
    fn an_authored_mother_with_a_y_contributes_her_actual_alleles() {
        let (mut mother, father) = parents();
        mother.sex = GenePair {
            maternal: SexAllele::Y,
            paternal: SexAllele::X,
        };
        let maternal_y_count = (0..200)
            .filter(|seed| {
                conceive(&mother, &father, &Rng::new(*seed), Tick(1), HumanId(1))
                    .sex
                    .maternal
                    == SexAllele::Y
            })
            .count();
        assert!((50..=150).contains(&maternal_y_count));
    }

    #[test]
    fn a_seeded_mutation_flips_an_allele_and_replays_exactly() {
        let parent = Genome {
            eye: GenePair {
                maternal: EyeAllele::Brown,
                paternal: EyeAllele::Brown,
            },
            hand: GenePair {
                maternal: HandAllele::Right,
                paternal: HandAllele::Right,
            },
            disease_x: GenePair {
                maternal: DiseaseAllele::Healthy,
                paternal: DiseaseAllele::Healthy,
            },
            sex: GenePair {
                maternal: SexAllele::X,
                paternal: SexAllele::X,
            },
            threat_salience: GenePair {
                maternal: crate::ThreatSalienceAllele::Median,
                paternal: crate::ThreatSalienceAllele::Median,
            },
            novelty_tolerance: GenePair {
                maternal: crate::NoveltyToleranceAllele::Median,
                paternal: crate::NoveltyToleranceAllele::Median,
            },
            robustness: locus(0),
            aptitude: locus(0),
        };
        let (seed, mutated) = (0..20_000)
            .find_map(|seed| {
                let child = conceive(&parent, &parent, &Rng::new(seed), Tick(3), HumanId(9));
                (child != parent).then_some((seed, child))
            })
            .expect("the mutation gate should fire in the sample");
        assert_eq!(
            mutated,
            conceive(&parent, &parent, &Rng::new(seed), Tick(3), HumanId(9))
        );
    }
}
