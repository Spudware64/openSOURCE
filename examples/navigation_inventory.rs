use glam::Vec3;
use portal_bsp_viewer::play::World;
fn main() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_01.bsp").unwrap();
    let w = World::parse(&b).unwrap();
    for y in (256..=544).step_by(16) {
        let x = 96.;
        let mut tops = vec![];
        for br in &w.brushes {
            let mut lo = -10000f32;
            let mut hi = 10000f32;
            for p in br {
                let d = p.distance - p.normal.x * x - p.normal.y * y as f32;
                if p.normal.z > 0.001 {
                    hi = hi.min(d / p.normal.z);
                } else if p.normal.z < -0.001 {
                    lo = lo.max(d / p.normal.z);
                } else if d < 0. {
                    lo = 10001.;
                }
            }
            if lo < hi && hi < 100. && hi > -300. {
                tops.push(hi);
            }
        }
        println!("ground x96 y{y}: {tops:?}");
    }
    for z in [0.03125, -63.96875, -127.96875, -143.96875] {
        println!("z {z}; columns x=-448..640 step32");
        for y in (-128i16..=640).step_by(32).rev() {
            let line: String = (-448..=640)
                .step_by(32)
                .map(|x| {
                    if w.overlaps(Vec3::new(x as f32, y as f32, z), 72.) {
                        '#'
                    } else {
                        '.'
                    }
                })
                .collect();
            println!("{y:4} {line}");
        }
    }
}
