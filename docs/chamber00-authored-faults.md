# Chamber00 authored-chain investigation

## Confirmed missing second-room blue portal
The immutable original testchmb_a_00 BSP entry trigger *32 at (-1088,-512,592) fires portal_red_1 SetActivatedState=1, case_spawn_portals InValue=1, and portal_cycle_timer Enable. The case routes OnCase01/02/03 to logic_make_blue_1/2/3. Those relays fizzle the previous blue portal and activate the next after 0.01 seconds. The five-second timer adds 1 to count_portal_chambers; its OutValue feeds the case and OnHitMax resets to 1.

Runtime::apply has no logic_case InValue or math_counter Add/SetValue handler, and Runtime::tick has no logic_timer scheduler. The new real-BSP characterization test actually touches the entry trigger: red activation is received, blue_1 activation is absent, and logic_case:InValue is diagnosed. This is a real missing generic entity-I/O path, not a gun scheduler failure or MDL warning. The test records the current missing behavior, not a fixed puzzle. Implementing only initial blue activation would still leave cycling broken.

## Floating cube: unresolved Windows observation
The authored box is prop_physics, spawnflags 256, at (-844.301,-1043.5,676.197). Its point_template uses flags 3; maker origins are (-832,-1056,560) and (-804,-556,640). The normal door-close trigger schedules logic_drop_box after two seconds. That relay opens the cover animation and enables trigger_drop_new_box. The filtered cube trigger issues Wake and, on departure, ForceSpawn plus cover close. Runtime applies gravity to physical bodies; Wake currently does nothing. There is no authored EnableMotion/DisableMotion command in this BSP to explain floating by itself.

The existing real_spawn_pickup_walk_drop_opens_door regression passed locally (1 test): actual scene binding, drop relay, pickup, walk/drop on plate, opened-door traversal and repickup. This does not reproduce or disprove the Windows floating report. Need the actual Windows CSV and exact build/assets before changing body positions or constants. No artificial cube movement was added.

## Elevator entrance: still unverified
Player::step only attempts its 18-unit step trial when grounded horizontal progress is blocked. A one-way render-mesh floor above the feet can be skipped without reducing horizontal progress, so that gate is a plausible entrance-seam failure. Existing center-spawn/riding tests do not exercise approach from the corridor. No speculative controller change was made. A useful next regression must walk across the real entrance from corridor support, then stop inside and ride; standing after noclip is not acceptance.

## MDL diagnostics
Rejected complete MDL headers now report observed magic, version, declared length and file length, plus supported versions. This allows comparison of Steam asset variants without assuming version mismatch. The local real-scene test continues to permit only the known signage decode warnings and known bind-pose/refraction warnings. Initial idle bind fallback for cover/decorations does not establish failure of an explicitly requested open sequence. These diagnostics do not fix missing format support.

Verification: studio 4/4, scene 1/1, authored-chain characterization 1/1, targeted real cube route 1/1. No full suite or GPU/Windows run; known blocked-maker baseline failure was not touched.
