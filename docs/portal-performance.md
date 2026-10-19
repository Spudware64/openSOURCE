# Visible-portal performance investigation

## Status

The persistent acceptance case is **reopened**, not closed: sustained Windows
Intel/Vulkan `escape_02` 8 FPS and intermittent chamber06 crossings are separate
cases. The latest pass found and fixed two production CPU amplification paths
(free-body scenery scans and repeated per-sweep exit clearance); see the final
section for actual-asset measurements and the one-command Windows capture.
Neither fix is yet verified against the user's Intel frame rate.

Production now conservatively rejects static contiguous geometry chunks per view
before submission (see implementation/results below). Earlier sections describe
the diagnostic baseline, not the current submission path.

The Windows Intel/Vulkan severe stall is **not reproduced or fixed** here.
The user tested release as well as debug and reports both laggy. The earlier
invisible-aperture fix does not address the remaining visible-portal case.
The initial diagnostic change added instrumentation, not a speculative
quality/resolution reduction. Original assets were read-only; no original executable
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

## Implemented reduction 1: conservative static submission chunks

`scene_visibility.rs` caches AABBs for contiguous 3,072-vertex chunks at upload.
Each view rejects only boxes wholly outside one homogeneous camera plane or
wholly behind its world exit plane; touching/crossing boxes remain. Adjacent
surviving ranges merge, preserving original primitive order and one shared
vertex buffer. No per-frame vertex upload, texture/resolution change, or new
index buffer. Any chunk touching an animation range or nonzero brush/model
index is **never rejected**: moved, parented and skinned geometry cannot inherit
stale static bounds. Translucent sorting is unchanged in this first reduction.
This is full-camera/exit-plane rejection, **not** portal-tight frusta or PVS.

Actual `escape_02`, same authored-spawn staged pair, release M4/Metal:

| Opaque submissions | Baseline | Chunked |
| --- | ---: | ---: |
| Main vertices | 4,230,378 | 2,921,706 |
| Remote vertices | 4,230,378 | 2,940,138 |
| Main opaque draws | 1 | 30 |
| Remote opaque draws | 1 | 29 |

The 310 sorted translucent draws and two pellet draws remain per view. Submission
counts are not visible triangles or GPU invocation counts. Seven 640x360 GPU
readback comparisons against the full-range reference had **zero changed
pixels**: center, lateral aperture edges, near-plane approach, rotated, away,
and moved geometry. The moved case translates all 214 non-world model origins
and uploads translated vertices in all 76 animation ranges after GPU creation.
This exercises stale-bound safety, not a full gameplay animation route.

80-sample completed-wall medians, reference then chunked:

| Resolution | None | Visible | Offscreen |
| --- | --- | --- | --- |
| 640x360 | 9.213 → 9.100 ms | 9.422 → 9.385 ms | 9.196 → 9.218 ms |
| 1280x720 | 9.212 → 9.113 ms | 9.530 → 9.381 ms | 9.207 → 9.237 ms |

Visible encode medians: 0.308 → 0.312 ms / 0.321 → 0.313 ms. These tiny
wall-time differences are **not a demonstrated frame-time speedup**. The
benchmark excludes view-uniform updates/CPU selection (outside encode timing),
simulation, presentation and uploads. The established outcome is reduced vertex
submission with equal pixels, not a Windows stall fix.

Reproduce differential verification and reference measurements:

```sh
SOURCE_VERIFY_VISIBILITY=1 cargo run --release --example portal_render_profile -- ../portal-original/portal escape_02 80
SOURCE_REFERENCE_RENDER=1 cargo run --release --example portal_render_profile -- ../portal-original/portal escape_02 80
```

New bounds/range tests were observed red before implementation, then green.
Focused lib/profile/portal tests pass. Full `cargo test --quiet --no-fail-fast`
reproduces exactly the three AGENTS.md baseline failing targets: chamber00_exit,
gun_pickup and rigid; no other failing target. Local raw evidence (not assets or
committed captures): `evidence/portal-chunks-{before,reference,verified,full-tests,focused}.log`.

