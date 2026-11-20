# Genesis scene contract v0.1 — architecture topology

## Core principle

The simulation is authoritative; rendering is a read-only projection. A player
command goes into the relevant Rust simulation, not into a mesh, animation,
camera, or visual position. The renderer observes ticks and draws state.

```text
Player input -> validated gameplay command -> Rust World -> tick
                                                    |
                                                    v
                         SceneSource::scene_frame() [READ ONLY]
                                                    |
                           +------------------------+------------------+
                           |                        |                  |
                       2D Macroquad             future Godot        future replay/
                       presentation             3D adapter         telemetry viewer
                           |                        |
                        visuals                  visuals
```

## Current actual systems

| Simulation | Authoritative module | Presently observed by | Status |
| --- | --- | --- | --- |
| Frontier kingdoms and armies | `src/frontier.rs` | `src/bin/frontier.rs` | Implemented |
| Individual agents, resources, camps | `src/society.rs`, `src/settlements.rs` | `src/bin/society.rs` | Implemented, separately |
| Tactical battles | `src/frontier_combat.rs` | `src/bin/tactics.rs` | Separate, not adapted yet |
| Voxel universe | `src/core.rs` | `src/main.rs` | Separate, not adapted yet |

**Do not confuse a common rendering contract with one shared simulation.**
There is no conversion of Society individuals into Frontier population counters
and no claim that those prototypes share resources or a common tick rate.

## New interface

`src/scene.rs` defines `SceneSource::scene_frame(&self) -> SceneFrame` and
implements it for both `frontier::World` and `society::World`.

The snapshot contains:

- a schema version, source mode, seed and authoritative tick;
- a row-major grid and its **topology**, including bounded vs toroidal edges;
- tile surfaces, existing stock/capacity where meaningful, and real ownership;
- stable, namespaced entity IDs, grid positions, and typed details (troop count,
  agent energy, camp stores).

The output is immutable *derived data*. In particular:

- Frontier farmland is not mapped to a fictitious per-tile food stock.
- Society social groups are not mapped to territory or government.
- Camps are public infrastructure, not invented private property.
- Render positions are logical grid coordinates; terrain elevation is **not
  fabricated** from a terrain category.
- A 3D consumer may map `(x, y)` grid coordinates into engine coordinates
  `(x, elevation, z)`. Engine-specific coordinates do not enter simulation.
- Visual interpolation may occur **between** tick snapshots, but must not write
  interpolated results back into Rust world state.

## Boundaries still missing

1. Explicit `Command -> World` ports for each simulation (Frontier already has
   commands; Society uses a different API). Keep semantics distinct until unified.
2. Batched or incremental scene updates for larger maps. v0.1 copies the map on
   request and should be sampled only at authoritative ticks / on demand.
3. A consumer in the existing 2D client, and an optional 3D/Godot consumer.
   This PR **does not upgrade graphics**.
4. Actual elevation/geometry generation, lighting, materials, procedural
   animation, and a flexible camera.
5. Shared-world rules that make a persistent individual affect a kingdom, and
   the reverse. This needs explicit design and game-design experiments.
6. Server-authoritative multiplayer, rollback/resimulation, and network deltas.

## Proof obligations

Run `cargo test --lib scene` or the full `cargo test --lib`:

- a scene read cannot modify the authoritative world;
- repeated reads of the same state have identical content;
- terrain and tile ownership are not silently reinterpreted;
- a dead agent isn't rendered as alive;
- camps have the observed store amount, and no invented ownership;
- modes preserve their own bounded/toroidal topology.

## Next integration milestone

Have **one existing renderer** read the new `SceneFrame` for a small subset of
its display (for example, Frontier army positions and faction ownership),
while keeping every existing command path and gameplay rule intact. Compare
matched seeds before and after the migration. Only after that should we begin
a Godot 3D renderer proof of concept.

**This v0.1 contract is an adapter, not the finished 3D engine.**
