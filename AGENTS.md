# AnanA — Agent Guide

Deterministic simulation in Rust. Functional core / imperative shell. You are driving a
solo 2-day build; optimize for correctness, determinism, and small verifiable steps.

## Workspace layout (Cargo workspace, 5 crates)
- `core/`  Pure domain. NO I/O, NO async, NO time, NO RNG side effects, NO bevy. Deterministic
           functions over owned data + explicit seed. This is where the tests live.
- `sim/`   Headless bevy ECS. Wraps `core`. Fixed timestep, explicit system ordering.
- `mind/`  GPT-5.6 client behind a trait (`trait Mind`). Real impl + a deterministic fake for tests.
- `tui/`   ratatui presentation only. No domain logic.
- `app/`   Thin binary. Wiring/args/config only.

Dependency direction (never violate): app -> {tui, sim, mind} -> core. `core` depends on nothing internal.

## The loop — run these yourself after EVERY change, do not ask
1. `cargo fmt --all`
2. `cargo clippy --all-targets --all-features -- -D warnings`   (fix warnings, never `#[allow]` to silence)
3. `cargo test --workspace`   (unit tests AND the executable `.feature` specs)
Do not report a task done until all three are green. Paste the final `cargo test` summary as evidence.

Fast inner loop while iterating on the domain: `cargo test -p anana-core --lib` — it skips the
integration targets and never compiles bevy. Run the full three before every commit.

## TDD (mandatory for `core` and `mind` logic)
- Write the failing unit test first, in the same file (`#[cfg(test)] mod tests`). Run it, see it fail.
- Then implement the minimum to pass. Re-run. Refactor. Business logic without a test is incomplete.
- `mind`: test against the deterministic fake, never the network.

## Tests are documentation — write them for a reader, not just a runner
- Test names are full sentences describing the behaviour being proved, not labels:
  `fn non_recall_experience_decays_until_recall_is_learned()`, never `fn test_decay()`.
- Every `#[cfg(test)] mod tests` opens with a `//!` comment saying, in one or two plain sentences,
  which part of the simulation that module proves.
- Assume someone non-technical will skim these files to understand what the simulation does. They
  should come away understanding it. Name things in domain language, not implementation language.

## Executable specs (Gherkin) — the documentation has to run
- The simulation's behaviour is described in Given/When/Then `.feature` files that EXECUTE as tests
  via the `cucumber` crate. Harness: `crates/sim/tests/bdd.rs`. Specs: `crates/sim/tests/features/`.
- Write them so a non-programmer reads a scenario and understands what the world does. Domain
  language only — no Rust type names, no function names, no internal jargon leaking into a step.
- They are living documentation of the CURRENT state of the build. When behaviour changes, the
  matching `.feature` changes in the same PR. A red spec is a broken build, not a stale document.
- Full guidance, setup, and step-writing rules: `.codex/skills/gherkin-bdd/SKILL.md`.

## Purity rules (functional core)
- `core` functions are pure: inputs in, value out, no globals, no I/O, no logging, no clocks.
- Side effects (rendering, model calls, file/network) live only in `sim`/`mind`/`app` (the shell).
- Prefer `&T`/owned returns over interior mutability. No `static mut`, no global RNG.

## Determinism rules (seed-reproducible — this is a hard requirement)
- Same seed => byte-identical trajectory. All randomness flows from an explicit seed stored in state.
- RNG: counter-based keyed draws — `draw(domain, tick, subject, salt)` derived from the master seed.
  Draws must NOT depend on iteration or system-scheduling order. Never use `thread_rng`,
  `rand::random`, or OS entropy in `core`/`sim`. (`rand_chacha::ChaCha8Rng` for the underlying stream.)
- NO `HashMap`/`HashSet` in any canonical, serialized, RNG-seeding, or iteration-order-sensitive path.
  Use `BTreeMap`/`BTreeSet`. `children` is a `Vec<HumanId>` in birth order.
- NO `f32`/`f64` in RNG, canonical state, hashing, or equality/ordering paths. Floats are
  non-associative and platform-variant. Use integers or fixed-point (`Permille(u16)` for
  probabilities/rates; `u16`/`u32` for health/xp). Floats are allowed ONLY in `tui` rendering.
- bevy: entity iteration order is NOT guaranteed. Before any canonical/ordered operation, collect and
  `sort_by_key` on a stable domain id (`HumanId`, never a bevy `Entity`). Use a fixed timestep and
  make system order explicit with `.chain()`; do not let parallel systems touch RNG unordered.
