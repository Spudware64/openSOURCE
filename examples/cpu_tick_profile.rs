//! Bounded CPU-only Chamber 00 profiling; no GPU or invented frame rate.
use glam::Vec3;
use portal_bsp_viewer::{assets::Assets, entity::Runtime, play::{Player, World}, scene::Scene};
fn main() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let scene = Scene::load(&b, &fs).unwrap();
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    r.bind_animations(&scene); w.bind_scene(&scene); w.update_models(&r.origins, &r.visible);
    let mut p = Player::new(Vec3::new(-740., -1056., 128.1), std::f32::consts::PI);
    p.pitch = -0.45;
    r.input("logic_drop_box", "Trigger", "");
    for _ in 0..480 { r.tick(1./120., &mut p, &mut w); }
    for phase in ["free", "held"] {
        if phase == "held" { assert!(r.use_player(&p, &w), "must profile a genuinely held cube"); }
        let t = std::time::Instant::now();
        for _ in 0..120 { r.tick(1./120., &mut p, &mut w); }
        println!("{phase}: 120 entity ticks {:?}", t.elapsed());
        let t = std::time::Instant::now();
        for _ in 0..120 { p.step(&w, Vec3::ZERO, false, 1./120.); }
        println!("{phase}: 120 controller steps {:?}", t.elapsed());
    }
}
