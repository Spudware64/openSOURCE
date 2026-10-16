# Animation stall capture (Windows PowerShell)

Set an explicit writable path before launching the normal executable with the same map and arguments you already use:

```powershell
$env:SOURCE_FRAME_PROFILE = "$PWD\animation-frames.csv"
# Launch the game normally from this PowerShell session.
```

Stand still for 5 seconds, activate the button, wait until movement stops,
then repeat with cube/button, dropper and gun pickup separately. Leave another
5 seconds idle before quitting. Send the CSV and the map/action sequence.
Remove the environment variable to disable: `Remove-Item Env:SOURCE_FRAME_PROFILE`.
The path is overwritten at startup. Parent directory must already exist.

One-second batches contain separate `static` (no changed animated vertex ranges)
and `active` rows, per-stage average/max CPU wall milliseconds, actual changed
instance/vertex/write/byte counts, simulation tick count, world rebuild and
Rapier scenery rebuild counts, and current-frame work histograms. No samples
or disk writes per tick. The last partial second is not flushed on exit.
`frame_work` is redraw CPU work through present, not event-loop idle time or FPS.
Nested runtime/world/collider durations overlap simulation and must not be summed.
`simulation` includes player stepping, portal sync and camera preparation.
`animate` includes the preceding tiny camera uniform write; `portal_encode`
includes command encoder/view creation. Capture time is separate: turn off
screenshot/demo/route capture flags when measuring interactive stalls.

These are NOT GPU timings. Submit is asynchronous. Acquire/present/submit wall
stalls can indicate driver/backpressure, not identify GPU shader cost. No GPU
poll waits, timestamp queries, animation skipping or rate caps are inserted.

Source inspection found Rapier `sync_world` removes/recreates *all* scenery
convex hulls whenever any brush plane changes. It does not establish that this
causes the reported skeletal-animation stall: correlate `collider_rebuilds`
and `collider_rebuild_nested_max_ms` with active rows before changing physics.
If animate dominates, inspect skinning; if vertex_upload/submit/acquire dominates,
compare Intel Vulkan driver behavior with static rows before another CPU microfix.
