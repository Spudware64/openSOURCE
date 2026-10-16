# Native first-person weapon

The interactive game-root renderer loads `WeaponData.viewmodel` from the shipped weapon script, decodes MDL/VVD/VTX and skins the original idle at time zero. Source X-forward/Y-left/Z-up coordinates feed a camera-local projection (54° horizontal at reference 4:3). This is a static idle, not draw/fire animation parity. Diffuse textures are original; normal/phong and screen-space refraction remain unsupported. The shipped model references a missing `v_hands.vtf`; that material is transparent, not a fabricated replacement.

The viewmodel owns a separate GpuScene and camera uniforms. It is drawn after the world and portal composites with loaded color and freshly cleared depth, only on the main surface and only when Runtime portal inventory has an owned channel. It is never part of linked remote world views. No per-frame skinning was added.

Direct-start launcher policy (not asserted as Valve spawn behavior): on initial Portal map launch only, consume an enabled, unparented `weapon_portalgun` whose authored targetname begins `portalgun_spawn_lvl`. Consumption uses the ordinary pickup implementation: preserve authored channel availability, enqueue OnPlayerPickup, kill only that entity and hide its model slot. Chamber00 and chamber01 do not qualify. Map transitions do not invoke this policy. Ordinary E pickup is unchanged; no blanket weapon deletion, proximity grant or all-map inventory grant was added.

Validation: real original-asset tests cover decoded textured authored idle, direct launch chamber00/chamber01 unowned, chamber03 blue-only owned and corresponding world-slot hidden. Metal presented-surface images are kept outside version control under evidence/. No Windows verification is claimed. Full-suite verification is not claimed.