## Implemented reduction 2: reject invisible static translucent triangles

The same conservative bounds classifier now caches one bound for each static
translucent triangle. Per-view selection happens before the existing stable
back-to-front sort. Remaining triangles keep the same relative ordering and
original shared-buffer indices. Model/animation triangles always survive CPU
selection, exactly as for chunks. The full-reference diagnostic bypasses both
optimisations. No alpha/blend shader or depth state was changed.

Actual staged `escape_02` visible-pair workload, at both target sizes:

| Main + one remote view | Original full rendering | Both reductions |
| --- | ---: | ---: |
| Translucent draws (main / remote) | 310 / 310 | 4 / 6 |
| Total scene draws (including empty pellets) | 626 | 73 |
| Total submitted scene vertices | 8,462,616 | 5,861,874 |

That is **88.34% fewer draw submissions** and approximately **31% fewer opaque
vertices per view**, not a GPU-timing speedup claim. Main alone is 36 scene draws;
remote is 37, versus 313 each originally. Opaque chunks still contain blended
vertices rejected by the fragment shader; splitting that buffer is not included.

Again all seven real-asset GPU differential cases are byte-identical (zero
changed pixels), including translated model origins and changed animation
uploads. The new static-triangle classifier test was observed failing before
implementation, then passing. Native release surface-event-loop portal smoke on
`testchmb_a_00` exits successfully after 180 scripted frames: portal crossing
count 1, grounded at finish. Inspected actual surface capture
`artifacts/portals/portal-0060.png`: linked tiled room and props are visible inside
the blue aperture, surrounding walls/floor remain intact. This is a scripted
native check, not physical keyboard/mouse verification or an Intel check.

Latest 80-sample M4/Metal visible-pair results, original full-reference → both
reductions, with the same cameras/resolutions:

| Target | Encode median | Completed-wall median |
| --- | --- | --- |
| 640x360 | 0.320 → 0.150 ms | 9.448 → 9.215 ms |
| 1280x720 | 0.296 → 0.131 ms | 9.372 → 9.203 ms |

CPU selection is now separately measured: 200 iterations of main plus **two**
remote view updates (including uniform writes and model-origin copy) have
medians 0.168 / 0.163 ms at the two target sizes. These updates are still outside
the encode/completed-wall tables. The full-reference switch only bypasses
submission rejection; it still computes selection, so its update measurement is
not an old-renderer CPU baseline. Small completed-wall differences and unmeasured
present/driver work prevent claiming a gameplay frame-time improvement.

Focused tests: lib 21, profile example 10, portals 15; animation, dynamic,
parented_support, chamber06_render and viewmodel pass. Full no-fail-fast suite
again has only the three documented failing targets (chamber00_exit, gun_pickup,
rigid). Clippy remains baseline-warning-blocked; no new production lint warning.
Local logs: `evidence/portal-translucent-{verified,reference,focused,full-tests,clippy,native}.log`.

The severe visible-portal Windows Intel/Vulkan stall remains **unverified**.
No PVS, portal-tight aperture frustum, occlusion-query system or dynamic-bound
culling is claimed. Remaining practical work: replay the user's failing camera
on Intel with these reduced submissions, inspect GPU timing/driver waits there,
and only then choose the next evidence-backed bottleneck.

## Reopened production investigation: CPU amplification and correlated spikes

### New executable evidence, not another renderer smoke

`examples/gameplay_cpu_profile.rs` loads both actual maps through
`GameRoot::load_map`, binds original scene/colliders/animation, and runs the same
120 Hz runtime → shot advance → portal synchronization → player controller
sequence as native gameplay, plus scene and original viewmodel animation. Each
sample has two fixed ticks; six phases have 240 samples each. It excludes GPU,
upload, surface/presentation and audio; these timings are **not gameplay FPS**.
The pair and approach poses are explicitly staged on actual legal wall surfaces,
not a claim to replay the user's route or owned-channel puzzle. Chamber06's
forward/jump inputs perform one real controller crossing; the probe fails if that
crossing does not occur. `escape_02`'s approach remains blocked (zero crossings)
and therefore does **not** validate escape traversal. The authored-spawn phase in
chamber06 also falls below its initial elevator; it is not a playable route.

