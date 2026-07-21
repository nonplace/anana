use crate::{
    DiseaseAllele, DiseaseStatus, EyeAllele, EyeColor, Genome, HandAllele, Handedness, HumanId,
    NoveltyToleranceAllele, PerceptualGain, Permille, Phenotype, Rng, RngDomain, Sex, SexAllele,
    ThreatSalienceAllele, Tick,
};

fn allele_gain(value: u8) -> u16 {
    500_u16.saturating_add(u16::from(value.min(2)).saturating_mul(500))
}

fn threat_allele_gain(allele: ThreatSalienceAllele) -> u16 {
    allele_gain(match allele {
        ThreatSalienceAllele::Low => 0,
        ThreatSalienceAllele::Median => 1,
        ThreatSalienceAllele::High => 2,
    })
}

fn novelty_allele_gain(allele: NoveltyToleranceAllele) -> u16 {
    allele_gain(match allele {
        NoveltyToleranceAllele::Low => 0,
        NoveltyToleranceAllele::Median => 1,
        NoveltyToleranceAllele::High => 2,
    })
}

fn expressed_gain<A: Copy>(pair: crate::GenePair<A>, allele_gain: fn(A) -> u16) -> PerceptualGain {
    let total =
        u32::from(allele_gain(pair.maternal)).saturating_add(u32::from(allele_gain(pair.paternal)));
    PerceptualGain::new((total / 2).min(u32::from(u16::MAX)) as u16)
}

/// Expresses and fixes a phenotype exactly once at birth.
///
/// Callers must store the result and never re-express a living human, because penetrance is keyed
/// by the birth tick and changing that key would change an already-living human.
#[must_use]
pub fn express(genome: &Genome, rng: &Rng, tick: Tick, id: HumanId) -> Phenotype {
    let eye_color =
        if genome.eye.maternal == EyeAllele::Brown || genome.eye.paternal == EyeAllele::Brown {
            EyeColor::Brown
        } else {
            EyeColor::Blue
        };
    let handedness =
        if genome.hand.maternal == HandAllele::Right || genome.hand.paternal == HandAllele::Right {
            Handedness::Right
        } else {
            Handedness::Left
        };
    let sex = if genome.sex.maternal == SexAllele::Y || genome.sex.paternal == SexAllele::Y {
        Sex::Male
    } else {
        Sex::Female
    };
    let disease_x = match (genome.disease_x.maternal, genome.disease_x.paternal) {
        (DiseaseAllele::Healthy, DiseaseAllele::Healthy) => DiseaseStatus::Clear,
        (DiseaseAllele::Risk, DiseaseAllele::Risk) => {
            if rng.gate(RngDomain::Penetrance, tick, id, 0, Permille(750)) {
                DiseaseStatus::Affected
            } else {
                DiseaseStatus::Carrier
            }
        }
        _ => DiseaseStatus::Carrier,
    };
    let robustness = genome.robustness.value().min(8);
    let aptitude = genome.aptitude.value().min(8);
    let threat_salience = expressed_gain(genome.threat_salience, threat_allele_gain);
    let novelty_tolerance = expressed_gain(genome.novelty_tolerance, novelty_allele_gain);
    Phenotype {
        sex,
        eye_color,
        handedness,
        disease_x,
        threat_salience,
        novelty_tolerance,
        robustness,
        aptitude,
        base_max_health: 80_u16.saturating_add(u16::from(robustness).saturating_mul(5)),
        learning_rate: Permille(500_u16.saturating_add(u16::from(aptitude).saturating_mul(50))),
        lifespan_ticks: 2_000_u32.saturating_add(u32::from(robustness).saturating_mul(100)),
    }
}

#[cfg(test)]
mod tests {
    //! Birth expression follows dominance, seeded penetrance, sex alleles, and bounded polygenic traits.

    use super::*;
    use crate::{
        DiseaseAllele, DiseaseStatus, EyeAllele, EyeColor, GenePair, Genome, HandAllele,
        Handedness, HumanId, PolySublocus, PolygenicLocus, Rng, Sex, SexAllele, Tick,
    };

    fn locus(value: u8) -> PolygenicLocus {
        let maternal = u8::from(value > 4);
        let paternal = u8::from(value > 0);
        let mut remaining = value.min(8);
        let mut subloci = [PolySublocus {
            maternal: 0,
            paternal: 0,
        }; 4];
        for sublocus in &mut subloci {
            sublocus.maternal = maternal.min(remaining);
            remaining = remaining.saturating_sub(sublocus.maternal);
            sublocus.paternal = paternal.min(remaining);
            remaining = remaining.saturating_sub(sublocus.paternal);
        }
        PolygenicLocus { subloci }
    }

    fn genome() -> Genome {
        Genome {
            eye: GenePair {
                maternal: EyeAllele::Blue,
                paternal: EyeAllele::Blue,
            },
            hand: GenePair {
                maternal: HandAllele::Left,
                paternal: HandAllele::Left,
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
                maternal: ThreatSalienceAllele::Median,
                paternal: ThreatSalienceAllele::Median,
            },
            novelty_tolerance: GenePair {
                maternal: NoveltyToleranceAllele::Median,
                paternal: NoveltyToleranceAllele::Median,
            },
            robustness: locus(0),
            aptitude: locus(0),
        }
    }

