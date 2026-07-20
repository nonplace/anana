Feature: A human must learn to remember before experience compounds
  Before a human learns Recall they live without accessible memory. What they practise
  fades instead of building up, and nothing can be truly learned. Learning Recall brings
  memory online, and from then on experience compounds.

  Scenario: Recall gates skill retention
    Given a newborn who has not learned Recall
    When the world advances 20 ticks of practice
    Then their skill experience decays instead of accumulating
    And no skill has been marked as learned

  Scenario: Learning Recall brings memory online
    Given a human who has just learned Recall
    When the world advances 20 ticks of practice
    Then their skill experience accumulates
    And a practised skill can be marked as learned

  Scenario: A mind too young cannot learn at all
    Given a human whose awareness is below the threshold for Recall
    When they try to learn Recall
    Then the attempt is refused because the skill is locked
