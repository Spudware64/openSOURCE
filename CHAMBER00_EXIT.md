# Chamber00 blue2: confirmed independent of door gate

`tests/chamber00_exit.rs` is a known-defect characterization, not a passing crossing acceptance test. It settles startup before entering the second room, uses the authored cycling blue2 at (-1344,-224,631.86), stages approach poses for a real maker-spawned cube, and uses production pickup/carry/drop. No button output or door collision is forced. Leaving the player plate after drop is essential: its authored outputs disable the cube trigger while occupied.

The actual cube settles at approximately (-1376,-569,604.22); the authored left door moves from Y=-285 to Y=-329. Nevertheless orange-to-blue2 walking remains blocked at (-1088,-615.96875,576.03125), crossings=0. The same exact pair is awaited again after carrying, avoiding a false positive from blue1 in the cycling sequence.

Destination standing hull overlap identifies World brush 299, bounds X=[-1360,-1344], Y=[-320,-192], Z=[576,704]. This is a perpendicular static sidewall, not the opened door. The blue2 centre is on its +X boundary while its imported normal points -Y. This proves an open-gate failure, but does not yet establish the correct production fix or material provenance. Do not treat the failure as expected closed-door gameplay, remove the wall, or change portal orientation without surface evidence.

Native GPU reproduction/screenshots and a production fix remain outstanding. Original assets were not changed.
