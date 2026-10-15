use portal_bsp_viewer::{assets::Assets, entity::parse_entities, studio::decode};
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn main() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let o = i(&b, 8) as usize;
    let n = i(&b, 12) as usize;
    for e in parse_entities(&b[o..o + n]).unwrap() {
        if e.get("classname") == Some("prop_dynamic") {
            let path = e.get("model").unwrap();
            let m = fs.read(path).unwrap();
            let stem = path.trim_end_matches(".mdl");
            let v = fs.read(&format!("{stem}.vvd")).unwrap();
            let t = fs.read(&format!("{stem}.dx90.vtx")).unwrap();
            println!(
                "{} {} flags {:x} bones {} bodies {} decode {:?}",
                e.get("targetname").unwrap_or(""),
                path,
                i(&m, 152),
                i(&m, 156),
                i(&m, 232),
                decode(&m, &v, &t).map(|x| x.vertices.len() / 3)
            );
        }
    }
}
