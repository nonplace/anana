---
name: gherkin-bdd
description: Write and maintain the AnanA simulation's executable Gherkin specs — Given/When/Then .feature files that run as tests via the cucumber crate and serve as living, human-readable documentation of what the simulation currently does. Use when adding or changing simulation behaviour, when a .feature file goes red, when setting up the BDD harness, or whenever asked for executable specs, behaviour specs, acceptance tests, feature files, or judge-readable documentation of the game.
---

# Executable Gherkin specs for AnanA

## Why this exists

The simulation has to be legible to someone who will never read Rust. Prose documentation goes stale
the moment behaviour changes and nobody notices. So the documentation is written as Given/When/Then
scenarios that **execute as tests**: if the description stops matching the simulation, the build goes
red.

Two audiences, one artifact:
- a reader who wants to know what the world does, and can read plain English scenarios;
- the build, which runs them and fails when they stop being true.

These files are the living record of the **current** state of the build. They are not a wishlist and
not a plan. Never write a scenario for behaviour that does not exist yet.

## Setup (verified against cucumber 0.23.0)

The specs live in the `sim` crate, because it depends on `core` and owns the tick loop, so one
harness can drive both pure domain functions and the whole running world. That also keeps
`cargo test -p anana-core --lib` fast and bevy-free.

`Cargo.toml` at the workspace root — add to `[workspace.dependencies]`:

```toml
cucumber = "0.23"
```

`crates/sim/Cargo.toml`:

```toml
[dev-dependencies]
cucumber.workspace = true
tokio.workspace = true      # the runner is async; this provides #[tokio::main]

[[test]]
name = "bdd"
harness = false             # cucumber prints its own output instead of libtest's
```

`harness = false` only applies to this one target. Other files in `tests/` (for example
`tests/determinism.rs`) are still auto-discovered and still run under the normal harness.

Layout:

```
crates/sim/
├── tests/
│   ├── bdd.rs                 # the harness + all step definitions
│   └── features/
│       ├── time_and_ageing.feature
│       ├── inheritance.feature
│       ├── recall.feature
│       ├── virus_spread.feature
│       ├── gosh.feature
│       └── determinism.feature
```

## The harness

```rust
//! Executable specifications for the AnanA simulation.
//!
//! Every scenario in `tests/features/` is a plain-English description of something
//! the world actually does, and it runs as a test. If a scenario goes red, either
//! the simulation broke or the description is out of date — fix whichever is wrong.

use cucumber::{given, then, when, World as _};

#[derive(Default, cucumber::World)]
pub struct AnanaWorld {
    seed: u64,
    app: Option<anana_sim::App>,        // the running world
    other: Option<anana_sim::App>,      // a second world, for determinism scenarios
    // scratch space for pure-domain scenarios:
    mother: Option<anana_core::Genome>,
    father: Option<anana_core::Genome>,
    child: Option<anana_core::Genome>,
    probability: Option<anana_core::Permille>,
    error: Option<String>,
}

// The derive requires `Debug`, but a bevy `App` is not usefully printable — implement
// it by hand rather than deriving, so the World can hold the running world directly.
impl std::fmt::Debug for AnanaWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnanaWorld")
            .field("seed", &self.seed)
            .finish_non_exhaustive()
    }
}

#[tokio::main]
async fn main() {
    AnanaWorld::cucumber()
        .fail_on_skipped()
        .run_and_exit("tests/features")
        .await;
}
```

`fail_on_skipped()` and `run_and_exit()` are both required and are not optional polish:
- without `fail_on_skipped()`, a scenario whose steps have no matching definitions is reported as
  skipped and the run still passes — silently green documentation, which is worse than none;
- `run_and_exit()` exits non-zero on failure, which is what makes `cargo test --workspace` gate.

Run only the specs while iterating: `cargo test -p anana-sim --test bdd`.

## Step definitions

The first parameter is always `&mut World`. Later parameters are captured from the step text. Steps
may be `async fn` or plain `fn`; the simulation is synchronous, so plain `fn` is fine.

```rust
#[given(expr = "a new world seeded with {int}")]
fn a_new_world(w: &mut AnanaWorld, seed: u64) {
    w.seed = seed;
    w.app = Some(anana_sim::build_headless_app(seed, anana_sim::Config::default()));
}

#[when(expr = "the world advances {int} tick(s)")]
fn advance(w: &mut AnanaWorld, ticks: u64) {
    let app = w.app.as_mut().expect("a world was started in a Given step");
    for _ in 0..ticks {
        anana_sim::step(app);
    }
}

#[then(expr = "the world clock reads tick {int}")]
fn clock_reads(w: &mut AnanaWorld, expected: u64) {
    let app = w.app.as_ref().expect("a world was started in a Given step");
    assert_eq!(current_tick(app), expected);
}
```

