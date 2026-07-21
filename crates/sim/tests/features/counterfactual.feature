Feature: One decree can be compared with the future where it was never spoken
  The world can split at one exact moment into an untouched future and a future changed
  by one decree. People already alive are the same people in both futures; later births
  are different lives and are compared only as totals. The seed, split-world fingerprint,
  decree, and horizon reproduce the comparison exactly.

  Scenario: Silence changes nothing
    Given a world reaches a chosen branch point
    When its future is projected without a decree
    Then the two futures have no differences

  Scenario: Branching does not disturb the untouched future
    Given a world reaches a chosen branch point
    When one future receives a deadly decree
    Then the untouched future matches a world that never branched

  Scenario: Ending a family line removes its future
    Given a living family exists at a branch point
    When a decree ends that entire family line
    Then that person and every descendant present at the split are gone from the changed future
    And that founding line survives only in the untouched future

  Scenario: Four identifying values reproduce the same comparison
    Given a seed, branch world, decree, and horizon
    When that counterfactual is projected twice
    Then both comparisons are byte for byte identical
