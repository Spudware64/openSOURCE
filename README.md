# openSOURCE — Rust / wgpu Source 1 reimplementation

## Windows: update and debug build

Install Git for Windows and Rust via rustup (including the MSVC build prerequisites).
From PowerShell in this checkout:

```powershell
.\update-debug.ps1                       # git pull --ff-only, cargo build; does not launch
.\update-debug.ps1 -Run                  # build then launch chamber 00
.\update-debug.ps1 -Run -Trace            # launch with bounded diagnostics
.\update-debug.ps1 -Run -Trace -TraceSeconds 300 -Map maps/testchmb_a_01.bsp
# Optional: -GamePath 'D:\SteamLibrary\steamapps\common\Portal\portal'
```

Default game directory: `C:\.programs\Steam\steamapps\common\Portal\portal`.
The script runs from its own directory, never resets local changes, and stops on pull/build failure.
If execution policy blocks it, review the script and use a policy permitted by your machine administrator; the script does not change policy or credentials.

Each `-Run -Trace` creates a unique ignored `logs/<timestamp>-<id>/` containing
`console.log` (combined stdout/stderr, also shown in the terminal) and `controller.csv`
(created when the interactive window initializes). Startup output identifies build, executable,
game path, map and selected GPU adapter. CSV defaults to 120 simulation seconds (14,400 ticks),
configurable from 1–3600 seconds; paused/load time is not simulation time. Direct launches can set
`SOURCE_CONTROLLER_TRACE` to a **new** filename and `SOURCE_CONTROLLER_TRACE_SECONDS`.
Tracing is opt-in; disabled tracing does not run extra support sweeps or format CSV rows.
CSV is buffered, flushed at most every 120 ticks and at the cap/normal close; forced process termination
can lose buffered tail rows. Existing trace files are never overwritten.

Rows include previous/current feet, post-runtime feet, controller/runtime displacement, velocity,
current hull/eye height, physics and interpolated render eye, input/ground/crouch state, old/new support
model IDs and their raw/resolved transforms. `raw_after`/`resolved_after` track the **old** support
across its runtime update; `new_support_raw`/`new_support_resolved` identify the new support after movement.
`probe_drop18_fraction` and `probe_drop18_normal` are a separate measured 18-unit downward hull sweep,
**not** the internal step-selection result or collision flags (those are not exposed by this trace).
No controller physics, original assets, or elevator behavior is changed/fixed by these diagnostics.


## Latest: full Chamber01 → Chamber02 route verified

**A continuous route now runs from Chamber01's authored spawn through gun acquisition, middle lift, both later portal gaps and final elevator into Chamber02, without noclip, staged positions or forced puzzle inputs.** The actual BSP has no cube/plate puzzle: its later section is two portal gaps, so none was invented. Existing generic gameplay semantics suffice; the new opt-in input driver and regression establish continuous completion of this BSP, not whole-engine/game parity.

**112 tests pass; clippy passes with warnings denied.** Actual Apple M4/Metal native verification: four portal transfers, 11,067 fixed ticks to authored ChangeLevel and 11,307 through grounded next-map arrival. Presented frames inspected. Evidence and explicit remaining limitations: [`evidence/native-chamber01-complete.md`](evidence/native-chamber01-complete.md).

```sh
SOURCE_PORTALS=1 SOURCE_CHAMBER01_SMOKE=1 SOURCE_CHAMBER01_FULL=1 SOURCE_CHAMBER01_EXIT=1 target/debug/openSOURCE --game ../portal-original/portal --map maps/testchmb_a_01.bsp --play
```

Omit `SOURCE_CHAMBER01_EXIT` to leave ordinary controls ready. Full Chamber02 gameplay, physical OS input, exact PHY/animation collision and material/sound parity remain unverified. The route uses normal jumps onto raised one-way elevator mesh support; no replacement floor was introduced. Historical sections below are superseded where they call whole-Chamber01 progression unresolved.


A native, interactive **Source 1 map/asset loader and bounded gameplay subset**, using Portal as the real-game test fixture. It is not Source engine parity, and does not load or execute proprietary game DLLs. Original game artifacts remain read-only.

## Latest Chamber01 first-puzzle increment

