# High-energy pellet increment

Verified locally against `testchmb_a_04.bsp`:

- Launcher class is **point_energy_ball_launcher**, not point_combine_ball_launcher. `energy_ball_launcher_lvl5` starts disabled (spawnflags 4096); normal map trigger enables it after two seconds. Authored radius 12, speed 200, lifetime 16, respawn delay 2, minimum life after portal 6.
- Catcher is **trigger_multiple** `ball_trap_10`, with **filter_combineball_type** `filter_combine_ballpermit_alt` (balltype 3), and a decorative `prop_dynamic` housing. Its actual OnTrigger outputs disable the launcher/catcher and start `rail_cart_lvl5` via StartForward, alongside animations, lights and overlay state. No puzzle target names are hard-coded in the detector.
- Projectiles sweep a radius-sized hull using existing MASK_SOLID queries, reflect, map position and velocity through active paired portals, expire and respawn. At most 64 pellets and eight collisions per runtime tick. Coordinates remain Source units, consistent with World queries; runtime ticks are normally 120 Hz.
- Launch and impact events use original discovered sound-script names NPC_CombineBall.Launch and NPC_CombineBall.Impact, with positional origins.
- Native rendering is an **approximation**: unlit orange procedural spheres, not Source additive sprites, glow/bloom, or the original particle material. Uses the scene depth/clip pipeline in main and portal views.

Validation: `cargo test --test pellet --quiet` (5 pass), `cargo test --test portals --quiet` (15 pass), `cargo build --quiet`. Catcher regression moves an already launched projectile to the authored trigger approach: this is a detector/I/O fixture, **not** an autonomous solved portal-puzzle route.

Native Apple M4 Metal run produced `evidence/pellet-native.png` with the orange pellet visible in the chamber. Invocation: SOURCE_SMOKE=1 SOURCE_SMOKE_POSITION='-392 128 64' SOURCE_SMOKE_ANGLES='0 40 0' SOURCE_SCREENSHOT=... cargo run -- --game ../portal-original/portal --map maps/testchmb_a_04.bsp --play. Normal trigger starts launcher. The smoke runner remains open after capture; process was stopped by timeout after 100 seconds. Screenshot is untracked and not shipped.

## Remaining limitations (do not claim full Portal parity)

Player pellet hazard/death/reset policy is not implemented. Collider sweep is an axis-aligned radius hull, not an exact sphere. Dynamic Rapier props are not included by trace_cuboid. No exact original DLL behavior reconstruction, direction/type/negation filtering parity, ballcount >1, random maxspeed range, launcher LaunchBall input, explosion/whiz loop sounds, or BSP excludednpc brush special handling. Catcher overlap is a swept expanded AABB; authored trigger brush shape/directional rejection needs refinement. Portal traversal tested in an isolated fixture, not a completed actual-map route. Windows unverified. Existing unrelated rigid-maker baseline failure is untouched. Full suite not rerun for this bounded increment.
