use glam::Vec3;
use opensource::{
    entity::Runtime,
    play::{Player, World},
};

// Enter the actual authored brush; never force a case output or portal input.
#[test]
fn second_room_authored_blue_cycle() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let mut r = Runtime::new(&b).unwrap();
    r.portal_events = Some(vec![]);
    let mut w = World::parse(&b).unwrap();
    let mut p = Player::new(Vec3::new(-1088., -512., 592.), 0.);
    for _ in 0..2040 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    let events = r.portal_events.as_ref().unwrap();
    assert!(
        events
            .iter()
            .any(|(name, active)| name == "portal_red_1" && *active),
        "entry trigger must actually fire: {events:?}"
    );
    let blue: Vec<_> = events
        .iter()
        .filter(|(n, active)| {
            ["portal_blue_1", "portal_blue_2", "portal_blue_3"].contains(&n.as_str()) && *active
        })
        .map(|(n, _)| n.as_str())
        .collect();
    assert_eq!(
        blue,
        [
            "portal_blue_1",
            "portal_blue_2",
            "portal_blue_3",
            "portal_blue_1"
        ]
    );
    for unsupported in [
        "logic_case:InValue",
        "math_counter:Add",
        "math_counter:SetValue",
    ] {
        assert!(
            !r.unknown_inputs.contains(unsupported),
            "{:?}",
            r.unknown_inputs
        );
    }
}
