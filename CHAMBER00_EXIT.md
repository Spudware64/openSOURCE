# Chamber00 blue2: fixed supporting-plane placement

## Fix and verification

The raw blue2 normal -Y is tangent to its actual supporting BSP face:
`concrete/concrete_modular_wall001e`, normal +X, on X=-1344, spanning
Y=[-320,-192], Z=[576,704]. This is the exposed portalable face of brush 299.
There is no corresponding -Y supporting face at Y=-224. Lateral bumping on
that nonexistent plane cannot fix the overlap. Authored activation now resolves
tangent poses onto the actual eligible supporting face and uses the existing
connected-patch fitter. Blue2 centre remains (-1344,-224,631.86); normal becomes
+X. Blue1, blue3 and red1 retain their centres and normals. Aligned placements
and unsupported floating/moving-model poses retain their legacy transforms.
Initial activation is fitted at first surface-enabled sync; entity/channel names
and cycling timing remain unchanged. No brush or door collision is disabled.
This is a geometry-based correction, not verified Valve DLL placement parity.

The updated real-cube pickup/carry/drop test fails before the correction with
crossings=0 and passes afterward with crossings=1, destination approximately
(-1316.657,-224,578.016), still using blue2. It stages approach poses, not a full
input-only route. The old closed-door crossing rejection assertion was invalid:
the gate is downstream of the correctly oriented destination portal.
A second test checks supporting material/plane and unchanged blue1/blue3/red1.

`BLUE2_CAPTURE=evidence/blue2-fitted-gpu.png cargo test --quiet --test chamber00_exit -- --nocapture`
produced an Apple M4 Metal offscreen render using advanced runtime model origins
and portal pair. Inspected view shows the room floor and round downstream exit,
without perpendicular slabs spanning the portal. This does not verify physical
input, a native presented window or every animated model pose.

Targeted exit, held/thrown cube, Rapier, cube regression, rigid, portal-height/rim
and contact-epsilon tests pass (known blocked-maker failure excluded); carry
scene/dynamic-collider unit test passes. The integration-wide filtered run stops
at gun_pickup::actual_weapon_pickup_fires_its_exit_outputs_once. This failure was
also reproduced with HEAD's unmodified portal.rs. No full-suite-green claim.
Original assets unchanged; generated GPU image/logs remain untracked.

## Historical pre-fix characterization (superseded)


`tests/chamber00_exit.rs` is a known-defect characterization, not a passing crossing acceptance test. It settles startup before entering the second room, uses the authored cycling blue2 at (-1344,-224,631.86), stages approach poses for a real maker-spawned cube, and uses production pickup/carry/drop. No button output or door collision is forced. Leaving the player plate after drop is essential: its authored outputs disable the cube trigger while occupied.

The actual cube settles at approximately (-1376,-569,604.22); the authored left door moves from Y=-285 to Y=-329. Nevertheless orange-to-blue2 walking remains blocked at (-1088,-615.96875,576.03125), crossings=0. The same exact pair is awaited again after carrying, avoiding a false positive from blue1 in the cycling sequence.

Destination standing hull overlap identifies World brush 299, bounds X=[-1360,-1344], Y=[-320,-192], Z=[576,704]. This is a perpendicular static sidewall, not the opened door. The blue2 centre is on its +X boundary while its imported normal points -Y. This proves an open-gate failure, but does not yet establish the correct production fix or material provenance. Do not treat the failure as expected closed-door gameplay, remove the wall, or change portal orientation without surface evidence.

Native GPU reproduction/screenshots and a production fix remain outstanding. Original assets were not changed.
