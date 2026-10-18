use glam::Vec3;
use opensource::{assets::Assets, portal::Surfaces};
fn main() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let s = Surfaces::load(&b, &fs).unwrap();
    let mut seen = std::collections::HashSet::new();
    for face in &s.faces {
        if !face.allowed {
            continue;
        }
        let c = face.points.iter().copied().sum::<Vec3>() / face.points.len() as f32;
        if let Ok(p) = s.place(c + face.normal * 8., -face.normal) {
            if seen.insert(format!("{:?}", p.center)) {
                println!("{} {p:?}", face.material);
            }
        }
    }
}