**The actual chamber01 gun-acquisition puzzle now runs from its authored spawn to its opened exit corridor without noclip, staged player positions, direct smoke-issued door inputs or a fake ending.** The map's rotating pedestal / reparenting relay cycle fires automatic blue portals using its current aim; the original weapon-script world model appears on the pedestal, E pickup grants the authored **blue-only** channel, disables the automatic relays, and fires the actual exit-opening outputs. Normal mouse firing is now ownership-gated rather than unlimited. Ownership survives map transitions.

The verified route waits for the actual entrance scene, descends the stairs, jumps through an automatic linked portal, reaches/picks up the gun, places an owned blue portal, traverses back, and walks through the opened exit. Genuine blockers repaired: packaged VSIF scene-duration completion scheduling, passable door/carrier flags, and a partially inserted jumping hull leaking through a portal backing brush without transfer. Parent transforms update posed child meshes and angular convex collision; this map's rotating brush itself has zero render faces.

**111 tests pass; clippy passes with warnings denied.** Apple M4/Metal native-window verification completed two portal transfers in 6,192 fixed 120 Hz steps, ending grounded at `(-256.0644,35.66831,0.031249575)`. Presented 2200×1446 remote-view / gun-before-and-after / placed-portal / opened-exit captures were inspected. Existing chamber00→01 elevator and cube/plate/exit/re-pickup native regressions also pass. Evidence, inventory, commands and explicit acceptance boundaries: [`evidence/native-chamber01.md`](evidence/native-chamber01.md), `artifacts/chamber01*`.

```sh
# Normal play, no scripted route:
SOURCE_PORTALS=1 target/debug/openSOURCE --game ../portal-original/portal --map maps/testchmb_a_01.bsp --play
# Reproduce the full first-puzzle route, capture real presented frames, then exit:
SOURCE_PORTALS=1 SOURCE_CHAMBER01_SMOKE=1 SOURCE_CHAMBER01_EXIT=1 target/debug/openSOURCE --game ../portal-original/portal --map maps/testchmb_a_01.bsp --play
```

Omit `SOURCE_CHAMBER01_EXIT` to leave ordinary controls ready after verification. **The later chamber01 lift/cube puzzle and eventual next-map exit remain unresolved; neither this entire BSP nor Portal is complete.** Scene support is duration/Start/Cancel/completion I/O only, not sound/actor choreography; pickup is bounded E-use, not auto-touch parity. General rotating nonzero-face BSP rendering, full rigid-body/PHY and animation collision, portal-straddle replacement recovery, materials/lighting, viewmodel/charge effects and physical OS input remain outside the verified subset. Older milestone counts and missing-feature lists below are historical where superseded here.

## Latest map-transition / fizzler increment

**Authored chamber 00 → 01 map progression now executes in the same native window.** The final elevator's real touch trigger, five-second delay, track traversal and `OnPass → ChangeLevel` load the next BSP through the retained selectable game root, preserve the player's landmark offset/view/velocity, transfer the matched `globalname` elevator carrier, and deliver `logic_auto.OnMapTransition` rather than `OnNewGame`. The rider lands on the arrival elevator and walks off into chamber 01's entry corridor. No chamber-completion script or direct smoke-issued ChangeLevel is substituted.

Map preparation validates destination assets/landmarks before replacement. Submitted GPU work is drained, then the previous scene's buffers/textures/bind groups and CPU scene are dropped. Old held/template bodies, linkage, pressed keys, cursor capture and fixed-step accumulator are reset; click again to resume controls. Missing maps/landmarks and unsupported inputs remain explicit diagnostics. Legacy windows without `--game` cannot transition and report that limitation.

**Opt-in Portal cleansers now remove free or held physics cubes, immediately remove their collision/visible instance, clear the held handle, fire authored `OnDissolve`/`OnDissolveBox`, and fizzle both active portals when the player touches the volume.** Bounds come from actual transformed BSP trigger models, not invented planes. This is instantaneous gameplay dissolution, **not** a particle/material dissolve effect. Trigger bounds remain AABBs and use fixed-step overlap, not exact convex-volume/swept-fizzler parity.

