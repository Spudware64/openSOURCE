use glam::Vec3;
use portal_bsp_viewer::play::World;
fn main() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_01.bsp").unwrap();
    let fs = portal_bsp_viewer::assets::Assets::from_game("../portal-original/portal", Some(&b))
        .unwrap();
    let surfaces = portal_bsp_viewer::portal::Surfaces::load(&b, &fs).unwrap();
    for f in surfaces.faces.iter().filter(|f| f.allowed) {
        let c = f.points.iter().copied().sum::<Vec3>() / f.points.len() as f32;
        if c.x < -1000. && c.z > 500. {
            println!("allowed {c:?} normal {:?}", f.normal);
        }
    }
    let w = World::parse(&b).unwrap();
    for y in (-64i16..=1152).step_by(64).rev() {
        let line: String = (-2048..=-192)
            .step_by(64)
            .map(|x| {
                let p = Vec3::new(x as f32, y as f32, 576.04);
                if w.overlaps(p, 72.) {
                    '#'
                } else {
                    let (t, n) = w.trace(p, p - Vec3::Z * 200.);
                    if t < 1. && n.z > 0.7 {
                        if t < 0.02 {
                            '='
                        } else {
                            'v'
                        }
                    } else {
                        '.'
                    }
                }
            })
            .collect();
        println!("{y:4} {line}");
    }
    println!("columns -2048..-192 step64; = floor576, v lower, # blocked");
}
