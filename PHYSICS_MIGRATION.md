# Rapier runtime migration — free-body portal increment

Rapier 0.36.1 now advances all free prop_physics bodies in Runtime::tick,
including linked-portal scenes. The scene-wide legacy free-body fallback and
its independent swept-body integration have been removed.

## Implemented

- Persistent Rapier body handles, simultaneous cube contacts and impulses,
  stacking, friction and CCD away from validated portal openings.
- Solid BSP convex brushes imported as Rapier convex hulls; synthetic single
  planes imported as halfspaces. World geometry snapshot caching remains.
- Z-up, gravity 600 Source units/s², 0.0254 metres/Source unit, 120 Hz fixed
  stepping and existing 0.03125-unit contact skin. Cube rotations remain locked.
- Portal contact hooks remove backing-plane solver manifolds only for bodies
  satisfying the existing aperture-fit/exit-clearance predicate. Other normals
  on that brush and dynamic body/body contacts remain enabled.
- A body disables Rapier CCD only while a validated opening is within its swept
  reach. Centre crossings map the resulting position and linear velocity after
  each fixed substep. This is not a legacy movement fallback.
- Spawn/external-pose floor recovery uses Rapier/Parry cuboid shape casts. The
  actual-map blocker was an authored cube pose about 0.19 units inside the floor:
  destination clearance stayed rejected until the wall solver had removed the
  throw momentum. Recovering the floor overlap before solving fixes that case.
- Held-object movement now casts a Parry cuboid against the production Rapier
  collider set (imported BSP convex/halfspace shapes and free-body colliders),
  replacing the old handwritten world/prop sweep. Existing sliding, portal
  ownership and player-crossing mapping remain; this is query-driven carry,
  not a simulated grab joint or held-body impulse transfer. Aperture exclusion
  requires the shared fit/exit predicate and a backing-plane hit normal.
- Runtime::rapier_collider_center exposes actual collider pose readback; the
  authored held/player-crossing/drop test now checks that pose after release.
- Runtime readback still publishes the existing collider view for gameplay.

## Verification

`cargo test --quiet --test rapier_runtime --test chamber00_cube_portal
--test cube_portal_regression --test rigid --
--skip blocked_maker_does_not_embed_or_fire_spawn_output`:
25 passed, one known baseline failure excluded. The carry shape-cast unit test
also passes (world and dynamic-body blocking); the runtime wall hold regression
checks 240 ticks followed by actual Rapier collider readback after dropping.

The integration-wide run `cargo test --quiet --
--skip blocked_maker_does_not_embed_or_fire_spawn_output` also exited 0 with no
failures. This remains a filtered-suite result, not an unqualified green suite.

This includes real runtime cube-to-cube impulses with linked portals (the
previously ignored regression is enabled), actual authored chamber00 thrown
portal transit, actual held-cube/player crossing and destination-side drop,
ordinary-wall rejection in a linked scene, and blocked-destination rejection.

## Parent-frame carry correction (separate from Rapier migration)

Runtime now resolves support-parent transforms before carrying the rider, including
rotation, rather than subtracting raw child origins. A new synthetic parented-floor
regression failed before the correction (motion transaction stalled at Z=0) and
passes afterward. Existing authored elevator ride/dismount, support-boundary,
cadence, identity and rotating-parent tests pass. The filtered integration suite
also passes; the known blocked-maker failure remains excluded. This does not
establish input-only original-spawn elevator entry or correct the reported model
floor alignment.

## Production player character queries

The default Player path now uses Rapier's Parry narrowphase for BSP/rigid
brush sweeps, ground/step probes, portal rim slices, and posed train floor/side
queries. There is no opt-in flag or old convex/SAT swept collision fallback.
Source acceleration, gravity, friction, standing/crouching, step selection,
portal eligibility/clearance and parent-resolved rider carry remain policy.
The hull remains an upright 32x32x72 box (crouch uses its existing height).

`player_query.rs` builds configuration-space obstacles using the box support
function. Plane intersections are calculated in f64, then localized and scaled
at 0.0254 metres/unit. Parry convex hull construction produces a triangle surface;
Parry BVH triangle ray queries determine entry TOI, followed by the existing
0.03125-unit Source skin and authored-normal snapping. Synthetic single planes
use Parry halfspaces. This is a query-driven kinematic box, not a dynamic capsule
or Rapier's stock character-controller movement policy. Direct convex GJK casts
were rejected: even convex ray casts use support-map tolerances and missed tiny
floor gaps. Triangle-surface ray queries pass the unchanged contact_epsilon cases.

The expanded shapes are cached by planes and hull extents. Separating halfspace
rejection happens before cache lookup; triangle groups retain swept AABB culling.
World brush candidate enumeration is still linear (not a scene-wide BVH), and
moving plane keys can rebuild cached shapes. This is a remaining performance
limitation, not a claim of broadphase-complete migration. Narrowphase invocation
counts are exposed as a simulation-thread diagnostic and tested in default Player
movement. Query colliders reflect current World geometry/body readback; they are
not additional dynamic solver bodies or velocity-aware kinematic Rapier bodies.

Verification: the unchanged contact_epsilon test passes all 60/120/240 Hz cases;
portal_height retains actual-floor partial-insertion rim and rejected-crossing
coverage. Actual-map chamber00 cube/player portal crossing, elevator, support
boundary/cadence and parented-support tests pass. The integration-wide filtered
suite exited 0. Its known blocked-maker exclusion remains explicit below.
No physical-input/native-GPU/Windows acceptance was performed for this change.

## Still unfinished — do not call this a complete migration

- Held carry remains query-driven, without a dynamic grab joint, held-to-free
  impulse transfer or angular dynamics. Shape casts currently scan the collider
  set; broad-phase acceleration is still future performance work.
- Player movement policy and overlap/portal-clearance predicates remain Source-specific;
  collision TOI queries now run Parry as described above.
- Posed train mesh floor/side character queries now use Parry configuration-space
  geometry, but velocity-aware kinematic elevator solver bodies are still absent.
  Existing transformed BSP world geometry is scenery, not advanced as kinematic
  bodies. Existing elevator regression routes pass; original-spawn input-only
  approach and the reported visual support-height alignment remain unverified.
- CCD suppression is per-body near approved openings, not per-contact CCD.
  Thin unrelated obstacles at extreme speeds and destination-side collisions
  within the mapped remainder of a substep need additional acceptance tests and
  scoped continuous queries/substep splitting. No broad portal-parity claim.
- PHY shapes, angular dynamics, full Source VPhysics parity and Windows runtime
  verification remain absent. No original assets or binaries were changed.
- Known baseline blocked_maker_does_not_embed_or_fire_spawn_output remains
  excluded above (historically left 1, expected 2); this is not a full-suite-green
  claim.