**99 tests pass; clippy passes with warnings denied.** Inventory covers all **26 supplied BSPs, 46 matching landmark links, and 40 cleanser brush volumes**. Actual Apple M4/Metal window smokes verify final elevator → next map → grounded arrival → walking off, held-cube removal/portal fizzle/output delivery, and the existing complete cube/plate/exit/re-pickup regression. Presented 2200×1446 screenshots and logs: [`evidence/native-progression.md`](evidence/native-progression.md), `artifacts/progression*`, `artifacts/cleanser-smoke.log`.

```sh
# Normal play (no scripted positioning):
SOURCE_PORTALS=1 target/debug/openSOURCE --game ../portal-original/portal --map maps/testchmb_a_00.bsp --play
# Stage on final elevator, then exercise only real map I/O and movement:
SOURCE_PORTALS=1 SOURCE_PROGRESSION_SMOKE=1 SOURCE_PROGRESSION_EXIT=1 target/debug/openSOURCE --game ../portal-original/portal --map maps/testchmb_a_00.bsp --play
# Stage a held cube before the real first cleanser and walk across:
SOURCE_PORTALS=1 SOURCE_CLEANSER_SMOKE=1 SOURCE_CLEANSER_EXIT=1 target/debug/openSOURCE --game ../portal-original/portal --map maps/testchmb_a_00.bsp --play
```

Omit the corresponding `*_EXIT` flag to leave normal controls ready after verification. These are scripted simulation inputs, not physical OS input verification, and staging on the final lift is **not** proof of a start-to-finish chamber solution. Later chamber logic still reports missing rotating-door, automatic portal-gun fire, sound/choreography and material inputs; **full puzzles/full Source parity are not complete**. Global-state handoff is the tested elevator-transform subset, not general Source save/restore. A degenerate zero-area grate face in chamber 01 is explicitly diagnosed and excluded from portal placement, not used to abort an otherwise renderable map. Historical sections below are superseded where they say ChangeLevel/fizzlers are wholly absent or show older counts.

## Latest cube / dropper gameplay increment

**Chamber 00 now supports a bounded cube gameplay loop against the original assets:** the authored dropper relay opens its animated cover and spawns template model instances; **E** picks up/drops, **F** throws (mouse buttons remain portal placement). Swept axis-aligned cuboids fall, slide, stack, collide with brushes/other bodies and block the player. Real posed mesh bounds supply extents. A dropped cube activates the authored filtered pressure trigger, opening the exit; walking through and back, then picking it up releases pressure and recloses the exit.

**89 tests pass; clippy passes with warnings denied.** The formerly failing real-map re-pickup regression remains intact. Actual Metal window smoke completed spawn → pickup → carry → drop → plate/door open → walk through and back → re-pickup → door closed; 2200×1446 presented frames were inspected. Evidence: [`evidence/native-cubes.md`](evidence/native-cubes.md), `artifacts/cube-{tests,clippy,smoke}.log`, `artifacts/cubes/`.

```sh
SOURCE_PORTALS=1 SOURCE_CUBE_SMOKE=1 SOURCE_CUBE_EXIT=1 target/debug/openSOURCE --game ../portal-original/portal --map maps/testchmb_a_00.bsp --play
# Omit SOURCE_CUBE_EXIT to leave the window playable after scripted verification.
```

Free bodies cross linked portals with mapped velocity, including the actual chamber floor→wall route and blocked exits. **Approximation:** fixed upright AABBs, no angular dynamics/PHY constraints, no rotating/split body render or held-object portal transfer. Template spawning uses bounded preallocated mesh slots; this raises resident geometry to 301662 triangles, not an unlimited dynamic asset loader. Trigger filters/activator support is a narrow subset. Sound, choreography, material proxies, ChangeLevel/full puzzle and game-DLL execution remain unsupported. Physical OS keyboard/mouse delivery remains unverified; window smoke supplies scripted simulation inputs. Original artifacts remain read-only. Historical sections below are superseded by this section where they list cubes as missing or older counts.

## Latest linked-portal increment