Three forms of matcher, in order of preference:
- `#[then("she is full")]` — a plain literal, when nothing varies.
- `#[given(expr = "a virus with a spreadscore of {int}")]` — Cucumber Expressions, `{int}`,
  `{word}`, `{string}`, `{float}`. Prefer this.
- `#[when(regex = r"^...$")]` — only when an expression genuinely cannot express it.

`unwrap`/`expect` are fine here: this is test code, and a panic is how a step reports failure.

## Writing the scenarios

Rules, in priority order:

1. **Domain language only.** A step says `a newborn who has not learned Recall`, never
   `a HumanState with skills.recall_learned() == false`. No Rust type names, no function names, no
   field names, no crate names anywhere in a `.feature` file.
2. **A scenario is one behaviour.** If you need "and also", it is two scenarios.
3. **Given sets up, When does one thing, Then observes.** Never assert in a Given. Never mutate in a
   Then.
4. **Write the Feature description for a stranger.** Two or three sentences under the `Feature:` line
   explaining why this behaviour matters in the world. This is the part a non-programmer actually
   reads.
5. **Concrete numbers beat vague words.** `advances 20 ticks` is testable; `advances a while` is not.
6. **Only describe what exists.** If the behaviour is not built, the scenario does not get written.

## The specs this project must carry

These six cover the simulation's core iterations. Extend them as behaviour lands; keep them green.

**`recall.feature`** — the headline idea of the whole simulation:

```gherkin
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
```

**`time_and_ageing.feature`** — a tick advancing time and ageing:

```gherkin
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
```

**`inheritance.feature`** — inheritance and gene activation at birth:

```gherkin
Feature: Children inherit genes from two parents and express them once, at birth
  Every child takes one copy of each gene from its mother and one from its father. Which
  of those genes actually show is decided once, at the moment of birth, and never again.

  Scenario: A child takes one gene copy from each parent
    Given a mother and a father with known genes
    When they conceive a child
    Then the child carries one copy from the mother and one from the father at every gene

  Scenario: The same parents and the same seed always produce the same child
    Given a mother and a father with known genes
    When they conceive a child twice from the same seed
    Then both children are genetically identical

  Scenario: A hidden gene is still passed on
    Given a parent who carries the disease gene without showing the disease
    When they pass their genes to a child
    Then the child can still inherit the disease gene

  Scenario: Traits are settled at birth and never re-rolled
    Given a newborn whose traits have been expressed
    When the world advances 50 ticks
    Then the newborn's expressed traits are unchanged
```

**`virus_spread.feature`** — the two endpoints, which hold by construction:

```gherkin
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
```

**`gosh.feature`** — a gosh changing the world, permanently and on the record:

```gherkin
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
```

**`determinism.feature`** — the property the whole project rests on:

```gherkin
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
```

## Keeping them green

- A behaviour change and its `.feature` change go in the **same PR**. Never merge a PR that leaves a
  spec describing something untrue.
- A red spec means one of two things: the simulation regressed, or the description is out of date.
  Work out which before touching either. Never "fix" a spec by weakening the assertion to match a
  bug.
- New behaviour that a reader would care about gets a scenario. Internal refactors do not.
- Keep the whole suite fast. These run on every `cargo test --workspace`; if the tick counts make it
  slow, lower them rather than letting people skip the suite.

## Judge-readable unit tests too

The same standard applies outside the `.feature` files:

- Full-sentence test names describing the behaviour proved:
  `fn non_recall_experience_decays_until_recall_is_learned()`, not `fn test_decay()`.
- Every `#[cfg(test)] mod tests` opens with a `//!` comment saying in plain language which part of
  the simulation the module proves.
- Prefer one clear assertion of one behaviour per test over a grab-bag of assertions.

## A note on off-the-shelf alternatives

Community agent-skill marketplaces carry several general-purpose Cucumber/Gherkin skills covering BDD
style, declarative scenario design, and step-definition patterns. They are written for the mainstream
Cucumber ecosystems (Ruby, JavaScript, Java, Python) and give generic advice about writing good
Gherkin. None of them know cucumber-rs's Rust API, this workspace's layout, or the determinism model
that most of these scenarios exist to protect. Use this skill; borrow general Gherkin style advice
from those if you want a second opinion on wording.
