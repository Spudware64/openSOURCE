use opensource::{assets::Assets, props::static_props};
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn main() {
    let name = std::env::args().nth(1).unwrap_or("testchmb_a_00".into());
    let b = std::fs::read(format!("../portal-original/portal/maps/{name}.bsp")).unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let p = static_props(&b).unwrap();
    for path in p.models {
        let mdl = fs.read(&path).unwrap();
        let stem = path.trim_end_matches(".mdl");
        let vvd = fs.read(&format!("{stem}.vvd")).unwrap();
        let vtx = fs.read(&format!("{stem}.dx90.vtx")).unwrap();
        println!("{path}: MDL v{} checksum {} flags {:x} textures {} dirs {} skins {}x{} bodyparts {}; VVD v{} LODs {} verts {} fixups {}; VTX v{} checksum {} bodies {}",i(&mdl,4),i(&mdl,8),i(&mdl,152),i(&mdl,204),i(&mdl,212),i(&mdl,220),i(&mdl,224),i(&mdl,232),i(&vvd,4),i(&vvd,12),i(&vvd,16),i(&vvd,48),i(&vtx,0),i(&vtx,16),i(&vtx,28));
    }
}