**One-hop linked portal views and player traversal are enabled automatically for maps authoring `prop_portal` or `weapon_portalgun`.** No shell environment setup is needed for the normal Portal `--game ... --map ... --play` command, including on Windows. `SOURCE_PORTALS=0` explicitly selects the generic explorer; `SOURCE_PORTALS=1` explicitly enables rules for diagnostic maps. Click to capture, then **LMB blue / RMB orange**, aiming at screen centre. Existing WASD/Space/Ctrl/E/Esc/R/N controls remain. No substitute portal-gun model is drawn. Chamber 00's first real portal pair follows its authored 66.5-second startup delay; activation and linkage are not forced early.

```sh
SOURCE_PORTALS=1 target/debug/openSOURCE --game ../portal-original/portal --map maps/testchmb_a_00.bsp --play
SOURCE_PORTALS=1 SOURCE_PORTAL_SMOKE=1 SOURCE_PORTAL_EXIT=1 target/debug/openSOURCE --game ../portal-original/portal --map maps/testchmb_a_00.bsp --play
SOURCE_PORTALS=1 SOURCE_PORTAL_SMOKE=floor SOURCE_PORTAL_EXIT=1 target/debug/openSOURCE --game ../portal-original/portal --map maps/testchmb_a_00.bsp --play
```

- Nearest world-surface placement is restricted to the tested modular-concrete family, noportal/sky/nodraw/tool/displacement exclusions, coplanar footprint fitting and brush-ray occlusion; overlapping same-plane pairs are rejected. This is a conservative sampled placement policy, not Source's complete portal-bumping/material solver.
- Actual map `prop_portal` positions/angles, blue/orange channel, `Activated`, delayed `SetActivatedState` and named `Fizzle` feed a separate opt-in Portal game module.
- Two bounded full-resolution linked views share real scene resources. Mapped camera position/forward/up, exit-world-plane fragment clipping, depth-tested elliptical compositing and coloured rims are real GPU rendering. Recursion is capped at **one hop**, without nested apertures; no oblique-projection parity claim.
- Swept upright-hull apertures support wall↔wall and wall↔floor transfer, preserve vector velocity and camera basis, retain remaining substep motion, block rim/blocked-exit cases, and do not erase fling momentum when forward remains held. Floor backing layers are handled; ordinary nearer collisions resolve first. Camera roll recovery is approximate.

**74 tests pass; clippy passes with warnings denied.** Presented Apple M4/Metal 2200×1446 screenshots were inspected: blue remote-cell view, authored wall crossing and orange reverse view; placed floor view, floor→wall crossing and upright recovery. Offscreen GPU tests verify aperture-local changes, parallax and exit clipping; real-map simulation tests cover both floor→wall and wall→floor velocity mapping. Evidence and exact bounds: [`evidence/native-portals.md`](evidence/native-portals.md); logs `artifacts/portal-{tests,clippy,smoke,floor-smoke}.log`; images `artifacts/portals/`.

**Still missing:** cubes/spawning/pickup/rigid-body physics, animated-door/PHY collision, ChangeLevel/full puzzles, noportal volumes/detectors/fizzlers/linkage-group parity, recursive views and split bodies, exact seamless floor eye correction, sound/choreography and material/lighting parity. No cubes were attempted in this increment. Physical OS input remains unverified. No universal Source engine or complete Portal claim. Earlier sections below are historical and superseded here where they describe portals as wholly absent or older test counts.

## Latest native animation/elevator increment

Real MDL44/46/48 local single-blend skeletal sequences now play through map `SetAnimation`, `SetDefaultAnimation` and one-shot `OnAnimationDone` outputs. Bone-world × inverse-bind skinning updates existing mesh positions/normals before authored entity/parent transforms; frozen poses avoid repeat CPU skinning/GPU uploads. Dropper closed/open/closed and elevator closed/open frames were captured from the actual native presented surface and inspected; a real-mesh elevator close/default-idle round trip is also tested. External ANI blocks, blends/layers, procedural bones/IK/flex remain explicitly unsupported.

The chamber-00 middle elevator now follows its real `path_track` chain, map-triggered five-second delay, `StartForward`/`Stop` and `OnPass` relay outputs. The player rides **576 Source units**, remains grounded, and walks into the upper corridor without noclip. This is a tested progression subset, not chamber/puzzle completion. The invisible train carrier's `SF_TRACKTRAIN_PASSABLE` flag is respected. Support comes from actual rigid posed MDL upward triangles as **one-way floor support**, not invented geometry or decoded PHY; prop walls, animated door collision and general rigid-body physics remain absent. Carried-player brush overlaps reject the movement transaction rather than crush the rider.

