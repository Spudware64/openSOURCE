# Original controller evidence and bounded repairs

## Scope and identity

Original inputs were inspected statically, never executed or edited. REA used
Ghidra 12.1.4, ephemeral source-immutable import, x86 PE images:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| portal/bin/server.dll | 7959552 | 6e0df352d464b1446ba32418578267ec623a6df2d326cf3265e2500f6f3a6162 |
| portal/bin/client.dll | 5500416 | 59abd9eea5c6a6a5a48bd5be3903b66bc97e7d2e1c893ac521d055fcea2da3d5 |

Full local evidence exports are `evidence/controller-rea-server.json` and
`evidence/controller-rea-client.json`, deliberately not committed (decompilation
and generated artifacts remain local). Addresses below are image virtual addresses,
not file offsets. Function names are inferred, not recovered symbols.

## Findings

- **Observed ground categorization:** server `0x1018cef0` probes down 2 units,
  accepts a floor-normal Z of at least 0.7, and rejects upward relative speed
  above 140 units/sec. For an existing ground entity, its vertical velocity is
  subtracted before the threshold comparison. REA evidence
  `ev_4533759265b86c5b6740e7df90d184169f72d141cd43bfb6fefccd87bd1053d1`.
  The movement-vtable entry at `0x1052e8c0` points here; the entry for the ground
  setter at `0x1052e8ec` points to `0x10192850`.
- **Observed ground transition/base velocity:** server `0x10192850` subtracts
  a new support's XY velocity when acquiring ground and adds the old support's
  XY velocity when losing ground; it also handles the support Z component and
  zeros movement vertical velocity when grounded. Evidence
  `ev_7dff9789f3856752942a398b05a02983e3793485b8f9b416bd4c36226c04a2b7`.
  Current Runtime platform displacement carry is a bounded approximation, not
  equivalent base-velocity/ground-entity semantics.
- **Observed stance/view transition:** server `0x1018ef50` contains the diagnostic
  "Restoring player view height", duck state checks, 0.4/0.2-second transition
  branches and cubic smoothstep. `0x101927a0` computes a standing/duck view offset
  blend. Evidence respectively
  `ev_2df5f4ecf3e3cba2a753ec7099b51920c305b998d34e6c31a4afd57b9bd83d40`
  and `ev_152c31a29f46c8b38e4c6a62da7f16e3784e916f21caf1ce716164a602fb3d07`.
  Client diagnostic `0x103beda8` has xref `0x10137849` inside `0x10137700`
  (`ev_756617dc60f1e65de5e4ed73a4e90d9f089057b8ee30b0934623a4af95d65c41`).
  This does NOT establish that stance is the cause of the user's oscillation;
  current standing trace remains eye height 64, crouched false.
- **Observed portal authoring names:** original server defines Activated,
  PortalTwo, SetActivatedState and LinkageGroupID strings. Evidence
  `ev_cfcdd45d3db1c2be943cc8cdad5d75050e1394b7f7e42e781907994aae54e46f`.
  SetActivatedState string `0x106117a8` has xref `0x10452ec2` inside the
  registration routine `0x10452a20`; this is registration evidence, not a
  decompiled activation/linkage implementation.
- **Observed launch-path defect:** normal CLI previously passed no portal surfaces
  unless SOURCE_PORTALS existed in the environment. That disables event handling,
  automatic portal shots, portal views and traversal together. Maps authoring
  prop_portal/weapon_portalgun now enable those rules without shell-specific
  setup; explicit 0/1 overrides remain. This is independently established source
  and native-execution evidence, not a binary-derived rule.

## Authored chamber 00 timeline

The actual BSP's `relay_start_map` schedules
`relay_portal_cancel_room1,Trigger,,66.5,-1`. Temporary blue/red_start portals
activate at startup and deactivate at 0.5 seconds. The later relay activates
`portal_blue_0` (-832,-320,180) and `portal_red_0` (-591,-412,212).
`tests/chamber00_start.rs` advances normal simulation from spawn through this
wait, then walks right briefly and forward into the actual first portal pair.
It crosses to (-830.73645,-318.49304,128.03125) without assigning a player pose
or adding partners. This verifies that path only, not all chamber 00 puzzles.

