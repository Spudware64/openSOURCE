use opensource::scenes::{durations, scene_crc};
#[test]
fn scene_image_validates_layout_ranges_and_actual_timing() {
    let fs =
        opensource::assets::Assets::from_game("../portal-original/portal", None).unwrap();
    let b = fs.read("scenes/scenes.image").unwrap();
    let d = durations(&b).unwrap();
    assert_eq!(d.len(), 1970);
    assert_eq!(
        scene_crc("SCENES/test_chamber/ai_01_a_entry.vcd"),
        0x50f9d3d1
    );
    assert_eq!(d[&0x50f9d3d1], 21.039);
    for len in [0, 4, 19, 100, 1000, b.len() / 2] {
        assert!(durations(&b[..len]).is_err(), "truncated {len}");
    }
    let mut bad = b.clone();
    bad[4..8].copy_from_slice(&3u32.to_le_bytes());
    assert!(durations(&bad).is_err());
    let mut bad = b;
    bad[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(durations(&bad).is_err());
}
#[test]
fn parent_cycles_are_rejected_without_mutating_original_entities() {
    let es=opensource::entity::parse_entities(br#"{"classname" "info_target" "targetname" "a" "parentname" "b"}{"classname" "info_target" "targetname" "b" "parentname" "a"}"#).unwrap();
    assert!(opensource::entity::Runtime::from_entities(es, vec![]).is_err());
}