**59 tests pass; `cargo clippy --all-targets -- -D warnings` passes.** Existing animation integration tests were preserved; the scene warning regression now expects explicit initial-pose fallback diagnostics instead of the obsolete blanket "all animation unsupported" warning. Chamber00 remains 53 static props/0 skipped, 34 entity models/2 skipped, 144302 triangles, 204 materials. Refract/sign-model warnings and several unsupported gameplay inputs remain visible.

Evidence: `artifacts/native-parity-tests.log`, `native-parity-clippy.log`, `native-parity-{dropper,elevator}.log`, and actual 2200×1444 window captures under `artifacts/native-parity/`. The lift smoke enters the real trigger, rides, waits for the authored opening relay, then walks off. Physical OS keyboard/mouse delivery is still not verified. Elevator lighting remains dark; no arbitrary brightness fix was added. Prior sections below are historical and are superseded here where they describe static-only dynamic props, absent animation/elevator support, or older test totals.

Reproduce scripted native evidence (omit `SOURCE_PARITY_EXIT=1` to leave a normal controllable window after the smoke):

```sh
cargo build
SOURCE_PARITY_SMOKE=dropper SOURCE_PARITY_EXIT=1 target/debug/openSOURCE --game ../portal-original/portal --map maps/testchmb_a_00.bsp --play
SOURCE_PARITY_SMOKE=elevator SOURCE_PARITY_EXIT=1 target/debug/openSOURCE --game ../portal-original/portal --map maps/testchmb_a_00.bsp --play
```

No portal gun/placement/traversal, cube pickup/spawning/physics, full chamber progression, map ChangeLevel execution, sound, choreography, HDR/material parity or game-DLL compatibility is claimed. Browser export was not changed by this increment.

## Run

Select the **game directory containing `gameinfo.txt`**, not a hardcoded game title:

```sh
cargo run -- --game ../portal-original/portal --map maps/testchmb_a_00.bsp --workers 4 --play
cargo run -- --game ../portal-original/portal --map maps/testchmb_a_00.bsp \
  --output evidence/textured-room.png --camera -752 -800 210 -1100 -864 210
cargo run -- --game ../portal-original/portal --map maps/background2.bsp \
  --workers 2 --output evidence/textured-background.png
cargo test
cargo clippy --all-targets -- -D warnings
```

`--map` is a game-relative asset path resolved through SearchPaths, or an absolute BSP file path. Choose `--play` **or** `--output FILE.png`; `--camera` takes six finite Source coordinates (eye, target). Workers default to four and are bounded to 1–16. The legacy `MAP.bsp OUTPUT.png [six camera coordinates]` CLI remains a **flat diagnostic renderer**; `--play MAP.bsp` remains a legacy untextured mode. Use `--game` for textured rendering.

Click to capture the mouse. **WASD** walk, mouse look, **Space** jump, **left Ctrl** crouch, **E** use nearby brush buttons/doors, **Escape** release cursor, **R** respawn, **N** noclip; Space/left Shift move vertically in noclip. Respawn does not reset entity state. Losing focus clears held keys. Noclip remains useful because portal mechanics, prop collision and complete puzzle scripting are absent.

## Implemented increment

