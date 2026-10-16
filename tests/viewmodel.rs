use portal_bsp_viewer::{assets::Assets, scene::Scene};
#[test]
fn original_weapon_idle_is_camera_local_and_textured() {
    let fs = Assets::from_game("../portal-original/portal", None).unwrap();
    let scene = Scene::weapon_viewmodel(&fs, "weapon_portalgun").unwrap();
    assert!(scene.vertices.len() > 1000);
    assert!(!scene.textures.is_empty());
    let mut lo = glam::Vec3::splat(f32::INFINITY);
    let mut hi = -lo;
    for v in &scene.vertices { let p = glam::Vec3::from(v.position); lo=lo.min(p); hi=hi.max(p); assert_eq!(v.model,0); }
    println!("authored idle bounds {lo:?} {hi:?}");
    assert!(hi.x > 10. && lo.x > -30.);
}
#[test]
fn direct_launch_consumes_only_authored_spawn_loadout() {
 let fs=Assets::from_game("../portal-original/portal",None).unwrap();
 for (map, owned) in [("testchmb_a_00",false),("testchmb_a_01",false),("testchmb_a_03",true)] {
 let b=fs.read(&format!("maps/{map}.bsp")).unwrap(); let mut r=portal_bsp_viewer::entity::Runtime::new(&b).unwrap();
 r.direct_launch_loadout(); assert_eq!(r.portal_inventory.iter().any(|v|*v),owned,"{map}");
 if owned {assert_eq!(r.portal_inventory,[true,false]); let slot=r.entity_model("portalgun_spawn_lvl4").unwrap();assert!(!r.visible[slot]);}
 }
}
