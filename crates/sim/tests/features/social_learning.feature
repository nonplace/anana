Feature: People build knowledge through experience and one another
  People improve by doing things, by watching capable neighbours, and by teaching. Watching
  succeeds only when the watcher pays attention, can remember what happened, can reproduce the
  action, and has a reason to try. If any one of those stages fails, nothing is acquired.

  Teaching is most useful inside a moving zone of reachable difficulty. A beginner needs someone
  only somewhat ahead, while an already capable learner can be stretched by a wider gap. A peer
  has nothing new to offer and a distant expert is hard to understand.

  Memories also need use. Unpractised knowledge fades quickly at first and then more slowly.
  Repeated retrievals spread across time make it increasingly stable, and relearning something
  once known is cheaper than learning it for the first time.

  Scenario: Watching helps but doing teaches more
    Given an attentive remembering adult watches a more capable neighbour
    When their observational learning is compared with doing the same task
    Then watching produces some learning but less than direct experience

  Scenario: Every stage of observation is necessary
    Given four otherwise ready observers each missing one stage of observation
    When each watches the same capable neighbour
    Then none of the four observers learns from watching

  Scenario: Teaching works best within reachable difficulty
    Given a beginner can choose a peer, a nearby teacher, or a distant expert
    When the beginner receives the same length lesson from each
    Then the nearby teacher transfers the most

  Scenario: The useful teaching gap grows with competence
    Given a beginner and an already capable learner can choose among teachers
    When each chooses the lesson that transfers the most
    Then the capable learner chooses a teacher further ahead

  Scenario: Spaced retrieval outlasts massed restudy
    Given equal experience is massed for one learner and retrieved over time by another
    When both are tested immediately and again after a long delay
    Then massed restudy looks better immediately but spaced retrieval lasts longer
