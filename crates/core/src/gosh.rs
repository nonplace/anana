use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    CoreError, EffectSummary, EventOutcome, Genome, GodId, HumanId, Seq, SkillId, Tick, VirusId,
    WorldView, full_learning_gain,
};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct God {
    pub id: GodId,
    pub goshes_spoken: u32,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum GoshKind {
    Bless {
        subject: HumanId,
        boon: Boon,
    },
    Afflict {
        target: GoshTarget,
        bane: Bane,
    },
    Teach {
        subject: HumanId,
        skill: SkillId,
        xp: u32,
    },
    Seed {
        genome: Genome,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum GoshTarget {
    One(HumanId),
    Lineage(HumanId),
    All,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Boon {
    Heal(u16),
    Fertility(u8),
    GrantImmunity(VirusId),
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Bane {
    Harm(u16),
    Infect(VirusId),
}

fn outcome_from_effects(effects: BTreeMap<HumanId, EffectSummary>) -> EventOutcome {
    if effects.is_empty() {
        EventOutcome::NoOp
    } else {
        EventOutcome::Occurred(effects)
    }
}

fn resolve_targets(
    target: GoshTarget,
    view: &WorldView<'_>,
) -> Result<BTreeSet<HumanId>, CoreError> {
    match target {
        GoshTarget::One(id) => {
            if view.humans.contains_key(&id) {
                Ok(BTreeSet::from([id]))
            } else {
                Err(CoreError::EmptyTarget)
            }
        }
        GoshTarget::All => {
            let targets = view.humans.keys().copied().collect::<BTreeSet<_>>();
            if targets.is_empty() {
                Err(CoreError::EmptyTarget)
            } else {
                Ok(targets)
            }
        }
        GoshTarget::Lineage(root) => {
            if !view.humans.contains_key(&root) {
                return Err(CoreError::EmptyTarget);
            }
            let mut found = BTreeSet::new();
            let mut pending = BTreeSet::from([root]);
            while let Some(id) = pending.pop_first() {
                if !found.insert(id) {
                    continue;
                }
                if let Some(human) = view.humans.get(&id) {
                    for child in &human.lineage.children {
                        if view.humans.contains_key(child) && !found.contains(child) {
                            pending.insert(*child);
                        }
                    }
                }
            }
            if found.is_empty() {
                Err(CoreError::EmptyTarget)
            } else {
                Ok(found)
            }
        }
    }
}

pub fn resolve_gosh(
    kind: &GoshKind,
    view: &WorldView<'_>,
    _tick: Tick,
    _seq: Seq,
) -> Result<EventOutcome, CoreError> {
    let mut effects = BTreeMap::new();
    match kind {
        GoshKind::Bless { subject, boon } => {
            let human = view.humans.get(subject).ok_or(CoreError::EmptyTarget)?;
            let mut effect = EffectSummary::default();
            match boon {
                Boon::Heal(amount) => {
                    let missing = human.body.max_health.saturating_sub(human.body.health);
                    effect.health_delta = i32::from((*amount).min(missing));
                }
                Boon::Fertility(amount) => {
                    let headroom = 100_u8.saturating_sub(human.body.fertility.min(100));
                    effect.fertility_delta = i16::from((*amount).min(headroom));
                }
                Boon::GrantImmunity(virus) => {
                    if !human.body.immunities.contains(virus) {
                        effect.immunities_granted.insert(*virus);
                    }
                }
            }
            if !effect.is_empty() {
                effects.insert(*subject, effect);
            }
        }
        GoshKind::Afflict { target, bane } => {
            for id in resolve_targets(*target, view)? {
                let Some(human) = view.humans.get(&id) else {
                    continue;
                };
                let mut effect = EffectSummary::default();
                match bane {
                    Bane::Harm(amount) => {
                        effect.health_delta = -i32::from((*amount).min(human.body.health));
                    }
                    Bane::Infect(virus) => {
                        if !human.body.immunities.contains(virus) {
                            effect.infection = Some(*virus);
                        }
                    }
                }
                if !effect.is_empty() {
                    effects.insert(id, effect);
                }
            }
        }
        GoshKind::Teach { subject, skill, xp } => {
            let human = view.humans.get(subject).ok_or(CoreError::EmptyTarget)?;
            let gain = full_learning_gain(&human.consciousness, &human.phenotype, *skill, *xp)?;
            if gain > 0 {
                let mut effect = EffectSummary::default();
                effect.skill_xp.insert(*skill, gain);
                effects.insert(*subject, effect);
            }
        }
        GoshKind::Seed { genome } => {
            genome.validate()?;
            let effect = EffectSummary {
                seeded_genome: Some(genome.clone()),
                ..EffectSummary::default()
            };
            effects.insert(view.next_human_id, effect);
        }
    }
    Ok(outcome_from_effects(effects))
}

#[cfg(test)]
mod tests {
    //! Divine decrees clamp to real state, walk lineages canonically, and author genes only at creation.

    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        CoreError, EventOutcome, HumanId, PolySublocus, Seq, SkillId, Tick, VirusId, WorldView,
        fixture_human,
    };

    fn family() -> BTreeMap<HumanId, crate::HumanState> {
        let mut root = fixture_human(HumanId(1));
        root.lineage.children.push(HumanId(2));
        let mut child = fixture_human(HumanId(2));
        child.lineage.mother = Some(HumanId(1));
        child.lineage.generation = 1;
        child.lineage.children.push(HumanId(3));
        let mut grandchild = fixture_human(HumanId(3));
        grandchild.lineage.mother = Some(HumanId(2));
        grandchild.lineage.generation = 2;
        BTreeMap::from([
            (HumanId(1), root),
            (HumanId(2), child),
            (HumanId(3), grandchild),
            (HumanId(4), fixture_human(HumanId(4))),
        ])
    }

    fn view<'a>(humans: &'a BTreeMap<HumanId, crate::HumanState>) -> WorldView<'a> {
        WorldView {
            humans,
            subjects: &[],
            next_human_id: HumanId(5),
        }
    }

    #[test]
    fn blessings_clamp_to_the_subjects_actual_health_and_fertility() {
        let mut humans = family();
        let human = humans.get_mut(&HumanId(1)).expect("fixture exists");
        human.body.health = 70;
        human.body.max_health = 80;
        human.body.fertility = 95;
        let EventOutcome::Occurred(healing) = resolve_gosh(
            &GoshKind::Bless {
                subject: HumanId(1),
                boon: Boon::Heal(100),
            },
            &view(&humans),
            Tick(1),
            Seq(1),
        )
        .expect("subject exists") else {
            panic!("healing should occur");
        };
        assert_eq!(healing[&HumanId(1)].health_delta, 10);
        let EventOutcome::Occurred(fertility) = resolve_gosh(
            &GoshKind::Bless {
                subject: HumanId(1),
                boon: Boon::Fertility(100),
            },
            &view(&humans),
            Tick(1),
            Seq(2),
        )
        .expect("subject exists") else {
            panic!("fertility should change");
        };
        assert_eq!(fertility[&HumanId(1)].fertility_delta, 5);
    }

    #[test]
    fn a_lineage_affliction_reaches_the_root_child_and_grandchild_only() {
        let humans = family();
        let EventOutcome::Occurred(effects) = resolve_gosh(
            &GoshKind::Afflict {
                target: GoshTarget::Lineage(HumanId(1)),
                bane: Bane::Harm(7),
            },
            &view(&humans),
            Tick(1),
            Seq(1),
        )
        .expect("lineage exists") else {
            panic!("harm should occur");
        };
        assert_eq!(
            effects.keys().copied().collect::<Vec<_>>(),
            vec![HumanId(1), HumanId(2), HumanId(3)]
        );
    }

    #[test]
    fn an_all_affliction_reaches_every_human() {
        let humans = family();
        let EventOutcome::Occurred(effects) = resolve_gosh(
            &GoshKind::Afflict {
                target: GoshTarget::All,
                bane: Bane::Harm(1),
            },
            &view(&humans),
            Tick(1),
            Seq(1),
        )
        .expect("world is populated") else {
            panic!("harm should occur");
        };
        assert_eq!(effects.len(), 4);
    }

    #[test]
    fn missing_one_and_lineage_targets_are_rejected() {
        let humans = family();
        for target in [
            GoshTarget::One(HumanId(99)),
            GoshTarget::Lineage(HumanId(99)),
        ] {
            assert_eq!(
                resolve_gosh(
                    &GoshKind::Afflict {
                        target,
                        bane: Bane::Harm(1),
                    },
                    &view(&humans),
                    Tick(1),
                    Seq(1),
                ),
                Err(CoreError::EmptyTarget)
            );
        }
    }

    #[test]
    fn immunity_makes_duplicate_blessing_and_infection_no_ops() {
        let mut humans = family();
        humans
            .get_mut(&HumanId(1))
            .expect("fixture exists")
            .body
            .immunities
            .insert(VirusId(8));
        assert_eq!(
            resolve_gosh(
                &GoshKind::Bless {
                    subject: HumanId(1),
                    boon: Boon::GrantImmunity(VirusId(8)),
                },
                &view(&humans),
                Tick(1),
                Seq(1),
            ),
            Ok(EventOutcome::NoOp)
        );
        assert_eq!(
            resolve_gosh(
                &GoshKind::Afflict {
                    target: GoshTarget::One(HumanId(1)),
                    bane: Bane::Infect(VirusId(8)),
                },
                &view(&humans),
                Tick(1),
                Seq(2),
            ),
            Ok(EventOutcome::NoOp)
        );
    }

    #[test]
    fn divine_teaching_respects_awareness_in_both_directions() {
        let mut humans = family();
        humans
            .get_mut(&HumanId(1))
            .expect("fixture exists")
            .consciousness
            .awareness = 4;
        let teach = GoshKind::Teach {
            subject: HumanId(1),
            skill: SkillId::Recall,
            xp: 200,
        };
        assert_eq!(
            resolve_gosh(&teach, &view(&humans), Tick(1), Seq(1)),
            Err(CoreError::SkillLocked(SkillId::Recall))
        );
        humans
            .get_mut(&HumanId(1))
            .expect("fixture exists")
            .consciousness
            .awareness = 5;
        assert!(matches!(
            resolve_gosh(&teach, &view(&humans), Tick(1), Seq(1)),
            Ok(EventOutcome::Occurred(_))
        ));
    }

    #[test]
    fn divine_teaching_records_the_full_unattenuated_gain() {
        let humans = family();
        let EventOutcome::Occurred(effects) = resolve_gosh(
            &GoshKind::Teach {
                subject: HumanId(1),
                skill: SkillId::Motor,
                xp: 200,
            },
            &view(&humans),
            Tick(1),
            Seq(1),
        )
        .expect("skill is unlocked") else {
            panic!("teaching should occur");
        };
        assert_eq!(effects[&HumanId(1)].skill_xp[&SkillId::Motor], 100);
    }

    #[test]
    fn seeding_authors_genes_only_for_the_next_human_identifier() {
        let humans = family();
        let genome = humans[&HumanId(1)].genome.clone();
        let EventOutcome::Occurred(effects) = resolve_gosh(
            &GoshKind::Seed {
                genome: genome.clone(),
            },
            &view(&humans),
            Tick(1),
            Seq(1),
        )
        .expect("genome is valid") else {
            panic!("seeding should occur");
        };
        assert_eq!(
            effects.keys().copied().collect::<Vec<_>>(),
            vec![HumanId(5)]
        );
        assert_eq!(effects[&HumanId(5)].seeded_genome, Some(genome));
    }

    #[test]
    fn seeding_rejects_an_invalid_allele_dose() {
        let humans = family();
        let mut genome = humans[&HumanId(1)].genome.clone();
        genome.robustness.subloci[0] = PolySublocus {
            maternal: 9,
            paternal: 0,
        };
        assert_eq!(
            resolve_gosh(&GoshKind::Seed { genome }, &view(&humans), Tick(1), Seq(1)),
            Err(CoreError::BadDose(9))
        );
    }
}
