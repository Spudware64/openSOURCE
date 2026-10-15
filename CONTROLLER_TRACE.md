# Bounded controller trace (Windows / PowerShell)

Update and rebuild this commit (`cargo build --release`); old executables do not implement this switch.
From the repository directory, replace the game directory with your existing Portal directory:

```powershell
$env:SOURCE_CONTROLLER_TRACE = Join-Path $env:USERPROFILE ("controller-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".csv")
Write-Host "Trace output: $env:SOURCE_CONTROLLER_TRACE"
.\target\release\portal-bsp-viewer.exe --game "C:\Program Files (x86)\Steam\steamapps\common\Portal\portal" --map maps/testchmb_a_00.bsp --play
Remove-Item Env:SOURCE_CONTROLLER_TRACE
```

Output is exactly the path printed above (under your Windows user folder). New file only: existing paths fail clearly, never overwrite a prior capture. No parent directories are created. The game continues after recording stops. Recording starts with the first normal interactive simulation tick, including uncaptured standing time: click promptly and reproduce in the first 15 simulation seconds (1800 rows maximum). For chamber01 change `testchmb_a_00` to `testchmb_a_01`. Restart with a fresh filename for each run. Do not use smoke/demo environment switches.

Do one standing/walking run without pressing N, then a separate elevator run if reachable in the window. If not reachable, this bounded startup recorder needs a future delayed-start facility; it cannot record a late elevator encounter. Send the CSV plus console adapter information and explain which interval shows the problem. No physics behavior is changed.

Rows are per normal 120 Hz tick, not per rendered frame. `frame_dt` is unclamped wall frame delta (repeated across ticks in the same frame); `dt` is fixed simulation delta. Includes feet, velocity, grounded/crouched, eye height, physics eye, interpolated eye evaluated at each tick, input/capture/noclip, yaw/pitch, map, executable path and compile-time Git HEAD (unknown without Git). Build SHA identifies HEAD, not uncommitted changes. `render_eye_at_tick` is an interpolation diagnostic, not necessarily the final presented eye when multiple ticks occur in a frame. Vector/optional fields are quoted Rust debug text inside valid CSV.

`support_before` uses the runtime's carry eligibility and model support query before entity tick; `support_after` is an unconditional geometric query after movement, not a grounded-state assertion. Raw/resolved before/after origins refer to the SAME pre-tick support model. `runtime_displacement` is net player displacement across entity tick plus portal synchronization, before player movement; it is not a claim that all displacement was elevator carry. Resolved transforms are sampled on either side, not inside the entity transaction. World support has no model ID (`None`). Disabled tracing does not evaluate snapshot closures, resolve transforms, format rows, or touch files per tick; only option branches and trivial scalar copies remain.

The reported log has grounded feet Z160.03125, then Z175.481 before the deliberate `noclip true` / `noclip false` interval; later feet stabilize at Z128.03125. Noclip means that interval is not a clean walking-only reproduction, but does not invalidate the earlier height report or the elevator failure. These diagnostics are intended to distinguish feet/support motion from stance and render-eye motion without speculative controller changes.
