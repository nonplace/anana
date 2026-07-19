# The soul of AnanA

AnanA is technically a simulation, not a game. But it lives or dies by game-design
discipline, so we hold to game principles. This file is the essence we protect:
when a design decision is unclear, it must serve what is written here. Everything
else is negotiable; this is not.

## 1. One world, one truth

There is a single **canonical world**. It is the source of truth. Every observer
sees the same world, the same state, the same history. The world is never
instanced per user; it is shared. What is true for one god is true for all.

## 2. The future is not precomputed — a fast-forward is a simulation of the simulation

The canonical world advances tick by tick, at its own cadence, generating each
moment live: stochastic draws resolve, non-scripted events are authored, and every
result is recorded. Because the future has not happened yet, there is nothing to
"fast-forward" *to*.

Any attempt to see ahead is a **projection**: a separate simulation, drawn
independently, that necessarily diverges from what the canonical world will
actually produce — and diverges between any two observers who run it. A projection
is never canonical. Never confuse the map you rolled forward with the territory
that will actually unfold.

The past is different: it is fully determined and replayable, because it is in the
log. A god may scrub and replay history (canonical); a god may one day project the
future (speculative, and labelled as such); a god may never rewrite the one true
timeline.

## 3. Determinism is what makes the shared world possible

Same seed → same world, tick for tick. The **event log is the deterministic
spine**: an identical world must be reconstructable from seed + log alone, with the
network switched off. This is not a testing convenience — it is *why* everyone can
share one world, and why a life once lived can be replayed exactly.

## 4. gosh-mode — the god observes, and creates goshes

The user is a god. gosh-mode gives the god exactly three affordances:

- **Navigate** — move through the world and its history. (Observation. No influence.)
- **Deep-dive** — open a single human and study their whole inner life: genome,
  instincts, consciousness, skills, lineage, story. (Observation. No influence.)
- **Create a gosh** — the god's one and only lever on the world.

A **gosh is an act of the god**: a god-authored event injected into the world — a
blessing, a plague, a catastrophe, a gift of a skill or a gene, a nudge to
fortune. It is the *only* mechanic that changes world state. Everything else the
god does is watching.

A gosh flows through the **same event pipeline** as every other event (authored by
`God` rather than the engine or the AI), is resolved by the pure core, and is
**recorded into the canonical log** — so a gosh is persistent, and replays exactly.
In a future fast-forward projection (§2), a gosh would touch only that speculative
branch, never the canonical world.

_(The exact v1 form of a gosh is being refined; the invariant is: goshes are the
sole influence, they are god-authored events, and they are recorded.)_

## 5. Emergence over scripting

Interesting lives must *fall out of* the mechanics. Prefer a few simple,
well-understood systems that interact over hand-authored storylines. The god
creates conditions (goshes); the world creates meaning.

## 6. Functional core, observable shell

The domain logic — genetics, learning, event resolution — is pure, deterministic,
and heavily tested. The game loop, the AI, and the terminal UI are the thin shell
around it. Anything that touches the network or the screen lives at the edge.

## 7. The event log is the single spine

Every change to the world is a recorded event. That one log is simultaneously the
**audit trail**, the god's **observability feed**, the substrate GPT-5.6 **narrates**
from, and the record that makes **replay** exact. One mechanism, four jobs.

## 8. A life is made of containers

Every human carries **instincts** (the animal past), **consciousness** (the human
differentiator — what a virus and an animal lack), and a **body**. The combinatorics
across these containers, seeded by genome and shaped by experience, make every life
unique. **Recall** is a learned skill, and one of the first: before a human learns
to remember, experience cannot compound.

## 9. GPT-5.6 lives at the edge

GPT-5.6 narrates lives and authors non-scripted events. It never computes outcomes
— the core resolves everything. Its authored events are recorded with their result,
so the world stays deterministic and replayable even with no network and no key.
