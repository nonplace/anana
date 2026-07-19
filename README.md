# AnanA

**A world of living souls.**

AnanA is a tick-driven simulation of human life. Each person is not a scripted
NPC but a genuine agent with an inner life: instincts inherited from our animal
past, a consciousness that sets us apart from every other organism, and a body
that ages. They are born, they learn, they love, they pass on their genes, and
they die. You study life in **gosh-mode**.

> Conway's *Game of Life* had cells. AnanA has souls.

Read the name forwards or backwards — it's the same. So is a life, told from its
birth or from its death.

---

## What you can do — gosh-mode

As a god you have three affordances, and only one of them touches the world:

- **Navigate** the living world and its recorded history.
- **Deep-dive** any human — inspect their instincts, consciousness, body, skills,
  and the genes they carry and pass on — and read their story, a life narrative
  drawn from the event log.
- **Create a gosh** — a god's-eye intervention (a blessing, a plague, a gift of a
  skill or a gene). This is your *one lever* on the world; everything else is
  watching.

And the world lives on its own: mating and birth recombine genes stochastically so
every generation is new, and a **virus** spreads through the population on its own
logic — a second lifeform proving the world isn't hardcoded for humans alone.

## Design

The principles that hold AnanA together live in [`soul.md`](./soul.md): one
canonical world, a future that is generated (never precomputed), determinism as the
spine, and emergence over scripting.

- **A functional core, an imperative shell.** The domain logic — genetics,
  learning, event resolution — is pure and tested. The game loop and the terminal
  UI are the thin shell around it.
- **Deterministic by seed.** Given the same seed, the same world unfolds. Every
  run is reproducible; every claim is testable.

## Tech

Rust. A headless [Bevy](https://bevyengine.org) ECS as the simulation engine,
[ratatui](https://ratatui.rs) for the gosh-mode terminal interface.

## Status

In active development for **OpenAI Build Week** (July 13–21, 2026). Build and
usage instructions land here as the world comes to life.

## License

[MIT](./LICENSE)
