Feature: Time passes and bodies age
  The world moves in discrete ticks. Every tick, each living human grows a little older,
  and enough ticks carry them from one stage of life into the next.

  Scenario: A tick advances the clock and ages everyone alive
    Given a new world seeded with 42
    When the world advances 1 tick
    Then the world clock reads tick 1
    And every living human is one tick older

  Scenario: Enough time carries a human into a later stage of life
    Given a new world seeded with 42
    When the world advances 2000 ticks
    Then at least one human has reached a later stage of life than they were born into
