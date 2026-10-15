use portal_bsp_viewer::{assets::Assets, studio::decode};
#[test]
fn chamber_contains_dynamic_dropper_and_elevator_geometry() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let s = portal_bsp_viewer::scene::Scene::load(&b, &fs).unwrap();
    for (origin, radius) in [
        (glam::Vec3::new(-830.233, -1054.27, 276.), 80.),
        (glam::Vec3::new(-1472., -864., 145.), 100.),
    ] {
        assert!(
            s.vertices
                .iter()
                .filter(
                    |v| (glam::Vec3::from(v.position) + s.origins[v.model as usize])
                        .distance(origin)
                        < radius
                )
                .count()
                > 300,
            "missing dynamic geometry at {origin}"
        );
    }
}

#[test]
fn real_dropper_cover_and_elevator_bind_geometry_decodes() {
    let fs = Assets::from_game("../portal-original/portal", None).unwrap();
    for stem in [
        "models/props/box_dropper_cover",
        "models/props/round_elevator_body",
        "models/props/round_elevator_doors",
    ] {
        let m = fs.read(&format!("{stem}.mdl")).unwrap();
        eprintln!(
            "{stem} MDL version {} length {} actual {}",
            i32::from_le_bytes(m[4..8].try_into().unwrap()),
            i32::from_le_bytes(m[76..80].try_into().unwrap()),
            m.len()
        );
        let model = decode(
            &m,
            &fs.read(&format!("{stem}.vvd")).unwrap(),
            &fs.read(&format!("{stem}.dx90.vtx")).unwrap(),
        )
        .unwrap();
        assert!(!model.vertices.is_empty());
        assert!(model.vertices.iter().all(|v| v.position.is_finite()));
    }
}
