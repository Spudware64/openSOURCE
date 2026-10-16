use glam::Vec3;
use portal_bsp_viewer::{
    entity::{parse_entities, Runtime},
    play::{Plane, Player, World},
};
#[test]
fn parented_platform_carries_its_actual_rider() {
    let es = parse_entities(br#"{"classname" "func_door" "targetname" "carrier" "model" "*1" "movedir" "-1 0 0" "speed" "12" "lip" "0" "wait" "-1"}
    {"classname" "func_brush" "targetname" "floor" "parentname" "carrier" "model" "*2"}"#).unwrap();
    let mut r = Runtime::from_entities(
        es,
        vec![
            (Vec3::ZERO, Vec3::ZERO),
            (Vec3::ZERO, Vec3::new(64., 64., 12.)),
            (Vec3::ZERO, Vec3::new(64., 64., 4.)),
        ],
    )
    .unwrap();
    let mut w = World::from_brushes(vec![]);
    w.model_brushes = vec![
        vec![],
        vec![],
        vec![vec![
            Plane {
                normal: Vec3::Z,
                distance: 0.,
            },
            Plane {
                normal: -Vec3::Z,
                distance: 4.,
            },
            Plane {
                normal: Vec3::X,
                distance: 64.,
            },
            Plane {
                normal: -Vec3::X,
                distance: 64.,
            },
            Plane {
                normal: Vec3::Y,
                distance: 64.,
            },
            Plane {
                normal: -Vec3::Y,
                distance: 64.,
            },
        ]],
    ];
    w.model_solid = vec![false, false, true];
    let (o, a) = r.model_transforms();
    w.update_transforms(&o, &r.visible, &a);
    let mut p = Player::new(Vec3::Z * 0.03125, 0.);
    p.grounded = true;
    assert_eq!(w.support_model(p.position, p.height()), Some(2));
    r.input("carrier", "Open", "");
    for _ in 0..60 {
        r.tick(1. / 120., &mut p, &mut w);
        p.step(&w, Vec3::ZERO, false, 1. / 120.);
    }
    let z = r.model_transforms().0[2].z;
    assert!(z > 5., "parent must move {z}");
    assert!(
        (p.position.z - z - 0.03125).abs() < 0.01,
        "rider {:?}, resolved floor {z}, raw {:?}",
        p.position,
        r.origins[2]
    );
    assert!(p.grounded);
}
