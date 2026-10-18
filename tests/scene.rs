use opensource::{assets::Assets, scene::Scene};
#[test]
fn real_chamber_has_textures_uvs_baked_lighting_and_no_nodraw() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let s = Scene::load(&b, &fs).unwrap();
    assert!(s.textures.len() > 20);
    assert!(s.filtered_faces > 100);
    assert!(s.lightmapped_faces > 100);
    assert!(s.vertices.len() - s.prop_triangles * 3 < opensource::load(&b).unwrap().len());
    assert!(s
        .vertices
        .iter()
        .all(|v| v.uv.iter().chain(v.light_uv.iter()).all(|n| n.is_finite())));
    assert!(s.lightmap.chunks_exact(4).any(|p| p[0] > 0 && p[0] < 255));
    assert!(
        s.warnings.iter().all(|w| w.contains("Refract")
            || (w.starts_with("initial pose models/")
                && w.ends_with("unknown/unsupported sequence idle; bind fallback"))
            || (w.starts_with("prop models/props_animsigns/signage_num0")
                && w.contains("unsupported MDL header/version/length: id="))),
        "{:?}",
        s.warnings
    );
    eprintln!(
        "textures {}, triangles {}, filtered {}, lightmapped {}, warnings {:?}",
        s.textures.len(),
        s.vertices.len() / 3,
        s.filtered_faces,
        s.lightmapped_faces,
        s.warnings
    );
}
