Feature: Children inherit genes from two parents and express them once, at birth
  Every child takes one copy of each gene from its mother and one from its father. Which
  of those genes actually show is decided once, at the moment of birth, and never again.

  Scenario: A child takes one gene copy from each parent
    Given a mother and a father with known genes
    When they conceive a child
    Then the child carries one copy from the mother and one from the father at every gene

  Scenario: The same parents and the same seed always produce the same child
    Given a mother and a father with known genes
    When they conceive a child twice from the same seed
    Then both children are genetically identical

  Scenario: A hidden gene is still passed on
    Given a parent who carries the disease gene without showing the disease
    When they pass their genes to a child
    Then the child can still inherit the disease gene

  Scenario: Traits are settled at birth and never re-rolled
    Given a newborn whose traits have been expressed
    When the world advances 50 ticks
    Then the newborn's expressed traits are unchanged