- Ordered `gameinfo.txt` Game/Mod/Platform SearchPaths; `|gameinfo_path|`, `|all_source_engine_paths|`, sorted `custom/*` directories/VPKs, packaged `.vpk` to `_dir.vpk` resolution, loose files, VPK v1/v2 trees/preload/inline and numbered archive reads, and BSP embedded ZIP overrides. No archive extraction. Traversal, escaping symlinks, truncated structures and oversized assets are rejected.
- KeyValues parsing with comments, nested blocks and duplicate entity output keys. VMT base texture and recursive Patch include/insert/replace resolution; VTF top-mip decode for DXT1/DXT3/DXT5, one-bit-alpha DXT1, and selected uncompressed RGB/RGBA layouts.
- Real diffuse VTF samples using BSP texture vectors, repeating UVs, per-face **LDR RGBExp32 lightmaps**, bilinear sampling with duplicated atlas borders, UnlitGeneric and alpha cutouts. Sky/nodraw/tool/trigger faces are suppressed. Materials use 256×256 resampled array layers (limit 1,024); lightmaps use a 4096×4096 atlas, with explicit capacity failure.
- Brush-model ownership through models → nodes → leaves → leafbrushes. Local brush faces/planes now receive entity origins and initial rotations, rather than placing every submodel at world origin. Sliding model translations update both GPU rendering and solid collision.
- Fixed 120 Hz swept-hull movement, gravity/jump/wall sliding, 18-unit step-up with clearance and landing checks, 36-unit crouched hull/28-unit eye, reduced crouch speed, and obstruction-checked standing. Source Z-up coordinates and units are preserved; movement tuning is an approximation, not recovered Source movement code.
- A bounded entity I/O scheduler honours output delays, duplicate outputs and finite fire counts. Basic `logic_auto`, `logic_relay`, brush doors/buttons/linear motion, Open/Close/Toggle/Press/SetPosition, Enable/Disable/Kill, trigger_once/trigger_multiple player touch events, and ordinary trigger_teleport destinations are handled. E uses a 96-unit occlusion-checked ray. Moving doors stop instead of crushing a player. Unsupported delivered inputs are retained in diagnostics rather than pretending they ran.

## Displacement/static-prop increment

- BSP v20 DISPINFO/verts/tris v0 with 176/20/2-byte strides. Powers 2–4, rotated start corner, Source row/column orientation, bilinear quad base + displacement direction/distance + elevation, alternating full-resolution triangle diagonals and smooth outward normals. Texture/lightmap coordinates project the **undisplaced base**, not the elevated terrain. Real chamber 15 has four power-4 surfaces / 2,048 terrain triangles; escape 02 has five power-3 surfaces / 640 triangles.
- BSP-absolute `sprp` game-lump offsets, v5 (60 bytes) and v6 (64), validated dictionaries/leaf ranges/model indices/finite transforms. A shared bounded asset pool reads distinct model triplets once; instance geometry receives origin and Source pitch/yaw/roll. Model texture directories and skin-family remapping resolve real VMT/VTFs.
- **Static-flagged MDL v44/v46**, IDSV VVD v4 and packed `.dx90.vtx` v7: checksum correlation, LOD0/default body-model selection, VVD fixup reconstruction, MDL mesh/model vertex offsets, VTX original-vertex remapping, triangle-list/strip decoding and normals/UVs. Unsupported or missing prop assets are skipped with explicit model/instance diagnostics—not replaced with placeholder geometry.
- Opaque VMT diffuse alpha is no longer mistaken for coverage: Source models often use it as a specular/environment mask. Coverage remains an alpha-cutout approximation only for explicit `$alphatest`/`$translucent`; blended sorting is not implemented. Prop lighting is **approximate ambient + fixed directional diffuse**, not Source static-prop baked lighting or leaf ambient probes.
- Actual GPU uploads check texture/layer/buffer limits and return errors before wgpu validation panics. The larger atlas and layer ceiling were needed by the supplied terrain maps; no unlimited resource claim.

**Collision remains convex BSP brushes only.** Terrain tessellation and static props currently render but do not add triangle/PHY collision; walking on these maps can disagree with visible terrain and pass through props. No neighbor stitching/LOD, displacement blend alpha, subdivision reconstruction, model animation/skinning/flex, dynamic props, bodygroup selection, PHY/physics, or MDL versions other than the tested 44/46. Different-version game lumps/MDLs are rejected/skipped explicitly. Invalid unused normals and out-of-range skin families in some supplied models are conservatively skipped, not silently repaired.

| Actual fixture | Rendered / skipped static props | Prop triangles | Total rendered triangles |
|---|---:|---:|---:|
| `testchmb_a_00.bsp` (sprp v5) | 53 / 0 | 57,522 | 65,576 |
| `testchmb_a_15.bsp` (sprp v5) | 436 / 14 | 421,868 | 480,507 |
| `escape_02.bsp` (sprp v6) | 900 / 4 | 897,371 | 930,553 |

Reproduce the terrain and isolated real-model captures:

