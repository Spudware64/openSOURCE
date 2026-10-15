use portal_bsp_viewer::{assets::Assets, scene::Scene};
#[test]
fn chamber_renders_all_53_real_static_props() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let s = Scene::load(&b, &fs).unwrap();
    assert_eq!(s.static_props, 53);
    assert_eq!(s.skipped_props, 0);
    assert!(s.prop_triangles > 10000);
    assert!(s
        .vertices
        .iter()
        .all(|v| v.normal.iter().all(|n| n.is_finite())));
    eprintln!(
        "props {}, skipped {}, prop triangles {}, scene triangles {}, layers {}",
        s.static_props,
        s.skipped_props,
        s.prop_triangles,
        s.vertices.len() / 3,
        s.textures.len()
    );
}
