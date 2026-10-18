use opensource::{assets::Assets, scene::Scene};
#[test]
fn props_use_baked_coloured_light_instead_of_fixed_directional_white() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let s = Scene::load(&b, &fs).unwrap();
    assert!(
        !s.vertices.iter().any(|v| v.unlit == 2.),
        "fixed white prop light still enabled"
    );
}

#[test]
fn overbright_lightmaps_preserve_rgb_chromaticity() {
    let mut b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let offset = i32::from_le_bytes(b[136..140].try_into().unwrap()) as usize;
    let len = i32::from_le_bytes(b[140..144].try_into().unwrap()) as usize;
    for p in b[offset..offset + len].chunks_exact_mut(4) {
        p.copy_from_slice(&[200, 100, 50, 2]);
    }
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let s = Scene::load(&b, &fs).unwrap();
    let p = (2 * opensource::scene::LIGHT_SIZE + 2) * 4;
    assert_eq!(&s.lightmap[p..p + 3], &[255, 128, 64]);
}
