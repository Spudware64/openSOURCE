# Travelling portal-gun shots

Interactive LMB/RMB shots now launch blue/orange additive energy bolts from an approximate world-space gun muzzle. White-hot heads, feathered trails and short muzzle blooms are procedural geometry. The existing original `fire1` viewmodel sequence and channel-specific firing sounds still run immediately. Portal placement waits for arrival: 3000 Source units/second with a 60 ms minimum visible flight. These values are an approximation, not measured original-game timing.

Invalid shots travel to the first traced brush/rendered face and produce a coloured impact ring and the existing rejection diagnostic on arrival. Valid shots retain existing surface fitting, inventory, brush obstruction and paired-portal overlap rules, rechecked at impact. A rejected shot cannot become valid by retargeting during flight. Multiple in-flight bolts resolve chronologically, including several arrivals in one tick. The bounded queue accepts at most 64 bolts. Reset/respawn cancels transient effects; map replacement creates a fresh game and queue.

The expanding 300 ms impact ring provides an opening cue; the portal itself becomes fully functional on arrival, rather than reproducing Source's original portal growth shader. This is not exact particle/shader parity. Original read-only `portal_pak_dir.vpk` inspection found `materials/sprites/portalgun_effects.vmt`: UnlitGeneric, additive, vertex colour/alpha, using `sprites/portalgun_effects`. Its sprite atlas and the original particle system are not decoded for this effect. Muzzle position uses camera-basis offsets, not a sampled animated MDL attachment. Bolts are drawn in the main world view with depth tests, not recursively through linked portal views.

## Verification

`cargo test --test portal_shots` covers travel/deferred placement, invalid surfaces, rejection snapshotting, chronological multiple shots/distance-dependent flight, reset cancellation, fading impacts, channel-coloured feathered geometry and short-range trail bounds. Additional relevant tests: `portal_fitting`, `escape_fire`, `portal_orientation`.

A staged native-window diagnostic (no physical input claim):

```sh
SOURCE_PORTALS=1 SOURCE_SHOT_DEMO=/absolute/evidence/path cargo run -- --game /path/to/portal --map testchmb_a_00 --play
```

PowerShell equivalent:

```powershell
$env:SOURCE_PORTALS='1'
$env:SOURCE_SHOT_DEMO="$env:TEMP\portal-shot-evidence"
cargo run -- --game 'C:\.programs\Steam\steamapps\common\Portal\portal' --map testchmb_a_00 --play
Remove-Item Env:SOURCE_SHOT_DEMO
```

This stages camera/inventory in chamber00, launches a valid blue shot and a brush-blocked orange shot, captures 11 actual GPU surface PNGs, and exits after frame 120. Logs show the blue bolt's 0.140 s flight and no portal before impact. Captures 0032 and 0092 show distinct travelling blue/orange energy heads and trails; 0042 shows blue portal arrival plus expanding impact ring. Smoke was exercised on Apple M4 / Metal. Windows Intel/Vulkan and physical mouse routing remain human checks.

Integration suite was run with `cargo test --no-fail-fast --quiet`. It is **not green**: `chamber00_exit::authored_second_room_open_gate_crosses_fitted_blue2`, `gun_pickup::actual_weapon_pickup_fires_its_exit_outputs_once`, and the documented `rigid::blocked_maker_does_not_embed_or_fire_spawn_output` fail. All three were independently reproduced against clean baseline commit `9005ba9` extracted to profile scratch. `cargo clippy --all-targets --quiet` exits successfully with existing warnings; this does not mean warning-free.

Keep evidence, logs, generated files and original game assets out of Git.
