Feature: A virus spreads according to how contagious it is
  A virus with no contagiousness is dormant and cannot infect anyone, however exposed they
  are. A fully contagious virus cannot be resisted at all, however healthy, careful or
  well-treated its victim is. Everything else falls between those two ends.

  Scenario: A dormant virus never infects anyone
    Given a virus with a spreadscore of 0
    When a completely exposed human is contacted
    Then the chance of infection is none

  Scenario: A fully contagious virus always infects
    Given a virus with a spreadscore of 100
    When a maximally resistant, fearful and well-treated human is contacted
    Then the chance of infection is certain

  Scenario Outline: Being more contagious never makes a virus less infectious
    Given a virus with a spreadscore of <lower>
    And a second virus with a spreadscore of <higher>
    Then the more contagious virus is at least as likely to infect
    Examples:
      | lower | higher |
      | 10    | 20     |
      | 40    | 75     |
      | 75    | 99     |
