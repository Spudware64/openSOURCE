use opensource::entity::*;
#[test]
fn real_entities_preserve_repeated_outputs_and_local_brush_origins() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let e = parse_entities(opensource::lump(&b, 0, 1).unwrap()).unwrap();
    let relay = e
        .iter()
        .find(|e| e.get("targetname") == Some("e3_start"))
        .unwrap();
    assert!(relay.outputs("OnTrigger").len() > 4);
    let door = e
        .iter()
        .find(|e| e.get("targetname") == Some("door_right_a00"))
        .unwrap();
    assert_eq!(door.model().unwrap(), Some(14));
    assert_eq!(door.origin().unwrap(), glam::Vec3::new(-484., -720., 184.));
}
