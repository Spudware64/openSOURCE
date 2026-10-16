# Rapier runtime migration — bounded first increment

`rapier3d 0.36.1` (Cargo.lock) now advances ordinary free `prop_physics`
bodies in `Runtime::tick` by default. This is not a standalone physics example.
The momentum-transfer test failed against the old independent sweep solver and
passes through the production runtime after migration.

## Implemented

- Persistent Rapier body handles keyed by entity slot; live body removal when
  held, killed, or leaving this backend. Runtime position and velocity remain the
  adapter boundary, preserving authored spawning, throw impulses and outputs.
- Simultaneous dynamic cuboid contacts, impulses, stacking, CCD and friction.
  Rotation stays locked to preserve the current renderer/collision contract.
- Real solid BSP convex brushes become Rapier convex hull colliders by
  intersecting their halfspaces. Single-plane test floors use halfspaces.
- Z-up, gravity 600 Source units/s², conversion 0.0254 metres/Source unit;
  120 Hz fixed stepping with accumulated remainder. Existing 0.03125 Source-unit
  collision skin is retained as a Rapier contact skin, not a second solver.
- Body readback still publishes the existing player/ray/trigger collider view.
- World collider snapshot is cached. Changes to transformed BSP brushes rebuild
  the scenery colliders; this is **not** velocity-aware kinematic-platform support.

## Explicitly not migrated

- When both portals are linked, free cubes use the previous portal-aware solver
  for the entire tick. Position/velocity portal mapping is deliberately unchanged.
  Removing this scene-wide fallback requires Rapier aperture filtering and
  swept portal substeps; do not claim portal collision is Rapier-backed yet.
- Held cubes retain the existing swept carry/portal ownership constraints.
- The player controller, visibility/placement queries and entity I/O remain
  existing code. No Bevy rewrite, original binaries or game assets changed.
- Posed model triangle support/side colliders and parented elevator kinematic
  bodies are not imported into this first Rapier world. Cube/elevator support
  parity is therefore unfinished; this does not fix the reported elevator issue.
- Automatic prop rotation, PHY shape import and source VPhysics parity remain
  unfinished. Geometry conversion assumes bounded valid convex BSP brushes.

## Verification

- `cargo test --test rapier_runtime`: real cube-to-cube momentum transfer.
- `cargo test --test rigid -- --skip blocked_maker_does_not_embed_or_fire_spawn_output`:
  14 passing, including actual chamber cube/plate/door interaction, posed-mesh
  extent against a floor, stacking, falling, pickup/drop/throw and player blocking.
- `cargo test --test cube_portal_regression --test chamber00_cube_portal --test portals`:
  existing held/free portal and actual-map regressions retained.
- `cargo check --quiet` passes.
- Known baseline `blocked_maker_does_not_embed_or_fire_spawn_output` still fails
  unchanged (left 1, expected 2). No full-suite-green or interactive elevator claim.

Next acceptance slices: import rigid posed scenery as persistent parent-transform
kinematic colliders and test real elevator entry/walking/full ride; migrate
portal apertures/transfers with matching render and collision readback; replace
player collision with Rapier character-controller queries without changing
Source movement rules.

## Linked-portal migration blocker

A production-runtime regression now records cube-to-cube impulse transfer while
both portals are linked (linked_portals_keep_runtime_cube_impulses_active).
It is explicitly ignored pending migration; running it with --ignored fails
because the second cube remains at x=65 instead of moving beyond x=70.

A bounded attempt to use Rapier contact-pair hooks with the existing validated
aperture/hull/exit-clearance predicate and swept position/velocity mapping did
not pass actual_chamber00_thrown_cube_crosses_authored_pair: the real spawned
cube stopped near the source wall (origin approximately -811.912, -320, 148.209)
instead of reaching the authored destination. Suspending CCD only near validated
openings did not resolve this. The cause is not yet isolated; do not assume CCD
alone is responsible. That incomplete implementation was reverted. Free cubes
in linked scenes therefore still use the legacy solver. No elevator/player
migration or portal-physics completion is claimed.
