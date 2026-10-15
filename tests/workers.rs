use portal_bsp_viewer::{assets::Assets, scene::Scene};
#[test]
fn single_and_four_worker_assets_are_byte_identical() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let a = Scene::load_with_workers(&b, &fs, 1).unwrap();
    let z = Scene::load_with_workers(&b, &fs, 4).unwrap();
    assert_eq!(a.textures, z.textures);
    assert_eq!(a.lightmap, z.lightmap);
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(&a.vertices),
        bytemuck::cast_slice::<_, u8>(&z.vertices)
    );
    assert_eq!(a.warnings, z.warnings);
}
