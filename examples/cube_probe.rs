use glam::Vec3;
use portal_bsp_viewer::{
    assets::Assets,
    entity::{parse_entities, Runtime},
    play::{Player, World},
    scene::Scene,
};
fn main() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let es = parse_entities(portal_bsp_viewer::lump(&b, 0, 1).unwrap()).unwrap();
    let base = portal_bsp_viewer::lump(&b, 14, 48).unwrap().len() / 48;
    let n = es
        .iter()
        .filter(|e| e.get("classname") == Some("prop_physics"))
        .position(|e| e.get("targetname") == Some("box"))
        .unwrap();
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let s = Scene::load(&b, &fs).unwrap();
    for path in [
        "models/props/box_dropper_cover.mdl",
        "models/props/metal_box.mdl",
    ] {
        let mdl = fs.read(path).unwrap();
        let i = |o: usize| i32::from_le_bytes(mdl[o..o + 4].try_into().unwrap());
        let string = |o: usize| {
            String::from_utf8_lossy(&mdl[o..mdl[o..].iter().position(|b| *b == 0).unwrap() + o])
                .into_owned()
        };
        let count = i(188) as usize;
        let start = i(192) as usize;
        assert!(count < 4096);
        for n in 0..count {
            let p = start + n * 212;
            let name = string(p + i(p + 4) as usize);
            let events = i(p + 24);
            println!("MDL {path} sequence {name} event_count {events}");
            assert!((0..4096).contains(&events));
            for e in 0..events as usize {
                let q = p + i(p + 28) as usize + e * 80;
                println!(
                    "  event cycle {} id {} type {} options {}",
                    f32::from_le_bytes(mdl[q..q + 4].try_into().unwrap()),
                    i(q + 4),
                    i(q + 8),
                    string(q + 12)
                );
            }
        }
    }
    r.bind_animations(&s);
    w.bind_scene(&s);
    w.update_models(&r.origins, &r.visible);
    let bounds = s
        .vertices
        .iter()
        .filter(|v| v.model as usize == base + n)
        .fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(lo, hi), v| {
                let p = Vec3::from(v.position);
                (lo.min(p), hi.max(p))
            },
        );
    println!("cube mesh bounds {:?}", bounds);
    let mut p = Player::new(Vec3::new(-740., -1050., 136.), 0.);
    r.input("logic_drop_box", "Trigger", "");
    for i in 0..1200 {
        r.tick(1. / 120., &mut p, &mut w);
        if i % 120 == 0 {
            println!(
                "{} cube {:?} visible {} active {}",
                i,
                r.origins[base + n],
                r.visible[base + n],
                r.visible.iter().filter(|v| **v).count()
            );
        }
    }
    println!(
        "physical {:?}",
        es.iter()
            .filter(|e| e.get("classname") == Some("prop_physics"))
            .enumerate()
            .map(|(i, e)| (base + i, e.get("targetname")))
            .collect::<Vec<_>>()
    );
    for i in base..r.origins.len() {
        if r.visible[i] {
            println!("slot {i} {:?}", r.origins[i]);
        }
    }
    println!("unknown {:?}", r.unknown_inputs);
}
