# Visible-portal performance investigation

## Status

The Windows Intel/Vulkan severe stall is **not reproduced or fixed** here.
The user tested release as well as debug and reports both laggy. The earlier
invisible-aperture fix does not address the remaining visible-portal case.
This change adds diagnostics, not a speculative quality/resolution reduction or
rendering optimisation. Original assets were read-only; no original executable
was run. Local GPU measurements are Apple M4/Metal only.

## Shipped Source/Portal evidence (REA, not public source)

REA/Ghidra 12.1.4 imported the supplied x86 PE binaries ephemerally:

| Input | SHA-256 |
| --- | --- |
| `bin/engine.dll` | `6a277234ae8edb4939ed7ef45a27499210f14db3a7f6a9e7e83540492b638d6e` |
| `portal/bin/client.dll` | `59abd9eea5c6a6a5a48bd5be3903b66bc97e7d2e1c893ac521d055fcea2da3d5` |

Existing controller notes were reused for artifact identity, not as evidence for
rendering. No matching live analysis existed. Names below are diagnostic labels
or inferred roles, not recovered function symbols. Full local retained records:
`evidence/portal-performance-rea-engine.json` and
`evidence/portal-performance-rea-client.json` (not committed).

### Observations

1. **Portal-specific geometric frustum construction.** Client string
   `r_portal_use_complex_frustums` at `0x103fe1fc` leads through registration
   bytes at `0x1006f080` to ConVar object `0x1051cdd0`, parent pointer
   `0x1051cdec`, then its reader `0x1023f1c0`. That routine clips a four-corner
   polygon repeatedly against existing planes, rejects fewer than three surviving
   vertices, forms edge/eye cross products, normalises planes, transforms them,
   and appends linked-portal and existing depth planes. It includes a proximity
   fallback (squared distance 2916) and facing/link checks; it is not simply
   rendering everything at a smaller resolution.
   Evidence `ev_74bea8c57665af90d03e5567e53504f13ad24651f8e45dbe6075917922f0fb9d`.
   Stack recovery has an explicit warning; exact structure layouts remain inferred.
2. **Those planes reach an actual portal-view path.** Caller `0x102429c0`
   saves 24 words of the current view planes, calls the above routine, maps the
   eye and view basis through a transform, installs a destination plane through a
   render-context virtual call, conditionally replaces the view planes, increments
   portal depth, renders via `0x101e6870`, and restores the old state and clip
   plane. `0x1023f090` supplies a bounded list of up to four portal-corner origins
   (capacity check 32) and additional visibility data to that path. These are
   observed calls/data flow; unresolved virtual method names are not asserted.
   Evidence `ev_16d5f4620c1356332e4fcdac491a244e9218e8da102de1d46645c6767f55cfde`
   and `ev_db968d9e6919afbf273ca0e6357b68af44cfedff17860a2f1295cecb60e80afc`.
3. **PVS union/caching, not unrestricted whole-world submission.** Engine
   `0x1017c490` is diagnostic-linked to GetPVSForOrigin and calls `0x10195e80`.
   The `r_novis` registration/parent reference leads to `0x101e65a0`, which calls
   the same helper for multiple origins/clusters, ORs their bitsets, tracks a
   visibility generation, and searches cached cluster sets. Its bypass marks
   all nodes/leaves with the current generation. The exact decompressor was not
   traced; this establishes a visibility-bitset pipeline, not its entire format.
   Evidence `ev_51b544e4ff98d3f6021c96d8d4bb165cdbee9d40266d1043f0e1e89ae84d06bc`
   and `ev_d8db5d8fe936e156788ef25efba8d2db1da2341b03ad1e0b519d0fca62a4764f`.
