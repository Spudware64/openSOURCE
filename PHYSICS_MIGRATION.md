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
- Runtime readback still publishes the existing collider view for gameplay.

## Verification

`cargo test --quiet --test rapier_runtime --test chamber00_cube_portal
--test cube_portal_regression --test rigid --
--skip blocked_maker_does_not_embed_or_fire_spawn_output`:
24 passed, one known baseline failure excluded.

The integration-wide run `cargo test --quiet --
--skip blocked_maker_does_not_embed_or_fire_spawn_output` also exited 0 with no
failures. This remains a filtered-suite result, not an unqualified green suite.

This includes real runtime cube-to-cube impulses with linked portals (the
previously ignored regression is enabled), actual authored chamber00 thrown
portal transit, actual held-cube/player crossing and destination-side drop,
ordinary-wall rejection in a linked scene, and blocked-destination rejection.

## Still unfinished — do not call this a complete migration

- Held-object collision/carry constraints still use the old swept adapter.
- Player collision/controller still uses the old implementation.
- Posed model triangles and velocity-aware kinematic elevator colliders have
  not been imported. Existing transformed BSP world geometry is rebuilt as
  scenery, not advanced as kinematic bodies. Elevator entry/ride/exit migration
  and the reported support-height regression are not fixed by this increment.
- CCD suppression is per-body near approved openings, not per-contact CCD.
  Thin unrelated obstacles at extreme speeds and destination-side collisions
  within the mapped remainder of a substep need additional acceptance tests and
  scoped continuous queries/substep splitting. No broad portal-parity claim.
- PHY shapes, angular dynamics, full Source VPhysics parity and Windows runtime
  verification remain absent. No original assets or binaries were changed.
- Known baseline blocked_maker_does_not_embed_or_fire_spawn_output remains
  excluded above (historically left 1, expected 2); this is not a full-suite-green
  claim.
