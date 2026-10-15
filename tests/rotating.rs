use glam::Vec3;
use portal_bsp_viewer::{
    entity::{parse_entities, Runtime},
    play::{Plane, Player, World},
};
#[test]
fn rotating_brush_collision_and_parented_aim_follow_angle() {
    let es=parse_entities(br#"{"classname" "func_door_rotating" "targetname" "hinge" "model" "*1" "distance" "90" "speed" "90" "wait" "-1" "origin" "0 0 0"}
 {"classname" "info_target" "targetname" "child" "parentname" "hinge" "origin" "10 0 0"}"#).unwrap();
    let mut r = Runtime::from_entities(
        es,
        vec![
            (Vec3::ZERO, Vec3::ZERO),
            (Vec3::ZERO, Vec3::new(20., 2., 72.)),
        ],
    )
    .unwrap();
    let mut w = World::from_brushes(vec![]);
    w.model_brushes = vec![
        vec![],
        vec![vec![Plane {
            normal: Vec3::X,
            distance: 20.,
        }]],
    ];
    w.model_solid = vec![false, true];
    let mut p = Player::new(Vec3::splat(500.), 0.);
    r.input("hinge", "Open", "");
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    let child = r.entity_transform("child").unwrap();
    assert!(child.w_axis.truncate().distance(Vec3::new(0., 10., 0.)) < 0.01);
    assert!(
        w.brushes[0][0].normal.distance(Vec3::Y) < 0.01,
        "{:?}",
        w.brushes
    );
    r.input("child", "ClearParent", "");
    r.tick(1. / 120., &mut p, &mut w);
    r.input("hinge", "Close", "");
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    assert!(
        r.entity_transform("child")
            .unwrap()
            .w_axis
            .truncate()
            .distance(Vec3::new(0., 10., 0.))
            < 0.01
    );
}