```sh
cargo run -- --game ../portal-original/portal --map maps/testchmb_a_15.bsp \
  --output evidence/displacement-chamber15.png --camera -2010 400 -230 -1650 450 -340
cargo run --example prop_preview
cargo run --example prop_preview -- models/props/lab_monitor_pose01/lab_monitor_pose01.mdl evidence/prop-monitor-preview.png
cargo run --example prop_inventory
```

`evidence/static-props-chamber.png`, `displacement-chamber15.png`, `static-props-escape02.png`, `prop-model-preview.png`, `prop-monitor-preview.png` and actual-window `static-props-interactive-pressure.png` were inspected. The new pressure-trigger window smoke preserved grounded movement, moving doors and native cursor release. **47 tests passed** including real GPU normal/opaque-alpha tests, malformed indices/bounds/fixups, supplied map/model fixtures and deterministic worker-count output; clippy passed with warnings denied. Logs use `evidence/terrain-props-*` and per-capture names. Physical keyboard/mouse verification is still not claimed.

## Architecture and modern infrastructure

| Module | Responsibility |
|---|---|
| `assets.rs`, `kv.rs` | Read-only, ordered generic game filesystem and KeyValues |
| `texture.rs`, `scene.rs` | CPU material decode, BSP UVs, lightmap preparation, visibility filtering |
| `displacement.rs`, `props.rs`, `studio.rs` | Validated terrain and static model assets |
| `models.rs`, `play.rs` | Versioned model ownership, transformed convex collision, traversal |
| `entity.rs` | Reusable entity parsing, delayed I/O and bounded gameplay subset |
| `scene_gpu.rs`, `textured.wgsl` | Shared window/offscreen textured wgpu pipeline and model translation uniforms |
| `window.rs`, `gpu.rs`, `main.rs` | Main-thread window/events/GPU submission, readback and CLI |

A **bounded Rayon worker pool** loads/decodes/resamples independent materials. Indexed collection keeps results/diagnostics deterministic; a real-map test compares single-worker and four-worker texture bytes, lightmap bytes and vertex bytes. Model-triplet decoding also uses the same pool; instance expansion, material registration and geometry/lightmap packing stay serial; no per-file thread spawning. winit/events, GPU resource creation and submission remain on the main thread. Normal gameplay/entity/collision updates use a fixed-step accumulator independent of rendering. Both rendering routes request wgpu's available backends; **only Apple M4 / Metal has actually been exercised**.

## Compatibility matrix

| Input / feature | Status and evidence |
|---|---|
| Portal Source 1 BSP v20 | Real `background2.bsp` and `testchmb_a_00.bsp` loaded and GPU-rendered; chamber collision and entity tests |
| BSP v19 | Header admitted only with supported selected layouts; no real v19 game fixture verified |
| BSP v21+, divergent/unknown layouts, compressed selected lumps, Source 2 | Rejected / unsupported; never claimed compatible |
| Geometry/material layouts | Vertices 12, faces 56/version 1, edges 4, surfedges 4, models 48, texdata 32, texinfo 72; unknown versions rejected |
| Collision layouts | Planes 20, brushes 12, sides 8, nodes 32, leaves v1/32 or v0/56, leafbrush refs 2; real fixture uses leaves v1 |
| VPK | v2 real Portal/HL2 archives; v1 inline/preload verified with a renamed synthetic game fixture; CRC/signature verification not implemented |
| VTF | 7.0–7.5 supported header/resource subset; real 7.1 DXT1 fixture explicitly tested. Volumes/cubemaps, unsupported pixel formats and truncation rejected |
| SearchPaths | Ordered local installation roots, gameinfo and shared-engine macros, custom wildcard mounts; external sibling installations/Steam dependency discovery and language variants are not implemented |
| Other Source games/branches | Generic root/map interfaces exist; **no other real game fixture was available**, so no “any Source game works” claim |
| Proprietary gameplay DLLs | Never loaded/executed; arbitrary DLL gameplay needs a compatibility layer or a separate reimplementation |

Triggers currently use **transformed model bounding boxes**, not exact non-axis-aligned volume intersections. Named I/O is exact-name lookup; filters, wildcard targets, activator context, locking/spawnflag parity and complete entity semantics are not implemented. Brush doors are useful, not a reproduction of every Source door rule. Static initial rotation is supported; rotating doors and riding/pushing platforms are not.

