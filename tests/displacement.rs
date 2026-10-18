use glam::Vec3;
use opensource::displacement::tessellate;
#[test]
fn real_map_uses_displaced_positions_not_flat_quads() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_15.bsp").unwrap();
    let fs = opensource::assets::Assets::from_game("../portal-original/portal", Some(&b))
        .unwrap();
    let scene = opensource::scene::Scene::load(&b, &fs).unwrap();
    assert_eq!(scene.displacement_triangles, 2048);
    assert_eq!(scene.displacement_faces, 4);
    // Actual DISP_VERTS[1], independently evaluated from the source grid:
    // start (-1904,496,-319), column step +6 X, direction -Z, distance 4.850190.
    let expected = Vec3::new(-1898., 496., -323.8502);
    assert!(scene
        .vertices
        .iter()
        .any(|v| Vec3::from(v.position).distance(expected) < 0.001));
    assert!(scene.vertices.iter().any(|v| v.position[2] < -319.1));
}
#[test]
fn source_rows_advance_to_corner_one_and_columns_to_corner_three() {
    let c = [
        Vec3::ZERO,
        Vec3::Y * 4.,
        Vec3::new(4., 4., 0.),
        Vec3::X * 4.,
    ];
    let mut info = vec![0; 176];
    info[20..24].copy_from_slice(&2i32.to_le_bytes());
    let mut verts = vec![0; 25 * 20];
    for a in verts.chunks_exact_mut(20) {
        a[8..12].copy_from_slice(&1f32.to_le_bytes());
    }
    verts[20 + 12..20 + 16].copy_from_slice(&7f32.to_le_bytes());
    let mesh = tessellate(&c, &info, &verts, &[0; 64]).unwrap();
    assert!(mesh
        .iter()
        .filter(|v| v.position.z == 7.)
        .all(|v| v.base == Vec3::X));
    assert!(mesh.iter().all(|v| v.normal.z > 0.));
}
#[test]
fn malformed_displacement_bounds_power_corner_and_floats_are_rejected() {
    let c = [
        Vec3::ZERO,
        Vec3::Y * 4.,
        Vec3::new(4., 4., 0.),
        Vec3::X * 4.,
    ];
    let mut info = vec![0; 176];
    info[20..24].copy_from_slice(&2i32.to_le_bytes());
    for (offset, value) in [
        (12, -1i32),
        (12, i32::MAX),
        (16, -1),
        (20, 5),
        (0, f32::NAN.to_bits() as i32),
        (24, f32::NAN.to_bits() as i32),
    ] {
        let mut bad = info.clone();
        bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        assert!(tessellate(&c, &bad, &[0; 500], &[0; 64]).is_err());
    }
    assert!(tessellate(&c, &info, &[0; 499], &[0; 64]).is_err());
    assert!(tessellate(&c, &info, &[0; 500], &[0; 63]).is_err());
    let mut v = vec![0; 500];
    v[12..16].copy_from_slice(&f32::INFINITY.to_le_bytes());
    assert!(tessellate(&c, &info, &v, &[0; 64]).is_err());
}
#[test]
fn elevation_follows_source_outward_normal() {
    let c = [
        Vec3::ZERO,
        Vec3::Y * 4.,
        Vec3::new(4., 4., 0.),
        Vec3::X * 4.,
    ];
    let mut info = vec![0; 176];
    info[20..24].copy_from_slice(&2i32.to_le_bytes());
    info[24..28].copy_from_slice(&3f32.to_le_bytes());
    let mesh = tessellate(&c, &info, &[0; 500], &[0; 64]).unwrap();
    assert!(mesh.iter().all(|v| v.position.z == 3. && v.base.z == 0.));
}
#[test]
fn start_corner_rotates_grid_and_preserves_base_uv_positions() {
    let corners = [
        Vec3::ZERO,
        Vec3::X * 4.,
        Vec3::new(4., 4., 0.),
        Vec3::Y * 4.,
    ];
    let mut info = vec![0u8; 176];
    info[0..4].copy_from_slice(&4f32.to_le_bytes());
    info[20..24].copy_from_slice(&2i32.to_le_bytes());
    let mut verts = vec![0u8; 25 * 20];
    for a in verts.chunks_exact_mut(20) {
        a[8..12].copy_from_slice(&1f32.to_le_bytes());
        a[12..16].copy_from_slice(&2f32.to_le_bytes());
    }
    let mesh = tessellate(&corners, &info, &verts, &[0; 32 * 2]).unwrap();
    assert_eq!(mesh.len(), 32 * 3);
    assert_eq!(mesh[0].base, corners[1]);
    assert_eq!(mesh[0].position, corners[1] + Vec3::Z * 2.);
    assert!(mesh.iter().all(|v| v.normal.dot(-Vec3::Z) > 0.99));
    assert!(mesh.iter().any(|v| v.base == corners[0]));
}
