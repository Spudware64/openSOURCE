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
