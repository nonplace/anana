Feature: Standing is given, relationships are bounded, and large groups reorganise
  Standing in this world is given rather than taken. People defer to someone they have watched
  perform competently, and they also use other people's existing deference as a cheaper clue.
  The resulting prestige can compound, but it disappears when followers withdraw it or die.
  Prestige attracts attention, imitation, and liking; it grants no power to coerce anyone.

  A person can actively maintain about one hundred and fifty relationships. Roughly five form
  the closest support circle, fifteen the sympathy circle, fifty an affinity circle, and the
  remainder the active network. Effort is concentrated inward, and a relationship demotes itself
  when contact becomes too rare. Weak ties beyond the capacity fall away deterministically.

  Mutual strong bonds form coalitions. Concentrated standing reduces reciprocity rather than
  creating cooperation. When a residence grows beyond what its members can maintain, dense flat
  bonds lead it to split, while concentrated standing creates mediators and formal structure.

  Scenario: Competence earns standing only when somebody observes it
    Given two equally capable people but only one has been watched
    When a neighbour decides where to confer respect
    Then the observed person receives more standing and the obscure person may remain unknown

  Scenario: Standing compounds and remains revocable
    Given a capable person has an early lead in freely given respect
    When neighbours use both competence and existing respect as clues
    Then the lead grows and falls again when a follower is removed

  Scenario: Prestige cannot coerce
    Given a maximally prestigious person and a neighbour who gave them nothing
    When the prestigious person is considered by that neighbour
    Then the neighbour's body skills bonds and choices remain unchanged

  Scenario: Relationships occupy earned layers and demote without contact
    Given a close relationship and more acquaintances than one mind can maintain
    When contact stops for long enough
    Then the close relationship moves outward and nobody exceeds the social bound

  Scenario: Oversized groups split or grow structure from their own state
    Given an oversized group with bonds and freely conferred standing
    When its members can no longer maintain one uniform network
    Then a flat connected group splits while a steep group grows mediators
    And concentrated standing supports less cooperation than flat standing
