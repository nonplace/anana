Feature: A population renews itself across human generations
  One tick represents about two weeks. A typical life lasts roughly two thousand four
  hundred ticks, or ninety years, and a generation turns over in about six hundred to
  eight hundred ticks. A five-thousand-tick history can therefore contain many generations.

  People are born, mature through a gradual fertile window, have children spaced across
  their fertile years, grow old and die. As the world fills, births become steadily less
  likely instead of striking a hard wall, so a large population can settle near what its
  world can support.

  Scenario: Children are spaced across a mother's fertile years
    Given a healthy fertile couple in an otherwise empty world
    When their world advances through several chances to conceive
    Then their children are born with recovery time between births

  Scenario: A crowded world gently dampens births
    Given two equally fertile worlds, one open and one nearly full
    When both worlds reach a chance to conceive
    Then the nearly full world has fewer births without forbidding them at a wall

  Scenario: The world remembers people after they die
    Given a living human whose life is about to end
    When the world advances beyond that life
    Then the human is gone from the living population
    And their lineage and learned skills remain in the world's memory

  Scenario: A long history contains a large many-generation population
    Given a new society seeded with 42
    When the society lives through 5000 ticks
    Then hundreds of people remain alive within the world's carrying capacity
    And the society has reached at least five generations
