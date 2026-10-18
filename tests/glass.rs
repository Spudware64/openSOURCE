use opensource::{assets::Assets, scene::Scene};
#[test]
fn refract_glass_has_translucent_coverage_instead_of_error_texture() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let scene = Scene::load(&b, &fs).unwrap();
    assert!(
        !scene.warnings.iter().any(|w| w.contains("Refract")),
        "{:?}",
        scene.warnings
    );
    let glass: Vec<_> = scene.vertices.iter().filter(|v| v.unlit < 0.).collect();
    assert!(!glass.is_empty(), "glass must enter the blended pass");
    assert!(
        glass.iter().any(|v| scene.textures[v.layer as usize]
            .chunks_exact(4)
            .any(|p| p[3] < 100)),
        "clear glass must retain low coverage"
    );
}
