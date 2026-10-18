use opensource::assets::Assets;
fn main() {
    let fs = Assets::from_game("../portal-original/portal", None).unwrap();
    for stem in [
        "box_dropper_cover",
        "round_elevator_body",
        "round_elevator_doors",
        "bed_cover_reference",
    ] {
        let b = fs.read(&format!("models/props/{stem}.mdl")).unwrap();
        let i = |o| i32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let s = |o: usize| {
            String::from_utf8_lossy(&b[o..o + b[o..].iter().position(|c| *c == 0).unwrap()])
                .to_string()
        };
        println!("{stem}: bones={} anims={} seqs={}", i(156), i(180), i(188));
        for j in 0..i(180) as usize {
            let p = i(184) as usize + j * 100;
            println!(" anim {j} {} fps={} flags={:x} frames={} block={} index={} sections={} sectionframes={}",s(p+i(p+4) as usize),f32::from_le_bytes(b[p+8..p+12].try_into().unwrap()),i(p+12),i(p+16),i(p+52),i(p+56),i(p+80),i(p+84));
        }
        for j in 0..i(188) as usize {
            let p = i(192) as usize + j * 212;
            println!(
                " seq {j} {} flags={:x} blends={} groups={}/{} anim={} layers={}",
                s(p + i(p + 4) as usize),
                i(p + 12),
                i(p + 56),
                i(p + 68),
                i(p + 72),
                i16::from_le_bytes(
                    b[p + i(p + 60) as usize..p + i(p + 60) as usize + 2]
                        .try_into()
                        .unwrap()
                ),
                i(p + 148)
            );
        }
    }
}