4. **World traversal rejects before surface submission.** The
   `r_frustumcullworld` registration identifies parent `0x105c1c44` read in
   `0x10123260`; it selects a plane mask `0xf` versus the disabled sentinel,
   then calls `0x10124230`. That recursive routine first requires the node's
   visibility generation to match, can return on a cull-helper result, visits
   children in eye/plane order and gates surfaces by a bitset and side test
   before `0x10123a70`. Exact cull-helper internals remain untraced.
   Evidence `ev_bd1a8d7ca178da5b7015539a796ad7f6b5629759c9ac7e188e496d20f0289a01`
   and `ev_8e0f1600a051bad7bf1e61ff44fc14f16b681546c65e327212d66772976d508a`.
5. **Other layers exist, but are only partly traced.** Client has
   `r_portal_use_stencils`; its parent is tested alongside render capability and
   portal-depth state in `0x1023ec40` / `0x10240620`. Engine diagnostic-linked
   `COcclusionSystem::IsOccluded` (`0x10144cc0`) performs size thresholds,
   projects box corners and queries an occlusion structure. This is evidence
   for stencil/occlusion mechanisms in the shipped build, **not** proof of their
   enablement, exact virtual API identities, portal-specific occlusion-query
   scheduling, or measured savings in the user's scene.
   Evidence `ev_02540773795f97bff0de4d022d23416362df438940cca81cff03e1b2935e1f64`,
   `ev_f02ca7d32d2fbf75a0cd4973b2ca32045c203a7da2088a9e11831280cef22c64`,
   `ev_dac65fc7fd6a22d12fffcb5728c36f5c8701e58392bc4a0c13e032cf1161e55c`.

**Inference:** a faithful direction is view-specific visibility and bounded
geometry submission plus portal clipping, rather than only an aperture on/off
check. Source's portal frustum, PVS and culling are complementary layers.
This is not yet a validated Rust design: moving/skinned geometry, portal-edge
crossings, material order, PVS unions and a submission/batching budget must be
covered before changing production rejection. Static REA does not establish
original frame times, Intel behaviour or a complete renderer reconstruction.

## Rust workload measured on actual `escape_02`

`examples/portal_render_profile.rs` uses the production `GpuScene`/`PortalGpu`
modules, one reused device/scene, five warmup frames and bounded repeated
submissions. No map/texture upload is included in measured frames. It deliberately
waits for each submitted frame, excludes simulation, surface acquire/present and
readback, and reports CPU encode plus completed **wall** time, not GPU timestamps
or playable FPS. The pair is staged in front of authored spawn, not the user's
exact pose or a legal-placement/gameplay reproduction. Assertions require zero,
one and zero remote passes in the none/visible/offscreen phases.

Release M4/Metal, 80 samples per cell:

| Target | No portals completed median ms | Visible linked view | Offscreen linked pair |
| --- | ---: | ---: | ---: |
| 640x360 | 9.112 | 9.387 | 9.229 |
| 1280x720 | 9.104 | 9.386 | 9.234 |

Visible encode medians: 0.307 / 0.302 ms. Completed visible maxima:
17.316 / 17.174 ms. The severe Windows interaction did not occur in this probe;
resolution independence here does not rule out fill/driver problems on Intel.

- Loaded mesh: **4,230,378 vertices / 1,410,126 triangles**.
- Per world view: **313 scene draw calls**, including two empty pellet draws;
  **310** are individually sorted translucent triangles. The opaque call still
  submits all geometry, with blended material rejection in the fragment shader.
- Every admitted remote view repeats this mesh and sorting. Its texture has
  full target dimensions; the world exit-plane rejection is a fragment discard,
  not CPU geometry rejection. There is no per-view world PVS/frustum filtering
  in `GpuScene::draw_view`. The GPU's ordinary clip stage still works, but only
  after vertex processing.
- One-time diagnostic triangle classification (not used to hide anything):
  main trivially outside full camera frustum **863,259**; remote **845,850**;
  wholly behind exit plane **378,207**; rejected by either remote condition
  **846,029**. Conditions overlap and must not be added. These are conservative
  trivial triangle tests, not an occlusion/PVS estimate or a portal-tight frustum.

