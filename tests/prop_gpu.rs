use portal_bsp_viewer::{assets::Assets, render_scene, scene::Scene};
#[test]
fn invalid_gpu_scene_returns_error_not_validation_panic() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let mut s = Scene::load(&b, &fs).unwrap();
    s.textures.clear();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        render_scene(&s, 128, 128, None)
    }));
    assert!(
        result.is_ok(),
        "invalid scene must return an error, not panic in wgpu"
    );
    assert!(result.unwrap().is_err());
}
#[test]
fn opaque_doorframe_with_mask_alpha_is_visible_on_gpu() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let props = portal_bsp_viewer::props::static_props(&b).unwrap();
    let mut s = Scene::load(&b, &fs).unwrap();
    let sizes = props
        .models
        .iter()
        .map(|path| {
            let stem = path.trim_end_matches(".mdl");
            portal_bsp_viewer::studio::decode(
                &fs.read(path).unwrap(),
                &fs.read(&format!("{stem}.vvd")).unwrap(),
                &fs.read(&format!("{stem}.dx90.vtx")).unwrap(),
            )
            .unwrap()
            .vertices
            .len()
        })
        .collect::<Vec<_>>();
    let mut offset = s.vertices.len() - s.prop_triangles * 3;
    for p in props.instances {
        let n = sizes[p.model as usize];
        if props.models[p.model as usize].contains("door_01_frame_wide") {
            s.vertices = s.vertices[offset..offset + n].to_vec();
            break;
        }
        offset += n;
    }
    let pixels = render_scene(&s, 128, 128, None).unwrap();
    let colors = pixels
        .chunks_exact(4)
        .collect::<std::collections::HashSet<_>>();
    assert!(
        colors.len() > 20,
        "opaque doorframe must not be discarded due to specular-mask alpha; {} colors",
        colors.len()
    );
}
#[test]
fn real_prop_normals_affect_gpu_shading() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let mut s = Scene::load(&b, &fs).unwrap();
    let start = s.vertices.len() - s.prop_triangles * 3;
    s.vertices = s.vertices[start..start + 3000].to_vec();
    for v in &mut s.vertices {
        v.normal = [0., 0., 1.];
    }
    let a = render_scene(&s, 128, 128, None).unwrap();
    for v in &mut s.vertices {
        v.normal = [0., 0., -1.];
    }
    let z = render_scene(&s, 128, 128, None).unwrap();
    assert!(a != z, "normal shader must change actual real prop pixels");
}
