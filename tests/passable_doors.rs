#[test]
fn actual_passable_pedestal_and_border_doors_do_not_obstruct_shots_or_player() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_01.bsp").unwrap();
    let w = opensource::play::World::parse(&b).unwrap();
    assert!(
        !w.model_solid[18],
        "rotating carrier flags12 includes passable8"
    );
    assert!(
        !w.model_solid[20],
        "lower borders flags12 includes passable8"
    );
}