### Fix 1: do not enumerate scenery for impossible free-body openings

`RigidPhysics::step` previously scanned every brush for every free body, even
with no linked pair or when that body could not fit/reach either aperture.
On escape02 the isolated physics stage rose from 1.654 ms unlinked to 6.472 ms
linked, per two fixed ticks; query-hull building was zero in those warm frames.
A brush-independent **necessary** swept-plane/reach/fit condition now rejects
impossible candidates once per body. Eligible bodies still run the original
backing-brush and destination-clearance predicates; contacts, CCD policy and
body transfer are unchanged. This is not disabling distant bodies or Rapier.

The regression was observed red with 256 unnecessary brush tests, then green
with zero; CCD remains enabled for the rejected body. A nearby valid opening
still checks backing geometry and retains the existing portal CCD policy.

M4/Metal host, same CPU content and staged pair, before → fix 1 medians:

| Case (two fixed ticks) | Simulation ms | Physics subset ms |
| --- | ---: | ---: |
| escape02 unlinked | 2.053 → 0.551 | 1.654 → 0.121 |
| escape02 linked, outside aperture fit | 7.841 → 1.521 | 6.472 → 0.124 |
| escape02 linked, approach | 7.430 → 1.127 | 6.468 → 0.127 |
| chamber06 linked, outside aperture fit | 1.188 → 0.646 | 0.610 → 0.066 |

### Fix 2: reuse exit clearance within one controller sweep

After fix 1, chamber06 still exhibited an actual intermittent controller spike:
**8.689 ms** simulation at its crossing (8.586 ms controller), while physics was
approximately 0.07 ms. Rim queries consumed only 0.359 ms and query-hull building
was not the cause. Ordinary approach frames performed **1,046** destination
clearance checks per two ticks; the crossing frame performed 512. Each clearance
check enumerated world/rigid geometry. Multiple backing layers were asking the
same destination question for the same sweep.

`trace_hull` now memoizes each direction's clearance result in a two-entry local
array. The world, endpoints, hull and ignored body cannot change during this
query. The array is discarded when the sweep returns: moving obstacles, portal
changes and held-body changes are observed by the next query. Each backing brush
still has to match the original support predicate. Standalone aperture/contact
queries keep independent results; this is not a persistent aperture-collider cache.

The regression was red at 128 exit scans for 128 backing layers and green at one.
It also inserts a new destination blocker between queries and requires the next
sweep to stop, guarding against stale clearance. Actual sampled player positions,
linkage and crossing counts are **identical across all 2,880 frames** before and
after this second fix.

| Case, after fix 1 → both fixes | Simulation p99 ms | Simulation max ms |
| --- | ---: | ---: |
| chamber06 real controller crossing phase | 7.208 → 1.728 | 8.689 → 2.204 |
| escape02 blocked approach phase (not traversal) | 10.236 → 3.310 | 10.263 → 3.319 |

The chamber06 crossing still occurs once. Approach clearance checks drop from
1,046 to 14 per two ticks; crossing checks drop from 512 to 87 (other independent
queries remain). These are local CPU results, not proof that the user's stalls
or 8 FPS are fixed. No resolution, rendering quality or geometry change is made.

### Native frame evidence and why averages were misleading

`SOURCE_FRAME_DETAIL=1` alongside `SOURCE_FRAME_PROFILE=FILE.csv` now writes
`FILE.csv.frames.csv`, buffered and flushed with the summaries. Each row retains
same-frame timings/counters, elapsed time, begin-to-begin gap, pose, yaw/pitch,
portal centers, linkage/crossings and framebuffer dimensions. The existing
summary CSV remains available. `frame_work_ms` excludes event-loop idle;
`begin_gap_ms` includes pacing/idle between begin calls. Extra scopes cover the
controller, physics, held-object work, portal sync, viewmodel, query-hull building
and portal rims. Counts include query cache misses/whole-cache clears and aperture
exit checks. Nested timings overlap parent stages and **must not be added**.

