Feature: The same seed always produces the same world
  Everything that happens grows from a single seed. Two worlds started from the same seed
  live identical lives, tick for tick, forever. Two worlds from different seeds do not.

  Scenario: Two worlds from the same seed stay identical
    Given two worlds both seeded with 42
    When both worlds advance 200 ticks
    Then the two worlds are identical at every tick

  Scenario: Different seeds produce different worlds
    Given a world seeded with 42 and another seeded with 43
    When both worlds advance 200 ticks
    Then the two worlds have diverged

  Scenario: Replaying a recorded history reproduces the same world
    Given a world that has run 100 ticks and recorded its history
    When that history is replayed from the same seed
    Then the replayed world matches the original exactly
