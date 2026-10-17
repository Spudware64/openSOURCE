use portal_bsp_viewer::{assets::Assets, scene::Scene};
#[test]
fn original_energy_ball_has_authored_mesh_and_materials() {
    let fs = Assets::from_game("../portal-original/portal", None).unwrap();
    let (vertices, textures) = Scene::energy_ball(&fs).unwrap();
    assert_eq!(vertices.len(), 1152);
    assert_eq!(textures.len(), 4);
    let model = portal_bsp_viewer::studio::decode(&fs.read("models/effects/combineball.mdl").unwrap(), &fs.read("models/effects/combineball.vvd").unwrap(), &fs.read("models/effects/combineball.dx90.vtx").unwrap()).unwrap();
    for (actual, original) in vertices.iter().zip(&model.vertices) {
        assert_eq!(actual.position, original.position.to_array());
        assert_eq!(actual.uv, original.uv);
        assert_eq!(actual.layer as usize, model.skins[0][original.material]);
    }
    eprintln!("authored material candidates {:?}",model.materials);
    for layer in 0..4 { eprintln!("layer {layer} bounds radius {}", vertices.iter().filter(|v|v.layer==layer).map(|v|glam::Vec3::from(v.position).length()).fold(0f32,f32::max)); }
    if let Ok(dir) = std::env::var("ENERGY_ASSET_CAPTURE") {
        for (i,t) in textures.iter().enumerate() {
            let file = std::fs::File::create(format!("{dir}/material{i}.png")).unwrap();
            let mut png = png::Encoder::new(file,256,256);
            png.set_color(png::ColorType::Rgba);
            png.set_depth(png::BitDepth::Eight);
            png.write_header().unwrap().write_image_data(t).unwrap();
        }
    }
    assert_eq!(vertices.len() % 3, 0);
    assert!(!textures.is_empty());
    assert!(vertices.iter().all(|v| [-2.,-3.,-4.].contains(&v.unlit) && v.model == 0));
    assert!(textures.iter().any(|t| t.chunks_exact(4).any(|p| p[0] != p[1] || p[1] != p[2])));
    let extent = vertices.iter().map(|v| glam::Vec3::from(v.position).length()).fold(0f32, f32::max);
    eprintln!("original combineball: {} triangles, {} materials, extent {}", vertices.len()/3,textures.len(),extent);
    // Launcher production loading must retain original geometry and remap every material layer.
    let map = fs.read("maps/testchmb_a_04.bsp").unwrap();
    let scene = Scene::load_with_workers(&map, &fs, 1).unwrap();
    assert_eq!(scene.pellet_template.len(), vertices.len());
    for (cached, original) in scene.pellet_template.iter().zip(&vertices) {
        assert_eq!(cached.position, original.position);
        assert_eq!(cached.uv, original.uv);
        assert_eq!(cached.layer as usize, scene.textures.len()-4+original.layer as usize);
    }
    assert!(!scene.warnings.iter().any(|w|w.contains("original energy ball unavailable")));
    assert!(extent > 0. && extent < 100.);
}
