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
fn chamber06_blue_shot_hits_actual_parented_white_panel() {
    use opensource::{
        assets::Assets,
        portal::{Game, Surfaces},
    };
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_06.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let (mut r, mut w, _) = setup();
    r.direct_launch_loadout();
    assert_eq!(r.portal_inventory, [true, false]);
    let (origins, rotations) = r.model_transforms();
    w.update_transforms(&origins, &r.visible, &rotations);
    // Model28's concrete side is at local x160, y64..192, z-64..64.
    let target = origins[28] + rotations[28] * Vec3::new(160., 128., 0.);
    let eye = target + Vec3::X * 100.;
    let (fraction, _) = w.trace_point(eye, target);
    assert!(
        fraction > 0.999,
        "panel hit must agree with posed BSP collision: {fraction}"
    );
    let mut game = Game::load(&b).unwrap();
    game.automatic_surfaces = Some(Surfaces::load(&b, &fs).unwrap());
    let placed = game
        .fire_player(0, eye, -Vec3::X, &r, &mut w)
        .expect("authored white brush panel must accept blue portals");
    assert!(
        placed.center.distance(target) < 0.1,
        "must hit panel, not world behind it: {:?} vs {target:?}",
        placed.center
    );
    assert!(game.fire_player(1, eye, -Vec3::X, &r, &mut w).is_err());
}

#[test]
fn chamber06_travelling_blue_shot_uses_the_moved_panel_pose() {
    use opensource::{
        assets::Assets,
        portal::{Game, Surfaces},
    };
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_06.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let (mut r, mut w, mut p) = setup();
    r.direct_launch_loadout();
    p.position = Vec3::new(-1800., 128., 48.);
    let initial = r.model_transforms().0[33];
    r.input("fling_pillar2_door_bottom_bottom_a2", "Close", "");
    step(&mut r, &mut w, &mut p, 500);
    let target = r.model_transforms().0[33] + Vec3::X * 160.;
    assert!(r.model_transforms().0[33].x > initial.x + 180.);
    assert!(w.model_origins[33].distance(r.model_transforms().0[33]) < 0.001);
    let eye = target + Vec3::X * 100.;
    let mut game = Game::load(&b).unwrap();
    game.automatic_surfaces = Some(Surfaces::load(&b, &fs).unwrap());
    game.launch_player(0, eye, -Vec3::X, -Vec3::X, eye - Vec3::X * 24., &r, &w)
        .unwrap();
    assert!(
        game.portals[0].is_none(),
        "placement must wait for projectile arrival"
    );
    assert!(game.shots[0].target.distance(target) < 0.1);
    game.advance_shots(1., &r, &mut w);
    assert!(
        game.portals[0]
            .expect("blue bolt must place on moved white panel")
            .center
            .distance(target)
            < 0.1
    );
    assert!(w.portals[0].unwrap().center.distance(target) < 0.1);
    assert!(game.portals[1].is_none());
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
