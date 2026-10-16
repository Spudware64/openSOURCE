use glam::Vec3;
use portal_bsp_viewer::{
    entity::Runtime,
    play::{Player, World},
};

// Characterization, not a portal fix: preserve the exact unsupported boundary
// so a future generic timer/counter/case implementation has an authored repro.
#[test]
fn second_room_authored_cycle_exposes_unsupported_case_input() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let mut r = Runtime::new(&b).unwrap();
    r.portal_events = Some(vec![]);
    let mut w = World::parse(&b).unwrap();
    let mut p = Player::new(Vec3::new(-1088., -512., 592.), 0.);
    for _ in 0..720 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    let events = r.portal_events.as_ref().unwrap();
    assert!(
        events
            .iter()
            .any(|(name, active)| name == "portal_red_1" && *active),
        "entry trigger must actually fire: {events:?}"
    );
    assert!(!events
        .iter()
        .any(|(name, active)| name == "portal_blue_1" && *active));
    assert!(
        r.unknown_inputs.contains("logic_case:InValue"),
        "{:?}",
        r.unknown_inputs
    );
}