A separate bounded ordinary native release launch at authored spawn recorded
1,395 frames / 23 flushed buckets: weighted CPU frame-work 16.537 ms, simulation
5.442 ms, acquire 9.878 ms, main encode 0.232 ms; worst frame-work 68.147 ms.
No linked view was active. Main counters include the world **and viewmodel**:
316 draws / 4,257,945 submitted vertices, two `GpuScene` calls per frame.
Collider rebuild average 2.897/frame warrants investigation separately; it is
not evidence that portal visibility causes those rebuilds. Native acquire can
contain vsync/GPU backpressure. The process was deliberately terminated after
25 seconds; this was not a full gameplay route or physical-input smoke.

Local raw outputs: `evidence/portal-render-escape02-final.log`,
`evidence/portal-render-escape02.csv`, `evidence/escape02-native-profile.{csv,log}`.
No before/after speedup is claimed: rendering is unchanged.

## Runnable diagnosis on the failing machine

PowerShell, from the repository (replace the game root):

```powershell
cargo run --release --example portal_render_profile -- 'C:\Games\Portal\portal' escape_02 120
$env:SOURCE_FRAME_PROFILE = "$PWD\portal-native-profile.csv"
cargo run --release -- --game 'C:\Games\Portal\portal' --map escape_02 --play
# Play the failing visible-portal sequence, then close the window normally.
Remove-Item Env:SOURCE_FRAME_PROFILE
```

Record the printed adapter/backend, exact camera/sequence and physical window
size alongside the CSV. `SOURCE_FRAME_PROFILE` adds no GPU waits in gameplay.
New counters split main/remote `GpuScene` calls, API draw submissions and
submitted vertices, plus translucent triangle draws; main includes the viewmodel.
Counts are submissions, not shader invocations, visible triangles or GPU time.
Existing CPU timing/upload/collider counters remain available. CSV buckets can
mix camera states and classify static/active by changed animation instances, not
by portal state. Correlate remote-view counts with the stalled intervals.

Ranked remaining hypotheses:

1. **Repeated full-map vertex/geometry workload**: supported by counts and
   original-engine architecture. Test bounded geometry rejection/batching with
   unchanged pixels and moving-geometry tests, measuring encode and GPU time on
   the failing hardware. Do not replace one opaque draw with thousands blindly.
2. **Remote fill/overdraw, clip discard or driver stalls**: still possible on
   Intel. Compare the fixed-content diagnostic's two target sizes and original
   gameplay CSV; no production resolution change is included.
3. **Unrelated simulation/collider/upload bursts**: inspect the existing nested
   CPU counters versus remote-pass admission during the actual interaction.
   Renderer-only success cannot rule out gameplay stalls.

## Verification and limits

New counter test was first observed failing (missing helper), then passed.
Conservative diagnostic rejection test likewise failed first, then passed.
`cargo test --lib`: 18 passed; `cargo test --example portal_render_profile`:
7 passed (includes shared-module tests); `cargo test --test portals`: 15 passed.
The release diagnostic exercised all six real-asset render cases.

The full suite is **not green**: `cargo test --quiet` stopped at
`chamber00_exit::authored_second_room_open_gate_crosses_fitted_blue2` (cube did
not change the actual door transform), also documented by the concurrent
chamber06 work. Separately, `cargo test --test rigid` reproduced the AGENTS.md
baseline `blocked_maker_does_not_embed_or_fire_spawn_output` (left 1, right 2;
14 other tests passed). Neither unrelated gameplay failure is changed here.
Strict Clippy (`cargo clippy --example portal_render_profile -- -D warnings`)
is also blocked by existing warnings in `window.rs`, `player_query.rs`, `play.rs`,
`controller_trace.rs` and the pre-existing range loops in `frame_profile.rs`.
No unrelated lint cleanup is bundled.