## Repairs and verification

1. Upward-launch regression first failed with velocity Z 265 instead of 395:
   the floor probe incorrectly allowed held jump to replace a 400-unit launch.
   The original 140-unit ground rejection now preserves the launch. Boundary
   assertions at 140/141 pass. This is a narrow real bug fix, **not a claimed
   fix for constant standing height oscillation**.
2. Default portal-rule selection now works without SOURCE_PORTALS. Actual native
   Metal chamber01 full route reaches four portal crossings, pickup, player shots,
   cleanser and ChangeLevel to chamber02 with a supported arrival. It exits
   normally. Actual Metal chamber00-to-01 elevator transition and destination
   walk-off also pass without SOURCE_PORTALS. Scripts generate normal control
   inputs but are not Windows physical-input tests.
3. `tests/height_trace.rs` records 2400 ticks each at spawn and on the real middle
   elevator. No stance flips; settled support-relative per-tick height change is
   below 0.1 units. This is a passing diagnostic baseline, not a red reproduction
   of the user's reported oscillation. Intro-script position changes occur and
   must not be confused with stance changes.

The suite excluding the explicitly reproduced known baseline failure passes.
`blocked_maker_does_not_embed_or_fire_spawn_output` still fails left 1/right 2;
its assertion was not weakened. Local logs: controller-tests.log,
controller-chamber01-native.log, controller-progression-native.log.
Inspected native surfaces include chamber01-full/phase-40.png (visible linked
remote room through blue aperture) and phase-32.png (supported arrival cabin).

## Prior-pass unresolved acceptance (superseded where noted below)

- Constant tall/small oscillation is not reproduced by the standing/elevator-center
  traces. Walking support-switch traces and a matching Windows route remain needed.
- Elevator cabin walls remain unsupported by floor-only rendered-triangle support.
  No blanket triangle-shell colliders or fabricated map-specific walls were added.
  The reported fall-through case is not established fixed by a center ride.
- Original StepMove and full portal transition/linkage implementation are not yet
  recovered. Render-only interpolation and instantaneous stance remain approximations.
- No Bevy migration: changing engines would not resolve these missing semantics.
- Windows physical keyboard/mouse and visual feel remain unverified.

## Standing/walking support follow-up

### Reproduced causes, not camera smoothing

1. A swept brush whose starting clearance is positive but smaller than the
   0.03125 contact skin discarded its negative epsilon-biased entry fraction.
   A standing/walking hull then entered the solid; subsequent sweeps began inside
   and could not recover an entry hit. `tests/contact_epsilon.rs` failed at the
   first tick (clearance 0.0001, feet Z=-0.16656668). Clamping a genuine entry to
   zero retains the contact normal without treating an initially embedded hull as
   a new entry. Standing/walking tests pass at 60/120/240 Hz physics dt.
2. Model floor triangles used only their three expanded edge halfspaces, missing
   the X/Y box-axis separating planes. Acute decorative wall triangles therefore
   supported a player far outside the actual triangle footprint. On the real
   middle elevator, center feet falsely settled at 189.19131 instead of 146.59492
   (42.59639 units too high). Walking changed which phantom surface won: the
   diagnostic fell to 178.57 or 158.90. Adding the missing triangle/AABB footprint
   axes removes those false supports; it adds no platform geometry.
3. Ground carry selected any dynamic brush within its probe, ignoring a nearer
   world floor, and returned the first nearby mesh support. The nearest actual
   floor now wins, with world winning coplanar ties. Three explicit ground-identity
   regressions cover a lower train, a nearer train, and coplanar world/train.
   This follows the original binary's distinction between ground entities and
   base velocity, but remains displacement carry rather than full Source parity.
