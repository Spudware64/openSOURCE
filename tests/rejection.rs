use portal_bsp_viewer::{assets::Assets, play::World, scene::Scene};
#[test]
fn rejects_unknown_source_branch_and_cyclic_collision_tree() {
    let original = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let mut b = original.clone();
    b[4..8].copy_from_slice(&21i32.to_le_bytes());
    assert!(World::parse(&b).is_err());
    let mut b = original.clone();
    let nodes = portal_bsp_viewer::lump(&b, 5, 32).unwrap();
    let off = nodes.as_ptr() as usize - b.as_ptr() as usize;
    b[off + 4..off + 8].copy_from_slice(&0i32.to_le_bytes());
    assert!(
        World::parse(&b).is_err(),
        "cyclic node must not silently become incomplete collision"
    );
}
#[test]
fn malformed_material_and_lighting_indices_fail_without_panics() {
    let original = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&original)).unwrap();
    for id in [2, 6, 8, 43, 44] {
        let mut b = original.clone();
        b[8 + id * 16 + 8..8 + id * 16 + 12].copy_from_slice(&99i32.to_le_bytes());
        assert!(Scene::load(&b, &fs).is_err());
    }
    let mut b = original.clone();
    let data = portal_bsp_viewer::lump(&b, 2, 32).unwrap();
    let off = data.as_ptr() as usize - b.as_ptr() as usize;
    b[off + 12..off + 16].copy_from_slice(&i32::MAX.to_le_bytes());
    assert!(Scene::load(&b, &fs).is_err());
}