**Deferred:** portal placement/teleportation/view rendering and puzzle parity; animated/dynamic props and physics objects; terrain/prop collision and displacement blending/LOD; PVS/occlusion traversal; decals/overlays/skyboxes; sound/animation/choreography/NPCs; complete logic timers/counters/filters; HDR/bump/detail/specular/refraction/translucent sorting, material proxies, self-illumination masks and full VMT fallbacks. The chamber reports **11 Refract-material warnings** and those unsupported materials receive a diagnostic checkerboard. HDR-only maps are not covered by the LDR renderer.

## Actual-window verification

The following reproducible fixture smoke places the real player on the first chamber's pressure trigger, runs actual fixed-step collision and entity updates, opens the sliding exit doors and captures the **presented window surface**, then releases the cursor and leaves the window playable:

```sh
SOURCE_SMOKE=1 SOURCE_SMOKE_POSITION='-511.87 -928.49 144.1' \
SOURCE_SMOKE_ANGLES='0 171 0' \
SOURCE_SCREENSHOT=evidence/textured-interactive-pressure.png \
target/debug/openSOURCE --game ../portal-original/portal \
  --map maps/testchmb_a_00.bsp --workers 4 --play
```

Without a smoke position, `SOURCE_SMOKE=1` runs the legacy settle/walk/jump sequence. `SOURCE_SCREENSHOT` alone captures the initial settled view. Old PORTAL_SMOKE/PORTAL_SCREENSHOT names remain aliases. Smoke inputs are scripted, **not OS-injected input**; raw physical keyboard/mouse still require a human check because desktop capture/input automation permissions were unavailable.

Verified window: Apple M4 / Metal, Retina 2200×1444 surface; native cursor grab/release succeeded. The pressure plate moved model 9 to `(-512.37,-928.49,132)`, exit-door models 16/17 moved to `(-912,-792,180)` and `(-912,-936,180)`, and the player settled grounded at `(-511.87,-928.49,136.03125)`. Separate tests exercise release/closing and obstruction-stopped closing. Offscreen and actual-surface images were visually inspected.

Chamber BSP-only baseline: **8,054 visible triangles**; current BSP + static props: **65,576 triangles**, **182 material array layers** (including hidden tool and diagnostic layers), **153 filtered faces**, **2,360 lightmapped faces**. Test/clippy logs and screenshots live in `evidence/`; debug symbols/incremental builds are disabled to conserve disk.

## REA/Ghidra evidence

`evidence/rea-material-layouts.json` retains seven genuine static REA records from read-only `bsppack.dll`, SHA-256 `e49cf01968df92812180e91cc6373eae50eccbd6d23c8a12470124e2a715f27c`, Ghidra 12.1.4; session closed after export.

- Defined TEXDATA/TEXINFO/LIGHTING/PAKFILE/string-table/HDR strings: `ev_5567225d4e123a32895ce1e834d1fab9296861469f0b23b48c3bd2a16a3bb49b`.
- Decompiled caller `FUN_1000db60` calls `FUN_10008b70(6)`: `ev_d72b676b757eb528205612cda79b547e19da4f50f734bb042a5a63963a0b8f6c`.
- That routine multiplies copied records by `0x48`: `ev_3d84e4963a9a75fdd97326f32c16ca86642d27bb4497f080843366df712648b6`.
- Copier `FUN_10003670` validates length modulo `0x48`, checks lump version and advances records by `0x48`: `ev_5be811b3f788e083c0ced16972db0531558a2162ddfe4f49d35b8da73e16b384`.

This supports the **72-byte TEXINFO stride through observed call correlation**. Individual UV/lightmap field meanings, filesystem/entity behaviour and movement tuning are compatible implementation choices checked against fixtures, **not recovered engine/client/server source or observed proprietary runtime behaviour**. Earlier header/geometry evidence remains in `evidence/rea-bsppack.json` and measured directories in `evidence/map-layouts.json`.

## Source and assets

Source repository: https://github.com/Spudware64/openSOURCE

This repository contains the reimplementation source only. Valve game assets are not distributed and are not covered by this project’s MIT licence. Supply your own Portal installation with `--game`. Runtime evidence and captures under `evidence/` are local and are not published.