- No wall-clock/system time in `core`/`sim`. Time is a simulation tick (`u64`).
- `Phenotype` is expressed exactly ONCE at birth and stored; never re-express a stored genome.
- Add at least one test that runs the sim twice with the same seed and asserts identical output.

## gosh-mode influence flow (the ONE way the world changes by a user)
TUI `cast_gosh` -> `PendingEvent{author: God}` pushed to a thread-safe intake -> the events system
drains it at the START of the events phase, assigns a monotonic `Seq` -> `core::resolve` (deterministic,
no RNG for goshes) -> `EventRecord{author: God}` appended to the canonical log. Navigate + deep-dive
never enqueue anything. Replay reads god events straight from the log at their recorded `(tick, seq)`.

## Error handling
- NO `unwrap()`/`expect()`/`panic!`/`todo!`/panicking-index in non-test code.
  Return `Result<_, E>` with `thiserror` enums in libs; `anyhow` only in `app`. Propagate with `?`.
- `unwrap`/`expect` are fine inside `#[cfg(test)]`.

## Branches, commits, and pull requests — NEVER commit directly to `main`

`main` must read as a clean sequence of meaningful, reviewable units. Every change reaches it through
a feature branch and a PR. No exceptions, including for one-line fixes.

**Branch per coherent unit.** Name it `feat/<area>-<thing>` — `feat/core-genetics`,
`feat/sim-tick-loop`, `feat/mind-event-validation`, `feat/tui-inspector`. Use `fix/…` for repairs and
`chore/…` for scaffolding. One branch is one reviewable idea, not one afternoon of unrelated work.

**Commits are professional chunks.** Imperative subject under ~72 chars, prefixed with the crate or
area. Then a blank line, then a body that says what changed, why it changed, and how it was verified.
No `wip`, no `fixes`, no `more stuff`, no noise commits. If a commit is not worth explaining, squash
it into the one that is.

```
sim: draw learning through the keyed RNG in HumanId order

Learning previously consumed RNG straight out of a bevy query, so the result
depended on ECS iteration order and diverged between thread counts. Collect
and sort by HumanId before drawing, keyed on (domain, tick, subject, salt).

Verified: cargo fmt --all; cargo clippy --all-targets --all-features -D warnings;
cargo test --workspace green, including no_rng_from_iteration_order.
```

**The exact sequence, every time:**

```sh
# 1. start from an up-to-date main — never work on main itself
git switch main && git pull --ff-only
git switch -c feat/sim-tick-loop

# 2. work in small commits; run the full loop before each one
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
git add -A
git commit                      # subject + body as above

# 3. publish the branch and open the PR
git push -u origin feat/sim-tick-loop
gh pr create --title "sim: deterministic tick loop" --body "$(cat <<'EOF'
## What
Adds the eight chained tick systems and the per-tick canonical world hash.

## Why
<the design reasoning — why this shape, and what it rules out>

## Verification
cargo fmt --all / cargo clippy --all-targets --all-features -- -D warnings / cargo test --workspace
<paste the test summary>
EOF
)"

# 4. merge so main stays a clean sequence of meaningful units
gh pr merge --squash --delete-branch
#   use --merge instead of --squash when every commit on the branch is
#   independently meaningful and worth keeping in main's history

# 5. return to main for the next unit
git switch main && git pull --ff-only
```

**Authorship.** Commits are authored as Janusch only — the repo's configured git identity. Do NOT
pass `--author`. Do NOT add a `Co-Authored-By` line, and do NOT add an assistant/tool attribution
trailer of any kind. No trailers at all. Keep messages plain and factual: what changed, why, how it
was verified. A commit hook enforces this and will reject a commit that violates it.

**The same rule applies to the PR title and body.** A squash merge takes its commit message from the
PR title and body, and that message is composed on the server where the local hook cannot see it —
so an attribution line in a PR body lands in `main` unchecked. Keep PR text as clean as commit text.

If the remote or `gh` auth is not configured, stop and tell me. Do not fall back to committing on
`main`.

## When to spawn subagents (delegation)
- Default: do the work yourself in this thread (keeps the build in one Session ID).
- Delegate READ-ONLY reconnaissance when you need to map an unfamiliar area or read many files.
- Delegate a self-contained, well-specified module only when it is truly independent; give it the exact
  contract + "must pass fmt/clippy/test".
- After a substantial change, run a self-review pass (`codex exec review --base main`) before merging.
- Keep delegation shallow (depth 1). Always wait for results and integrate + re-run the loop yourself.

## Do not
- Add dependencies casually — justify each crate. No web/browser/image tools; this is a Rust build.
- Refactor unrelated code, churn formatting, or "improve" things outside the task.
- Write prose visualizations or long status essays. Be terse. Let `cargo test` be the proof.
