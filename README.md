# AnanA

**Conway's *Game of Life* simulated cells. AnanA simulates lives.**

AnanA is a deterministic, tick-driven simulation of human life, written in
Rust. Every human is governed by the same interacting systems: diploid genes,
expressed traits, heritable instincts, developing consciousness, recall-gated
learning, social bonds, conferred prestige, anonymous positions, ageing, mating,
infection, and death. No individual life is scripted; it emerges from the
mechanics.

You watch the world in **gosh-mode**, as a god whose only way to change it is a
recorded divine decree. Read the name forwards or backwards and it is the same.
Given the same seed, configuration, and recorded influence, so is the world.

## Quickstart (30 seconds)

Prerequisite: [Rust via rustup](https://rustup.rs). The workspace requires Rust
1.97 or newer and pins the stable toolchain, rustfmt, and clippy in
`rust-toolchain.toml`.

```bash
git clone https://github.com/nonplace/anana.git
cd anana
```

Open the live dashboard with the deterministic offline mind. Press `q` to quit.

```bash
cargo run --release -p anana -- --mode live --offline --seed 42
```

Run 500 ticks without a terminal UI:

```bash
cargo run --release -p anana -- --mode headless --offline --seed 42 --ticks 500
```

That command currently prints one canonical line:

```text
hash=21346ff0615d57a7fda12593494d2242979e3c721b25a2bfd11717e6f4061602 tick=500 living=120 births=78 deaths=38 infections=86 generation=1 lineages=57 lived=158 faults=0
```

Prove determinism with two commands. The first compares two seed-42 runs; the
second checks that seed 43 diverges:

```bash
a=$(cargo run --release --quiet -p anana -- --mode headless --offline --seed 42 --ticks 500); b=$(cargo run --release --quiet -p anana -- --mode headless --offline --seed 42 --ticks 500); test "$a" = "$b" && echo "same seed: identical"
a=$(cargo run --release --quiet -p anana -- --mode headless --offline --seed 42 --ticks 500); b=$(cargo run --release --quiet -p anana -- --mode headless --offline --seed 43 --ticks 500); test "$a" != "$b" && echo "different seeds: diverged"
```

No sample data is needed because **the seed is the data**. There is no database,
fixture import, or world file to download. The seed and built-in configuration
create the founders and initial virus; the tick loop produces the rest.

## Counterfactuals: what did one decree cost?

The counterfactual command runs one world to a chosen instant, branches it, and
projects two futures to the same horizon: one untouched and one receiving exactly
one gosh. People alive at the split retain the same identity in both futures, so
their fates can be compared directly. People born later have branch-scoped
identities and are compared only in aggregate; the program never pretends that
two post-branch births are the same person.

This quick example kills founder H1 at tick 20 in a deliberately small world:

```bash
cargo run --release -p anana -- counterfactual --seed 42 --branch-at 20 --horizon 80 --initial-population 12 --carrying-capacity 40 --gosh '{"Afflict":{"target":{"One":1},"bane":{"Harm":65535}}}'
```

The real output, with terminal colours removed here, is:

```text
A n a n A · COUNTERFACTUAL
seed 42 · branch t20 · horizon t80 · branch hash 58f346a4573e0b293a79835d16321996084f2c7b3c5fe9c8c12b45fa2ecc4aa8
decree · Afflict { target: One(HumanId(1)), bane: Harm(65535) }

UNTOUCHED                                        │ DECREED
hash 3328d7ee1ee7ffac97d6aefbe3a3a7a6            │ hash 52088e1770ccd60edf57636f7885523c
     b3974487f066380b0c084123833ad246            │      2f094686246984221f7d2302ce11aac5
living                  13                       │ living                  12
births after branch      3                       │ births after branch      3
deaths after branch      1                       │ deaths after branch      2
surviving lineages      10                       │ surviving lineages       9
knowledge held           9                       │ knowledge held           9

WHAT THE DECREE CHANGED
DIED WHO OTHERWISE LIVED              1
NEVER BORN                            0
LINEAGES ENDED                        1
KNOWLEDGE LOST                        0

PEOPLE ALIVE AT THE BRANCH
H1 · alive · age 93 ticks                        │ H1 · died t21 · age 34 ticks

AFTER THE BRANCH · AGGREGATES ONLY
lineages surviving here: H1                      │ the same lineages are extinct
lives unique here: none                          │ additional lives here: none
knowledge surviving here: none                   │ knowledge unique here: none

This compares two runs of a model; it is not a claim about real people.
```

The identity of that result is the seed, branch-world hash, decree, and horizon.
The untouched continuation is tested against an unbranched run, and repeating the
same request is byte-identical. Add `--json` to emit the complete comparison as
structured data. The `--gosh` value is the same canonical gosh object used by the
dashboard, not a second command-only format.

## Command-line interface

```text
anana [--seed <SEED>] [--ticks <TICKS>] [--mode <live|replay|headless>] [--offline]
      [--initial-population <N>] [--carrying-capacity <N>] [--mating-interval <N>]

anana counterfactual --seed <SEED> --branch-at <TICK> --horizon <TICK>
      --gosh <CANONICAL_JSON> [--json] [--initial-population <N>]
      [--carrying-capacity <N>] [--mating-interval <N>]
```

| Flag | Meaning | Default |
|---|---|---|
| `--seed <u64>` | Master seed from which keyed draws are derived | `42` |
| `--ticks <u64>` | Tick limit | until quit in live mode; `5000` in replay and headless modes |
| `--mode <live\|replay\|headless>` | Dashboard, in-process replay scrubber, or one-line headless run | `live` |
| `--offline` | Force the deterministic, network-free mind | off; also used automatically when no API key is available |
| `--initial-population <u32>` | Founder population | `80` |
| `--carrying-capacity <u32>` | Population level that smoothly damps births | `300` |
| `--mating-interval <u64>` | Ticks between mating phases | `10` |

Replay mode runs a world from the requested seed, rebuilds it from its recorded
authored events, verifies the per-tick hash prefix, and opens the paused dashboard.
Use Page Up and Page Down to scrub. It does not currently read or write a log file.

```bash
cargo run --release -p anana -- --mode replay --offline --seed 42 --ticks 100
```

## Gosh-mode

The dashboard has four views: a deterministic population map, the canonical
event feed, a human inspector, and a narrative panel. Navigation, filtering,
pausing, stepping, and requesting a story are observational. They do not enqueue
events or alter canonical state.

Press `g` to prepare a gosh. The current modal can:

- heal the selected human, raise their fertility, or grant immunity;
- harm one human, their whole lineage, or everyone;
- teach the selected human Recall, subject to the awareness gate;
- seed a newborn using the selected human's genome.

Magnitude and affliction target can be adjusted before confirmation. Only Enter
emits the completed decree; Escape cancels it. A confirmed gosh enters the same
event intake as engine and AI-authored events, resolves in the pure core without
a random draw, and is appended to the log with author `God`. A decree is not a
gamble, and looking never changes anything.

### Recall is the central mechanic

Before a human learns Recall, incoming experience is attenuated, stored
experience decays, and no other skill can latch as learned. Once Recall latches,
decay stops and experience compounds. The same rule reaches the interface and
narrator: an amnesic human has no detailed remembered history to display or send
to a mind.

People also learn socially inside explicit residence groups. Observation passes
through separate attention, retention, reproduction, and motivation stages;
deliberate teaching works best at a moderate competence gap that widens as the
learner improves. Unused experience follows a concave forgetting curve, while
spaced retrieval builds stability and makes later relearning cheaper.

### A virus is the second lifeform

The virus has its own incubation, infectious, and recovery phases. Infection
probability combines integer-only contagiousness, resistance, fear, contact, and
medicine. Its endpoints are absolute by construction: spreadscore 0 can never
infect, while spreadscore 100 cannot be resisted. Recovered humans gain immunity
to that `VirusId`; v1 uses one strain per virus identifier and does not create new
strains.

### Positions are socially costly

Each remembering human can hold eight anonymous positions from -1000 to +1000.
The slots have no topics: the engine never knows what slot three means. When
contradicting information arrives, retained evidence pulls a position toward it,
while attachment to people who agree makes changing socially expensive. If that
social cost is larger, the person moves away from the evidence. Genes never write
a position; children acquire none until Recall lets them retain information.

That local rule produced a mean position spread of **814.496** with coalition
cost, against **14.396** in the same seed with coalition cost disabled. The normal
world contained **474 negative, 272 middle, and 426 positive** held positions;
the control contained **0 negative, 803 middle, and 0 positive**. No rule anywhere
mentions polarisation, camps, or a preferred position.

In a separate contradiction burst, **68 humans moved away** from strong opposing
information and **9 moved toward it**. The people who moved away had far more
attachment to people already aligned with them: mean summed attachment 24,623.0,
against 1,123.4 among those who moved toward the information.

### Inherited perception, not inherited opinions

Two additive diploid loci are expressed once at birth as gains between 500 and
1500 parts per thousand. Threat salience scales only the encoding of bad
experience before Recall; novelty tolerance scales only attention to unfamiliar,
non-kin people with whom attachment is weak. Neither locus touches a position,
preference, or value.

Across five paired seeds, the threat-allele distribution moved **0.0504** farther
with virus pressure than without it. Repeating the comparison with the locus fixed
at the population median produced **0.0000**, the logging control. This is an
observed association inside the model, not a calibrated claim about human biology.

The inheritance diagnostic also found no non-genetic transmission path. Novelty
tolerance was almost uncorrelated between partners (`r = 0.035`, or `0.062` when
weighted by births), while its parent-to-offspring regression slope was `0.562`.
The larger Pearson correlation (`r = 0.656`) is therefore best explained by the
unequal parent and child variance of the finite founder-derived cohort, with
selection still able to contribute—not by perceptual assortment or copied traits.

## Determinism

**Same seed, configuration, and recorded authored events: same trajectory, tick
for tick.**

Four constraints make that practical:

- **Keyed randomness.** Every draw is a pure function of the master seed plus
  `(domain, tick, subject, salt)`. No draw depends on ECS iteration order or on
  how many draws happened earlier.
- **Integer canonical state.** Probabilities use parts per thousand; health,
  experience, ages, and counters are integers. Floats exist only in terminal
  rendering and never flow back into the simulation.
- **Canonical ordering.** Serialized and order-sensitive paths use `BTreeMap`,
  `BTreeSet`, birth-ordered vectors, and stable `HumanId` sorting rather than
  Bevy entity order.
- **A recorded event spine.** Discrete births, deaths, infections, engine chance
  events, goshes, and AI-authored events carry tick, sequence, author, subjects,
  payload, and outcome. Routine ageing and health updates are deterministic
  systems and are re-derived rather than logged individually. Raw random values
  are not stored: replay re-derives them from the recorded keys.

After every tick, canonical state is serialized with `postcard` and hashed with
BLAKE3. The event-log digest extends only with records added during that tick,
then composes with the state hash, so long histories remain linear rather than
re-hashing their entire past. Tests compare the complete per-tick hash history,
not only the final state. Golden regression tests also pin one keyed draw and one
fully populated world hash, making dependency-stream or serialization-layout
changes fail loudly.

## Minimum viable population

There is no minimum population constant or survival threshold in the code. In a
3,000-tick experiment over seeds 41, 42, and 43, **30 founders collapsed to 1,
3, and 2 living humans**, **32 remained nonzero in two of three runs** (172, 0,
and 9), and **36 survived in all three** with 205, 195, and 197 living humans;
all three 36-founder runs reached five generations. The boundary falls out of
time to fertility racing age-structured mortality and the encounter rate needed
for courtship. It is an observed range for this configuration, not a universal
biological number, and “collapsed” is not quietly reported as literal extinction.

## Predictions the model got wrong

Two preregistered expectations were weaker or reversed, and they remain visible
rather than being tuned away:

- Partner traits followed the predicted broad ordering, but the middle classes
  did not separate into convincing distinct bands: the correlations were 0.592,
  0.254, 0.232, 0.215, and 0.162, with desirability at 0.709. The long-run test is
  ignored by default for runtime, and when run explicitly it passes its current
  broad assertions. We report that plainly rather than calling a passing test a
  failure or tightening weights until the gaps look impressive.
- Humans with more relationships changed their positions **more**, not less, over
  their lifetimes. The measured regression slope was **+52.410** across 364
  humans, opposite the predicted negative sign. We left the mechanism and result
  intact because forcing the sign would erase the finding we asked the model to
  test.

The skill system can represent loss when the last holder dies without teaching,
and counterfactual comparisons report whether either future still has a living
holder. But no skill has gone permanently extinct in any measured run: with nine
skills and well over a hundred living humans, somebody has always retained each
one. Implemented possibility and observed result are different claims.

## Tests and executable specifications

Run the full network-free workspace suite:

```bash
cargo test --workspace
```

Useful focused commands:

```bash
cargo test -p anana-core --lib
cargo test -p anana-sim --test bdd
cargo test -p anana-sim --test determinism
cargo test -p anana-sim --test replay
```

The suite proves the pure domain rules, exact RNG and world-hash pins, same-seed
per-tick equality, different-seed divergence, independence from requested executor
thread count, gosh replay, UI purity, offline-mind determinism, model-output
validation, and CLI defaults. Model-client tests use canned JSON; no test contacts
the API.

The default suite executes **206 Rust tests** plus **51 scenarios** from **13
Given/When/Then feature files**. Seven additional long-running statistical tests
are present but ignored by default. The feature files are living, non-technical
documentation:

| Feature | What it proves |
|---|---|
| `time_and_ageing.feature` | A tick advances the clock and ages every living human |
| `inheritance.feature` | Children inherit one copy from each parent and express traits once at birth |
| `recall.feature` | Recall determines whether experience fades or compounds |
| `virus_spread.feature` | Dormant never infects, full contagiousness always does, and probability is monotonic |
| `gosh.feature` | Divine influence is deterministic, permanent, recorded, and distinct from observation |
| `determinism.feature` | Same seeds match, different seeds diverge, and recorded history replays |
| `population.feature` | Fertile years, birth spacing, density dependence, death records, and many generations |
| `social_learning.feature` | Lived experience, four-stage observation, targeted teaching, forgetting, spacing, and retrieval |
| `bonds_and_courtship.feature` | Attachment builds with diminishing returns, betrayal wounds deeper than one kindness repairs, and courtship needs mutual attachment |
| `prestige_and_coalitions.feature` | Standing is conferred by followers rather than seized, relationships are bounded, and large groups reorganise |
| `counterfactual.feature` | A branch leaves its untouched future unchanged, an empty decree changes nothing, and the same comparison reproduces byte for byte |
| `perceptual_gains.feature` | Inherited gains alter threat encoding and unfamiliar attention without writing opinions |
| `positions.feature` | Recall gates anonymous positions and social cost can make contradiction backfire |

For example, this documentation is executable:

```gherkin
Scenario: Recall gates skill retention
  Given a newborn who has not learned Recall
  When the world advances 20 ticks of practice
  Then their skill experience decays instead of accumulating
  And no skill has been marked as learned
```

The specs live in [`crates/sim/tests/features/`](./crates/sim/tests/features/)
and the harness is [`crates/sim/tests/bdd.rs`](./crates/sim/tests/bdd.rs). The
harness fails on skipped steps, so missing documentation bindings cannot pass
silently.

## Architecture

AnanA is a functional core inside an imperative shell, split into five crates:

| Crate | Responsibility |
|---|---|
| `anana-core` | Pure domain: ids, keyed RNG, genetics, perception, skills, bonds, prestige, positions, events, goshes, canonical snapshot, and hash |
| `anana-sim` | Headless Bevy ECS: founder seeding, ordered tick systems, social interaction, event intake/log, snapshots, replay, and counterfactual projection |
| `anana-mind` | GPT-5.6 boundary, deterministic offline mind, prompt summaries, structured response parsing, and validation |
| `anana-tui` | ratatui presentation and input intents; no simulation rules |
| `anana` | Thin CLI binary that selects a mind and wires live, replay, headless, and counterfactual drivers |

Dependency direction is deliberately one-way:

```text
app -> {tui, sim, mind} -> core
```

Core has no internal dependencies and does not depend on the Bevy runtime, Tokio,
Reqwest, or Anyhow. The sim enables only core's optional ECS component derives.
This keeps domain tests fast and prevents engine, async-runtime, and HTTP-client
dependencies from leaking inward.

One explicit simulation step runs nine chained systems in a fixed order: advance
the clock; age and update health; learn; mate; give birth; spread and advance the
virus; resolve pending and engine events; process death; snapshot and hash. Before
order-sensitive work, humans are collected and sorted by stable domain id.

The design constraints behind the code are recorded in [`soul.md`](./soul.md).

## Built with Codex; using GPT-5.6 at runtime

### Human decisions

Janusch specified the product and every load-bearing constraint before
implementation: simulate lives rather than cells; make Recall the prerequisite
for durable experience; limit player influence to recorded goshes; treat
determinism as a product requirement; split the workspace into a pure core and
imperative shell; use keyed randomness, integer canonical state, ordered
containers, and an append-only event spine; and require test-first domain work,
executable specifications, warning-clean builds, and pull requests for every
coherent unit.

### Where Codex accelerated the build

Codex executed that design in a continuous build session. It scaffolded the five
crates, implemented the domain and runtime modules, wrote the unit, integration,
golden, replay, rendering, client, and Cucumber tests, ran the formatter/linter/test
loop, and landed the work as small pull requests. The leverage came from
applying the same prohibitions consistently across a large surface—no ambient RNG,
no unordered canonical maps, no canonical floats, no model calls inside a tick,
and no untested domain behavior—while the human retained control of mechanics,
architecture, tradeoffs, and acceptance criteria.

### What GPT-5.6 does when AnanA runs

GPT-5.6 lives at the edge behind the `Mind` trait. It has two jobs:

1. **Narrate on request.** Pressing `n` builds a compact `LifeHistory` from the
   selected human and their canonical log entries. GPT-5.6 returns structured
   `{title, story, epitaph}` JSON. If Recall is absent, detailed events are withheld
   before the request is built.
2. **Propose world events.** Every 25 ticks, between simulation steps, the shell
   sends a sorted, read-only `WorldContext` and requests a structured event batch.
   Unknown humans and targets are dropped, unknown JSON fields are rejected,
   probabilities and magnitudes are bounded, batch size is capped, and validated
   events are sorted before entering the simulation.

The model never computes an outcome and the sim never performs a network call.
Validated proposals become ordinary core event payloads, are resolved by keyed
domain logic, and are recorded with author `Ai`. Model or transport failure yields
no AI events for that cycle; the world keeps running.

The binary selects `GptMind` only when `OPENAI_API_KEY` is present and `--offline`
is absent. Otherwise it uses `OfflineMind`, whose stories and proposals are derived
deterministically from their inputs. To enable GPT-5.6, set that environment
variable and run the live quickstart command without `--offline`.

## License

[MIT](./LICENSE). Copyright (c) 2026 Janusch Häring.