    #[test]
    fn brown_eye_dominance_covers_every_genotype_ordering() {
        let cases = [
            (EyeAllele::Brown, EyeAllele::Brown, EyeColor::Brown),
            (EyeAllele::Brown, EyeAllele::Blue, EyeColor::Brown),
            (EyeAllele::Blue, EyeAllele::Brown, EyeColor::Brown),
            (EyeAllele::Blue, EyeAllele::Blue, EyeColor::Blue),
        ];
        for (maternal, paternal, expected) in cases {
            let mut genome = genome();
            genome.eye = GenePair { maternal, paternal };
            assert_eq!(
                express(&genome, &Rng::new(1), Tick(0), HumanId(1)).eye_color,
                expected
            );
        }
    }

    #[test]
    fn right_handed_dominance_covers_every_genotype_ordering() {
        let cases = [
            (HandAllele::Right, HandAllele::Right, Handedness::Right),
            (HandAllele::Right, HandAllele::Left, Handedness::Right),
            (HandAllele::Left, HandAllele::Right, Handedness::Right),
            (HandAllele::Left, HandAllele::Left, Handedness::Left),
        ];
        for (maternal, paternal, expected) in cases {
            let mut genome = genome();
            genome.hand = GenePair { maternal, paternal };
            assert_eq!(
                express(&genome, &Rng::new(1), Tick(0), HumanId(1)).handedness,
                expected
            );
        }
    }

    #[test]
    fn homozygous_recessive_traits_remain_visible_and_heritable() {
        let phenotype = express(&genome(), &Rng::new(1), Tick(0), HumanId(1));
        assert_eq!(phenotype.eye_color, EyeColor::Blue);
        assert_eq!(phenotype.handedness, Handedness::Left);
        assert_eq!(phenotype.disease_x, DiseaseStatus::Clear);
    }

    #[test]
    fn either_y_allele_expresses_male_sex() {
        let mut genome = genome();
        genome.sex.maternal = SexAllele::Y;
        assert_eq!(
            express(&genome, &Rng::new(1), Tick(0), HumanId(1)).sex,
            Sex::Male
        );
    }

    #[test]
    fn incomplete_penetrance_is_seeded_and_produces_both_outcomes() {
        let mut genome = genome();
        genome.disease_x = GenePair {
            maternal: DiseaseAllele::Risk,
            paternal: DiseaseAllele::Risk,
        };
        let outcomes = (0..100)
            .map(|seed| express(&genome, &Rng::new(seed), Tick(7), HumanId(3)).disease_x)
            .collect::<std::collections::BTreeSet<_>>();
        assert!(outcomes.contains(&DiseaseStatus::Affected));
        assert!(outcomes.contains(&DiseaseStatus::Carrier));
        assert_eq!(
            express(&genome, &Rng::new(88), Tick(7), HumanId(3)),
            express(&genome, &Rng::new(88), Tick(7), HumanId(3))
        );
    }

    #[test]
    fn polygenic_bounds_derive_health_learning_and_lifespan_integers() {
        let low = express(&genome(), &Rng::new(1), Tick(0), HumanId(1));
        assert_eq!((low.robustness, low.aptitude), (0, 0));
        assert_eq!(
            (low.base_max_health, low.learning_rate.0, low.lifespan_ticks),
            (80, 500, 2_000)
        );

        let mut high_genome = genome();
        high_genome.robustness = locus(8);
        high_genome.aptitude = locus(8);
        let high = express(&high_genome, &Rng::new(1), Tick(0), HumanId(1));
        assert_eq!((high.robustness, high.aptitude), (8, 8));
        assert_eq!(
            (
                high.base_max_health,
                high.learning_rate.0,
                high.lifespan_ticks
            ),
            (120, 900, 2_800)
        );
    }

    #[test]
    fn hostile_polygenic_doses_clamp_before_derived_values() {
        let mut hostile = genome();
        hostile.robustness = PolygenicLocus {
            subloci: [PolySublocus {
                maternal: u8::MAX,
                paternal: u8::MAX,
            }; 4],
        };
        hostile.aptitude = hostile.robustness;
        let phenotype = express(&hostile, &Rng::new(1), Tick(0), HumanId(1));
        assert_eq!((phenotype.robustness, phenotype.aptitude), (8, 8));
        assert_eq!(
            (
                phenotype.base_max_health,
                phenotype.learning_rate.0,
                phenotype.lifespan_ticks
            ),
            (120, 900, 2_800)
        );
    }

    #[test]
    fn perceptual_loci_express_additively_across_the_full_parts_per_thousand_range() {
        let mut low = genome();
        low.threat_salience = GenePair {
            maternal: crate::ThreatSalienceAllele::Low,
            paternal: crate::ThreatSalienceAllele::Low,
        };
        low.novelty_tolerance = GenePair {
            maternal: crate::NoveltyToleranceAllele::Low,
            paternal: crate::NoveltyToleranceAllele::Low,
        };
        let low = express(&low, &Rng::new(1), Tick(0), HumanId(1));
        assert_eq!(
            (low.threat_salience.value(), low.novelty_tolerance.value()),
            (500, 500)
        );

        let mut high = genome();
        high.threat_salience = GenePair {
            maternal: crate::ThreatSalienceAllele::High,
            paternal: crate::ThreatSalienceAllele::High,
        };
        high.novelty_tolerance = GenePair {
            maternal: crate::NoveltyToleranceAllele::Median,
            paternal: crate::NoveltyToleranceAllele::High,
        };
        let high = express(&high, &Rng::new(1), Tick(0), HumanId(1));
        assert_eq!(
            (high.threat_salience.value(), high.novelty_tolerance.value()),
            (1_500, 1_250)
        );
    }
}
