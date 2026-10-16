# Fired portal orientation

Floor and ceiling portals take their top axis from the firing ray's horizontal heading, projected onto the hit plane. The top therefore points away from the shooter. Their right axis is `up.cross(normal)`, retaining a right-handed frame on both surfaces. Wall portals remain world-Z upright.

For vertical or near-vertical player shots (horizontal ray length at most 0.00001 after normalization), placement uses the player's yaw heading. Scripted placement without a usable heading falls back deterministically to world Y. Orientation is captured only when firing, not updated by subsequent camera movement. Rendering, footprint validation, linked views and traversal use the same stored portal frame. Authored portal angles are unchanged.

Regression verification: `cargo test --test portal_orientation --test portals --test cube_portal_regression --test chamber00_cube_portal` and `cargo check`.