4. The expanded boundary walk exposed a separate failure hidden by the center
   ride: the player walked through the visible cabin side, reached the shaft wall,
   and stalled the train at Z=325.04077 instead of 790. The fix uses continuous
   AABB/triangle SAT for the actual posed rigid side triangles of solid one-clip
   models parented to `func_tracktrain`. It does **not** turn triangles into
   unbounded prism shells. Per-instance bounds reject unrelated geometry.
   Applying mesh collision to every prop also blocked the automatic gun's own
   ray; that rejected experiment is not shipped. Scope is moving train attachments.
   A walkable contact on the cabin's small side lip also participates in ground
   selection, so the train carries it rather than leaving it behind when starting.

`src/mesh_collision.rs` is geometric render-mesh collision, **not decoded PHY or
VPhysics parity**. Rigid initial posed triangles translate with their parent;
animated/deforming collision, general rotated-train collision, door
EnableCollision/DisableCollision parity and full Source ground/base-velocity
semantics remain outside this repair. No assets or original DLLs were modified
or executed. Existing REA ground evidence above was reused; exploratory PHY
inspection did not become an unverified PHY implementation.

### Acceptance actually exercised

- `tests/support_boundaries.rs`: actual middle-lift floor height, repeated input
  walks across triangles, stable support-relative height, eye offset 64, no crouch,
  full authored 576-unit ride. The initial test was red on the 189.19131 false floor.
- `tests/support_cadence.rs`: walk against the north cabin boundary then stand
  through the full ride. 30/60/144 Hz and irregular rendered-frame schedules all
  produce exactly equal final player/velocity/train state at fixed 120 Hz physics.
  The boundary detail creates a real 3.045868-unit step while walking; after
  settling, per-tick support-relative height change stays below 0.1. The original
  no-side-collision version failed the unchanged full-rise assertion at Z=325.04077.
- The old spawn trace now explicitly excludes 120 settling ticks after an authored
  intro viewcontrol horizontally relocates the player. Falling onto its different
  floor is not standing jitter; this does not exclude ordinary simulation motion.
- Correct floor/collision changes exposed a brittle portal run-up in the input
  driver. It now brakes at the two run-up marks and jumps nearer the aperture,
  using ordinary controls only. No pose, velocity or portal state assignments were
  added. Chamber01 full route still verifies automatic gun, pickup, owned channel,
  four crossings, cleanser, authored exit and supported chamber02 arrival.
- Final suite: **126 passing, one explicitly skipped known baseline failure**.
  Running that baseline separately still fails left 1/right 2 at
  `tests/rigid.rs::blocked_maker_does_not_embed_or_fire_spawn_output`.
- Final native Apple M4/Metal window runs exit normally: chamber01 full route;
  chamber00-to-01 elevator transition plus destination walk-off; and the new
  `SOURCE_PARITY_SMOKE=support-boundary SOURCE_PARITY_EXIT=1` fixture.
  The boundary fixture only stages its starting pose; all subsequent movement,
  collision, triggers and lift travel are simulated normally. It is not evidence
  of a continuous authored-spawn chamber00 completion.
- Inspected actual native surface PNGs: boundary frames 180/570 maintain the same
  wall-relative view during and after travel; progression transition-0716 shows
  the destination corridor floor and elevator behind the walked-off player.
  Chamber01 final arrival capture faces the cabin wall; it is not a floor view.
  These are presented GPU surfaces, not desktop/physical-input verification.

Local retained logs (not Git assets): `evidence/support-suite-final.log`,
`support-known-failure-final.log`, `mesh-sides-boundary-tests.log`,
`mesh-sides-route-tests.log`, `support-boundary-native.log`,
`support-chamber01-native-final.log`, `support-progression-native-final.log`.

### Still unresolved

The user's exact persistent stationary tall/small oscillation on Windows has not
been observed directly. The concrete phantom-height, contact-skin fall-through,
wrong support ownership and north-boundary train stall are reproduced and
regression-tested here; they must not be described as proof that every Windows
height/portal report is resolved. Physical Windows keyboard/mouse, original
StepMove/stance parity, complete cabin/animated-door collision and arbitrary
support-boundary routes still need verification. No Bevy migration, invisible
platforms or teleport/physics smoothing was introduced.
