use glam::Vec3;
use opensource::{
    entity::Runtime,
    play::{Player, World},
};

fn setup() -> (Runtime, World, Player) {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_06.bsp").unwrap();
    let r = Runtime::new(&b).unwrap();
    let w = World::parse(&b).unwrap();
    let mut p = Player::new(Vec3::new(704., 128., 88.), 0.);
    p.noclip = true;
    (r, w, p)
}
fn aim(p: &mut Player, target: Vec3) {
    let d = (target - (p.position + Vec3::Z * p.eye_height())).normalize();
    p.yaw = d.y.atan2(d.x);
    p.pitch = d.z.asin();
}
fn step(r: &mut Runtime, w: &mut World, p: &mut Player, n: usize) {
    for _ in 0..n {
        r.tick(1. / 120., p, w);
    }
}
#[test]
fn chamber06_start_open_door_completion_drives_delayed_next_stage() {
    let (mut r, mut w, mut p) = setup();
    p.position = Vec3::new(-1800., 128., 48.);
    let middle = "fling_pillar2_door_bottom_a2";
    let top = "fling_pillar2_door_top_a2";
    let start = r.origins[r.entity_model(top).unwrap()];
    r.input(middle, "Close", "");
    step(&mut r, &mut w, &mut p, 192); // 80 units at 50 units/s
    assert_eq!(r.origins[r.entity_model(top).unwrap()], start);
    step(&mut r, &mut w, &mut p, 48); // output delay is 0.5 s
    assert_eq!(r.origins[r.entity_model(top).unwrap()], start);
    step(&mut r, &mut w, &mut p, 36);
    let middle_closed = r.origins[r.entity_model(middle).unwrap()];
    step(&mut r, &mut w, &mut p, 900); // top completes its stroke and reopens middle
    assert!(
        r.origins[r.entity_model(middle).unwrap()].x < middle_closed.x - 1.,
        "top OnFullyClosed must reopen the middle stage"
    );
}

#[test]
fn chamber06_look_trigger_moves_pistons_only_after_continuous_gaze() {
    let (mut r, mut w, mut p) = setup();
    let name = "fling_pillar2_door_bottom_bottom_a2";
    let start = r.entity_transform(name).unwrap().w_axis.truncate();
    step(&mut r, &mut w, &mut p, 60);
    assert_eq!(r.entity_transform(name).unwrap().w_axis.truncate(), start);
    aim(&mut p, Vec3::new(136., 128., 525.946));
    step(&mut r, &mut w, &mut p, 12);
    assert_eq!(r.entity_transform(name).unwrap().w_axis.truncate(), start);
    // Looking away must reset, rather than accumulate, the required 0.2 s gaze.
    p.yaw = 0.;
    p.pitch = 0.;
    step(&mut r, &mut w, &mut p, 12);
    aim(&mut p, Vec3::new(136., 128., 525.946));
    step(&mut r, &mut w, &mut p, 12);
    assert_eq!(r.entity_transform(name).unwrap().w_axis.truncate(), start);
    step(&mut r, &mut w, &mut p, 24);
    let moving = r.entity_transform(name).unwrap().w_axis.truncate();
    assert!(
        moving.x > start.x + 1.,
        "look trigger must extend piston: {start:?} -> {moving:?}"
    );
    assert!(
        moving.x < 210.,
        "must move progressively, not teleport to endpoint"
    );
}
