use portal_bsp_viewer::{animation::Skeleton, assets::Assets, studio};
#[test]
fn actual_portal_open_sequences_skin_vertices_not_bind_pose() {
    let fs = Assets::from_game("../portal-original/portal", None).unwrap();
    for name in ["box_dropper_cover", "round_elevator_doors"] {
        let stem = format!("models/props/{name}");
        let mdl = fs.read(&format!("{stem}.mdl")).unwrap();
        let model = studio::decode(
            &mdl,
            &fs.read(&format!("{stem}.vvd")).unwrap(),
            &fs.read(&format!("{stem}.dx90.vtx")).unwrap(),
        )
        .unwrap();
        let sk = Skeleton::decode(&mdl, None).unwrap();
        let a = sk.skin(&model.vertices, "open", 0.).unwrap();
        let b = sk.skin(&model.vertices, "open", 19. / 30.).unwrap();
        let changed = a
            .iter()
            .zip(&b)
            .filter(|(a, b)| a.position.distance(b.position) > 1.)
            .count();
        assert!(changed > 100, "{name}: only {changed} changed vertices");
        assert!(sk.skin(&model.vertices, "does_not_exist", 0.).is_err());
        assert!(sk.skin(&model.vertices, "open", f32::NAN).is_err());
        assert!(b
            .iter()
            .all(|v| v.position.is_finite() && (v.normal.length() - 1.).abs() < 0.001));
        let end = sk.skin(&model.vertices, "open", 100.).unwrap();
        assert!(end
            .iter()
            .zip(&b)
            .all(|(a, b)| a.position.distance(b.position) < 0.001));
    }
}
