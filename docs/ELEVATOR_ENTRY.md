# Elevator corridor-entry correction

The supplied entry video shows the camera descending into the cab. Reproducing
entry from (-1360,-864,144.1), then using ordinary movement, exposed a step
hand-off defect, not an incorrect model origin or eye offset. The entrance rim
supports feet at 145.29724; the posed original cabin floor is about 146.55.
One-way floor queries ignore feet already below their top. Horizontal movement
was unblocked, so the controller never tried its existing 18-unit step path;
subsequent downward sweeps missed the higher floor. Centre-staged riders bypassed
this failure. No geometry, floor height, player clamp or view offset was added.

The controller now considers the ordinary raised route while moving grounded,
and accepts an equally advanced horizontal route when it raises the player onto
queried geometry. Existing clearance and overhead rejection remain intact.

`tests/elevator_entry.rs` stages outside the mouth, settles, walks through the
boundary, stops, and compares feet with a plane derived from original posed
triangles covering cabin centre. Reversing only the production fix makes this
exact regression fail (exit 101); restoring it passes. No pose edits occur after
fixture initialization. This is not an original-spawn puzzle route or physical
keyboard test.

Native Metal verification uses `SOURCE_PARITY_SMOKE=elevator-entry`,
`SOURCE_PARITY_EXIT=1`, and `SOURCE_PARITY_CAPTURE=<external directory>` with
`--game ../portal-original/portal --map maps/testchmb_a_00.bsp --play`.
The same camera captures approach, boundary crossing, stopping and complete ride.
Inspected frames 15/30/330 show the floor below the eye, not the below-floor view
in the user video. Native feet are 146.58545 during entry, 146.59492 after entry,
and 722.5949 at the top with train Z=790, all grounded. Capture/log artifacts are
outside Git in the profile scratch directory, `elevator-entry/` and
`elevator-entry-native.log`.

Targeted elevator entry/ride, support boundaries/cadence/parent, portal-height
(including running jump), both cube portal tests, chamber01 route, movement
response, contact epsilon and ground identity pass. `cargo check --quiet` passes.
Integration-wide filtered run exits 0 with the two known baseline tests excluded:
`blocked_maker_does_not_embed_or_fire_spawn_output` and
`actual_weapon_pickup_fires_its_exit_outputs_once`. An earlier run confirmed the
latter still fails its door assertion; no assertion was weakened. Not an
unqualified full-suite pass. Windows and PHY/VPhysics parity remain unverified.
