Feature: A god can change the world, and the change is permanent and recorded
  Speaking a gosh is the only way a player changes the world. A gosh is a decree, not a
  gamble: it always does exactly what it says, it is written into the world's history, and
  it is still there when that history is replayed. Merely watching the world changes nothing.

  Scenario: Blessing a human heals them
    Given a running world with an injured human
    When the god blesses that human with healing
    Then that human's health has increased
    And the blessing appears in the world's history

  Scenario: A decree is not a gamble
    Given a running world with an injured human
    When the same blessing is spoken in two worlds started from different seeds
    Then the blessing has exactly the same effect in both

  Scenario: The change outlives the moment it was made
    Given a running world where a human has been blessed
    When the world advances 50 ticks
    Then the blessing is still recorded in the world's history

  Scenario: Watching the world never changes it
    Given a running world
    When the god inspects a human without speaking
    Then the world's history is unchanged