Before the CPU fixes, ordinary native stationary launches (no scripted smoke,
no linked pair, forcibly terminated after 15 seconds) recorded 784 escape02 and
845 chamber06 complete detailed rows. Escape02's median simulation was 5.759 ms,
median acquire 9.491 ms; its 68.978 ms acquire spike did **not** coincide with
simulation work. The reported ~2.9 collider rebuilds/frame was a cold-start
averaging trap: exactly **4,042** escape02 hulls were built in frame zero and
**zero** thereafter; chamber06 likewise built its 1,873 hulls only in frame zero.
Neither run had a query-cache clear. This rules out sustained collider rebuilding
in those stationary local cases, not during the user's moving/held-body route.
Final native launches exercised the delivered row schema and production
simulation/render/upload/acquire/submit/present path on both maps (509 and 576
complete rows). They were also bounded stationary checks, not the missing Intel
interaction reproduction. Acquire can include vsync or previous GPU backpressure;
these CPU scopes do not report actual GPU execution time.

### Minimal Windows capture, with executable provenance

From PowerShell, after pulling these commits:

```powershell
Set-Location -LiteralPath 'C:\.dev\portal-rust'
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\capture-portal-performance.ps1
```

Defaults are exactly repository `C:\.dev\portal-rust` and game
`C:\.programs\Steam\steamapps\common\Portal\portal`. No path replacement or
Python is needed. The script builds release, obtains the actual application path
from Cargo's JSON artifact records (never a guessed executable name), records
checkout/worktree, executable SHA-256 and adapter-driver information, then launches
the existing inspected `--game ... --map ... --play` CLI for both maps. Startup
logs carry the embedded build SHA and actual wgpu adapter/backend. Close the first
game normally to move to chamber06, then close that normally. It produces one ZIP
with logs, both detailed/summary CSVs and worst-frame extracts. Environment profile
variables are restored afterward. A build failure does not launch an old binary.

In each map: briefly idle without a linked pair; reproduce the slow view; look
away/back; approach, cross, stop and return repeatedly. If safe, shrink the window
and repeat the same slow view: recorded dimensions permit resolution-dependent
GPU/driver load to be separated from controller/physics spikes. Avoid enabling
controller trace at the same time: its extra diagnostic sweeps alter CPU work.

The script passed the real PowerShell 7.6.7 parser on macOS, and its exact Cargo
artifact-discovery/hash pipeline was executed against a real release build.
The full Windows script, Windows PowerShell 5.1 and Intel/Vulkan execution remain
**unverified**; do not describe the script's local parser check as Windows testing.

### Verification and remaining acceptance

- New frame-row regression was observed red then green; final library tests:
  **25 passed**. Both CPU fixes have red/green work-count regressions.
- Portal/rigid/cube/Rapier/chamber06 focused integration tests pass; the known
  blocked-maker test was explicitly excluded from that focused run.
- Full `cargo test --quiet --no-fail-fast` after the fixes still has exactly the
  three AGENTS.md failures: `chamber00_exit`, `gun_pickup`, and `rigid`'s blocked
  maker. The suite is **not green**; no new failing target is hidden.
- Real-asset CPU probe completes with chamber06 crossing count 1; both final
  native frame CSVs parse with consistent columns and contain the new scopes.
- Raw uncommitted local evidence: `evidence/gameplay-cpu-{nested,fixed}*`,
  `evidence/gameplay-{exit,exit-fixed}*`, `evidence/*-reopened*`,
  `evidence/*-final.csv*`, and `evidence/performance-reopened-*-tests.log`.
- **Sustained Intel escape02 8 FPS: unresolved remotely.** A measured CPU cause
  is reduced; no Intel/GPU/presentation speedup is established.
- **Intermittent chamber06 traversal: local controller amplification fixed** in
  the staged actual-map crossing; the user's exact portal/held-object sequence
  and full native crossing presentation still require the Windows capture.
