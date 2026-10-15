use portal_bsp_viewer::{assets::Assets, render_scene, scene::Scene};
#[test]
fn textured_scene_reaches_real_gpu() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let scene = Scene::load(&b, &fs).unwrap();
    let p = render_scene(
        &scene,
        256,
        256,
        Some(([-608., -347., 220.], [-1000., -347., 220.])),
    )
    .unwrap();
    assert_eq!(p.len(), 256 * 256 * 4);
    let distinct = p
        .chunks_exact(4)
        .map(|p| p.to_vec())
        .collect::<std::collections::HashSet<_>>();
    assert!(
        distinct.len() > 500,
        "must show detailed material/lightmap pixels"
    );
}
