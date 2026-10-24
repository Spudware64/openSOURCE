# Chamber 06: parented panels and portal placement

The original `testchmb_a_06.bsp` uses nested, start-open `func_door` pistons with parented `func_brush` panels. It does **not** use `func_movelinear` for these structures; changing that class's travel calculation would not fix this map.

Three independent faults were reproduced with failing regression tests:

- **Extended-looking panels at spawn:** the GPU was given raw brush origins, while collision resolved the parent hierarchy. For example, panel model 22 rendered at `(536,128,576)` instead of the resolved retracted position approximately `(-32,128,576)`. Scene origins now resolve brush translations at load and each animation update; the native GPU model and visibility buffers consume those origins. MDL offsets use the same buffer origin so shared parent slots do not double-apply motion. Initial native/map-session collision also uses resolved transforms.
- **No staged activation:** `trigger_look` was skipped entirely. The map's triggers require 0.2 seconds of continuous gaze toward `target_move_pillar1`/`target_move_pillar2` while inside their brush volumes. Gaze tracking now resets when looking away or leaving, respects the authored dot-product FOV, and dispatches existing outputs. Existing door speed, nested transforms, delayed completion outputs and output counts continue to drive motion. The implemented gaze subset also handles fire-once and velocity-direction flags; timeout output semantics remain outside this change (this map has `Timeout=0`).
- **White panel shots rejected:** portal surfaces loaded only world model 0. Solid submodel faces now retain their model ownership and are posed using the current runtime hierarchy for player/automatic placement. A panel previously missed in favour of world `metal/metalgrate018b` accepts blue at approximately `(136,256,448)`. After piston movement, a travelling blue shot places on the moved panel at approximately `(320,128,448)`, after arrival, matching brush collision. Local, unposed submodel faces cannot contaminate standalone world fitting.

Chamber 06 intentionally grants **blue only** (`[true,false]`); orange was not enabled. This is still bounded modular-concrete/rectangular fitting, not Valve gameplay-DLL parity. Portal attachment/fizzle behavior when an already-occupied supporting panel subsequently moves is not implemented or claimed here. Arbitrary runtime brush rotation rendering and nonzero `trigger_look` timeout semantics are not established by these translation-only chamber regressions.

## Verification

- `cargo test --test chamber06 --test chamber06_render --test portal_fitting --test portal_shots --test portal_orientation --test portals --test weapon_probe --quiet` passes.
- Additional animation, door geometry, parented support, dynamic geometry, progression and GUI-map tests pass.
- `cargo test --quiet --no-fail-fast`: **219 passed, 3 failed, 1 ignored**. All three failures were separately reproduced from detached baseline `9f647d1`, without these changes:
  - `chamber00_exit::authored_second_room_open_gate_crosses_fitted_blue2`
  - `gun_pickup::actual_weapon_pickup_fires_its_exit_outputs_once`
  - `rigid::blocked_maker_does_not_embed_or_fire_spawn_output`
- A real macOS Metal window ran a scripted stationary-camera smoke for 600 frames, captured its rendered surface and successfully grabbed/released the native cursor. The authored gaze outputs moved piston models 21 and 30 to their closed endpoints. The screenshot was inspected; it is a GPU-surface capture, not desktop capture or physical mouse verification. This does not establish Windows execution or a complete chamber playthrough.

Example native render smoke (the process remains open after capture):

```sh
SOURCE_SMOKE=1 SOURCE_SMOKE_POSITION='704 128 88' \
SOURCE_SMOKE_ANGLES='-33 180 0' SOURCE_SCREENSHOT=/absolute/path/chamber06.png \
cargo run -- --game ../portal-original/portal --map testchmb_a_06 --play
```

Game assets remain external and read-only. Captures, logs and build products are not committed.

## Third-room floor crossing (2026-10-24)

The supplied recording shows the player stopped at approximately
`(606.9,123.7,-192)` over the authored orange floor portal. A failing regression
reproduced the same stop at `z=-191.96875` when blue was fitted to third-room
parented panel model 22. Placement accepted that panel, but player sweep,
overlap, rim and destination-clearance queries only cut apertures in world-model
brushes. Consequently the destination panel was treated as an obstruction and
the source floor stayed solid.

Those queries now include tracked, posed solid BSP brush submodels. Extra
collision blockers and rigid objects still obstruct an exit; the aperture remains bounded by the existing fit and rim
checks. Overlap queries also share destination-clearance results within one
query instead of repeating the exit scan for each backing layer. The cache is
not retained across queries or world changes.

`tests/chamber06_portal.rs` loads the actual complete map through `GameRoot` and
checks the recorded floor approach against the initial and moved parent-panel
poses. Both cross exactly once into a clear exit. The unit cache regression
requires one exit scan for 128 backing layers and checks that a subsequent query
sees a newly blocked destination. Existing portal, rejected-exit, rim, held-floor
and chamber 06 tests pass. The first broad run caught an extra blocker being
misclassified as portal backing; restricting eligibility to tracked BSP brush
IDs fixed that regression, and all eight cube portal regression tests then passed.

A temporary native staging driver placed this floor/panel pair and the recorded
approach pose, then used the normal interactive runtime, controller and renderer.
The Intel Iris Xe GL capture recorded a crossing and 945 complete warm frames:
median frame gap **16.696 ms**, simulation **3.851 ms**, controller **1.285 ms**,
and portal-rim work **0.459 ms**. This establishes a bounded staged crossing near
60 FPS, not a mouse-driven third-room playthrough or proof that every low-FPS
case has the same cause. The temporary driver was removed; original assets,
recording, frame data and generated images remain outside Git. Portal attachment
or fizzling after its backing panel subsequently moves remains outside this fix.

Final verification: `cargo test --offline --quiet --no-fail-fast --
--test-threads=1` reported **242 passed, 1 failed, 1 ignored**. The sole remaining
failure is `pedestal_render::actual_pedestal_child_mesh_follows_rotating_parent`,
also reproduced before this fix with the separate pending GPU rotation changes.
Its CPU-vertex assertion is not changed or bundled into this commit. The new
chamber 06 crossing, cache and all cube portal regressions pass. The final native
binary was rebuilt after removing the temporary staging driver. This run is not
claimed to be fully green; the earlier historical failure records remain intact.
