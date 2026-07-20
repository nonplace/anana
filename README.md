# AnanA

**Conway's *Game of Life* simulated cells. AnanA simulates lives.**

AnanA is a deterministic, tick-driven simulation of human life, written in
Rust. Every human is governed by the same interacting systems: diploid genes,
expressed traits, heritable instincts, developing consciousness, recall-gated
learning, ageing, mating, infection, and death. No individual life is scripted;
it emerges from the mechanics.

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
hash=207de206bea0287cb113d554b8e8834e9cc7bf5e1100e79d2dc2486813687a7a tick=500 living=138 births=100 deaths=42 infections=176 generation=1 lineages=62 lived=180 faults=0
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

## Command-line interface

```text
anana [--seed <SEED>] [--ticks <TICKS>] [--mode <live|replay|headless>] [--offline]
      [--initial-population <N>] [--carrying-capacity <N>] [--mating-interval <N>]
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

Eight Given/When/Then feature files execute through `cucumber` as living,
non-technical documentation:

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
| `anana-core` | Pure domain: ids, keyed RNG, genetics, phenotype, skills, virus probability, events, goshes, canonical snapshot, and hash |
| `anana-sim` | Headless Bevy ECS: resources, founder seeding, ordered tick systems, event intake/log, snapshots, and replay |
| `anana-mind` | GPT-5.6 boundary, deterministic offline mind, prompt summaries, structured response parsing, and validation |
| `anana-tui` | ratatui presentation and input intents; no simulation rules |
| `anana` | Thin CLI binary that selects a mind and wires live, replay, and headless drivers |

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
