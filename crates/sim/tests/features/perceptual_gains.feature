Feature: Inherited perception changes what experience feels like
  Threat salience and novelty tolerance are deliberately coarse approximations of
  inherited perceptual differences. They change how strongly experience is encoded
  and whether an unfamiliar person attracts attention. They never write an opinion,
  a preference, or a position for anyone.

  Scenario: A threatening experience is encoded through inherited salience before memory
    Given two remembering people with low and high threat salience
    When both live through the same bad experience and the same good experience
    Then the bad experience is stronger for the high salience person
    And the good experience is unchanged for both people

  Scenario: Novelty tolerance changes attention only outside familiar relationships
    Given two observers with low and high novelty tolerance
    When both watch an unfamiliar person to whom they are weakly attached
    Then the more novelty tolerant observer pays more attention
    But kin and close companions receive the same attention from both observers

  Scenario: Children inherit perception without copying a parent's disposition
    Given parents carrying different copies of both perceptual traits
    When they have a child
    Then the child receives one copy of each trait from each parent
    And both expressed perceptual gains remain between half and one and a half times normal
