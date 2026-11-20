# Genesis 3D graphics benchmark — 90 FPS target

This is the first separate **Godot 4.5.2** visual prototype, not a replacement
for the existing browser game. It tests 3D terrain, GPU-instanced trees,
GPU-instanced soldiers, an orbital camera, native-resolution 3D, and a
frame-time HUD. Nothing here alters a Rust simulation rule.

## Open and run

1. Install [Godot 4.5.2](https://godotengine.org/download/archive/4.5.2-stable/)
   or a compatible 4.5.x editor.
2. From the repository root, produce a recording of *actual* Rust Frontier
   positions and terrain:

   ```bash
   cargo run --release --bin scene_export -- godot/scene_data/frontier_demo.json
   ```

3. In Godot, import **godot/project.godot**, then press **F6/F5** to run.
   The project uses no external art assets.

Without the generated JSON, the Godot scene is still launchable, but the HUD
labels it **VISUAL PREVIEW (NO RUST SNAPSHOT)**. Its two moving preview armies
are NOT produced by the authoritative gameplay engine. Once exported, the HUD
labels it **RUST FRONTIER REPLAY — READ ONLY**.

### Controls

- **Left-drag / one-finger drag:** pan the world.
- **Right-drag:** orbit camera.
- **Scroll / two-finger pinch:** zoom.
- **Q:** cycle rendering resolution (100%, 85%, 70%).
- **Space:** pause or resume the replay.

## What a recording is (and is not)

`src/bin/scene_export.rs` uses `genesis::scene::SceneSource` to convert a
deterministic Frontier run into a JSON recording. The recording contains one
static tile atlas and 421 tick-stamped sets of **two real army positions**.
Godot uses fractional frame interpolation **only for display** between these
integer positions, without changing their orders or their simulated losses.

The 48 capsules in each formation are **visual representatives**, not 48 new
Rust agents. Forest and terrain vertex heights are **presentation geometry**,
not new physical terrain or pathfinding. No claims are made about individual
procedural gait, real-time Rust/Godot interoperability, or a playable multiplayer
session yet. Those are separate milestones.

## Performance acceptance criteria

The agreed **target is 90 displayed FPS** on a device with an actual 90 Hz
(or faster) refresh mode. This gives a nominal 11.11 ms/frame budget.
The engine MUST be profiled on the real target device before claiming success.

Quality gate for a 90-second warm run (not yet measured):

- 90 FPS median or better on a 90+ Hz display with the 90-FPS refresh mode
  available to the app.
- p95 frame-time **<= 11.1 ms**, excluding cold asset loading; report median,
  p95 and worst frame separately.
- 100% 3D render scale preferred. The benchmark may temporarily drop to
  85% or 70% if multiple 1-second windows exceed frame budget.
- Camera pan/zoom and unit interpolation remain visually smooth and do not
  modify the Rust authoritative positions.
- Measured on target Android hardware and compared against a desktop baseline.
- Never claim 90 FPS on a 60 Hz panel; the HUD shows the refresh limitation.

Engine.max_fps is uncapped so 90/120 Hz devices can present at their display
rate. Vsync and the operating system still determine actual cadence.
The first benchmark disables dynamic shadows intentionally to preserve GPU
headroom. High visual quality is a separate **art direction and materials**
milestone, not proven by this first project.

## Graphics topology

```text
Frontier World (Rust / authoritative)
   -> SceneSource::scene_frame()
      -> scene_export.rs (record state, no visual side effects)
         -> Godot scene_data/frontier_demo.json (static replay)
            -> terrain mesh (batched)
            -> trees (MultiMesh x2)
            -> two army formations (MultiMesh)
            -> orbit camera + interpolation
            -> HUD (FPS, p95, resolution, display refresh)
```

The future **live connection** should replace the recording with a
GDExtension/native IPC adapter and a validated command port. It must never let
render transforms drive simulation coordinates.

## Validation

- `cargo test --bin scene_export` (determinism and expected army payload).
- `cargo test --lib` (existing simulation behavior and scene contract).
- GitHub Actions' Godot headless import checks whether the scene and script
  load successfully; **headless success does not measure graphical quality**.
- Device playtest is mandatory for the 90 FPS claim.

## Priorities after this benchmark

1. Fix any CI parsing/runtime issues.
2. Measure real Android frame pacing (target 90 FPS, p95 <= 11.1 ms).
3. Increase terrain and materials fidelity within the measured GPU budget.
4. Introduce a live read-only Rust scene adapter and command transport.
5. Add animation/formation behaviors and semantic zoom.
