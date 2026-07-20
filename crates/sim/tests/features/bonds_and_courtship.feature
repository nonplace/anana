Feature: Attachment and childhood shape who becomes a partner
  People do not conceive with strangers. Shared good experiences build attachment quickly at
  first and then with diminishing returns, while neglect weakens it and betrayal leaves a larger
  wound than one helpful act can repair. Courtship begins only when both people have built enough
  attachment, so the weaker direction decides whether a pair is ready.

  Choice is mutual. People tend to choose partners near their own age, somewhat like themselves
  in values and ability, and desirable to everybody. Similarity matters by different amounts, so
  partners correlate without becoming copies and no rule tells anyone to seek their own rank.

  Early co-rearing creates a separate sexual aversion. An older child directly sees a newborn
  being cared for; the younger child instead accumulates a duration cue that is strongest in the
  earliest years. This reluctance is strong but not an absolute prohibition. Genetic first-degree
  kinship remains a separate safety rule.

  Scenario: Courtship grows from repeated mutual attachment
    Given two strangers begin sharing positive experiences
    When they meet repeatedly over time
    Then one meeting was not enough but both eventually become ready to court

  Scenario: Neglect and betrayal damage attachment
    Given an attached pair stops meeting and another pair experiences betrayal
    When their bonds are compared after time has passed
    Then neglect lowers attachment and betrayal costs more than one cooperation gains

  Scenario: Different qualities shape partner choice by different amounts
    Given a chooser compares partners differing in age, values, ability, body, and temperament
    When each difference is considered separately
    Then age matters most and temperament matters least

  Scenario: Unrelated children reared together develop reluctance
    Given two unrelated children share a home from infancy
    When they reach the age of courtship
    Then their childhood strongly suppresses pairing without making it impossible

  Scenario: Siblings reared apart are protected by lineage rather than childhood reluctance
    Given two half siblings grow up in separate homes
    When they reach the age of courtship
    Then they have no childhood reluctance toward each other
    And their first degree lineage still prevents conception
