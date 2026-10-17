# Held cube floor-portal checkpoint

## Confirmed defect and bounded fix

A held cuboid whose centre is still in front of a floor portal can already
penetrate its backing floor by its support extent. A subsequent lateral grab
sweep can leave the aperture. The Rapier whole-brush cast starts overlapping,
so it has no new convex entry face and permits movement into solid floor.
The player path already retained bounded aperture-rim slices; held sweeps did
not. `World::carry_portal_rim` now shares those boundary queries between the
player and held Rapier cast. The nearest rim hit competes with ordinary Rapier
contacts. This does not teleport the cube to the target, enlarge the aperture,
disable collision, rotate the upright hull, or rebuild scenery colliders.

The finite-floor regression was rerun with the new held rim query disabled:
roll 0, tick 0 incorrectly moved the cube from (0,0,8) to (0,-100,8).
With the query enabled the same test passes at floor rolls 0/45/90 over
120 held ticks and verifies the Rapier collider follows the model after drop.

## Acceptance exercised

- Finite backing-floor rim rejection at 0/45/90 degrees.
- Actual chamber00 floor, maker-spawned cube, posed-model bounds, authored orange
  portal, staged floor blue portal and partly inserted approach; 120 held ticks
  and dropped Rapier collider/model centre registration at all three rolls.
- Synthetic floor-to-wall fixture using real player stepping and production carry:
  independently observed cube-first and player-first orders at all three rolls,
  retrieval by ordinary movement, 120 stable hold ticks at the correct current
  eye target, then dropped Rapier/model centre agreement.
- Existing real-map authored wall-pair forward and backward/player-first cases.
- Library tests, cube portal tests, portal fitting/orientation/height/traversal
  tests and `cargo check` pass. Full suite not run; AGENTS.md baseline excluded.

## Visual evidence and remaining limits

The supplied 16.417-second video was inspected as a contact sheet and at
quarter-second intervals around 1.8–4.2 seconds. The player view transfers while
the cube subsequently appears partly hidden by the orange backing wall, and
later near an upper rim. Images alone do not establish collider separation or
prove that all these visuals are caused by the confirmed floor-rim leak.

Fine contact sheet (untracked external evidence):
`/Users/joshua/.hermes/profiles/duncan/cache/scratch/cube-floor-fine.jpg`.

No new native GPU capture was made in this checkpoint. Exact video-sequence
visual parity, continuous dual-side clipped cube rendering and rotation parity
are **not verified/fixed** here. The current cube model/hull remains upright;
portal transfer rotates velocity and carry targets, not full rigid orientation.
Synthetic transfer-order coverage is not a claim that both orders were exercised
through the newly placed floor portal in the actual chamber. No Windows claim.
