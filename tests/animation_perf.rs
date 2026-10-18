use opensource::{animation::{Skeleton, SkinMesh}, assets::Assets, studio};
#[test]
fn indexed_skin_matches_expanded_actual_models() {
    let fs = Assets::from_game("../portal-original/portal", None).unwrap();
    for name in ["box_dropper_cover", "round_elevator_doors"] {
        let stem = format!("models/props/{name}");
        let mdl = fs.read(&format!("{stem}.mdl")).unwrap();
        let model = studio::decode(&mdl, &fs.read(&format!("{stem}.vvd")).unwrap(), &fs.read(&format!("{stem}.dx90.vtx")).unwrap()).unwrap();
        let sk = Skeleton::decode(&mdl, None).unwrap();
        let mesh = SkinMesh::new(&model.vertices);
        assert!(mesh.vertices.len() < model.vertices.len()/2);
        for time in [0., 0.123, 0.4, 100.] {
            let expected = sk.skin(&model.vertices, "open", time).unwrap();
            let actual = sk.skin(&mesh.vertices, "open", time).unwrap();
            for (v, &i) in expected.iter().zip(&mesh.indices) {
                assert_eq!(v.position, actual[i].position);
                assert_eq!(v.normal, actual[i].normal);
            }
        }
        let start = std::time::Instant::now();
        for i in 0..200 { std::hint::black_box(sk.skin(&model.vertices, "open", i as f32/300.).unwrap()); }
        let expanded = start.elapsed();
        let start = std::time::Instant::now();
        for i in 0..200 { std::hint::black_box(sk.skin(&mesh.vertices, "open", i as f32/300.).unwrap()); }
        eprintln!("{name}: 200 poses expanded={} unique={} before={expanded:?} indexed={:?}", model.vertices.len(), mesh.vertices.len(), start.elapsed());
    }
}
