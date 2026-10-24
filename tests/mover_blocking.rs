use glam::Vec3;
use opensource::{
    entity::{parse_entities, Runtime},
    play::{Plane, Player, World},
};

#[test]
fn preexisting_overlap_only_blocks_movers_that_increase_penetration() {
    for (start, yaw, blocked, dt) in [
        (100., 0., false, 0.1),
        (20., 180., true, 0.1),
        (20., 0., false, 0.1),
        (26., 180., true, 0.1),
        (20., 180., true, 0.00001),
    ] {
        let fixture = r#"
{"classname" "func_door" "targetname" "door" "model" "*2" "origin" "$X 0 0"
 "movedir" "0 $YAW 0" "speed" "10" "wait" "-1" "OnBlocked" "result,SetActivatedState,0,0,-1"}
{"classname" "func_brush" "targetname" "spare" "model" "*1" "origin" "1000 0 0"}
{"classname" "prop_portal" "targetname" "result"}
"#
        .replace("$X", &start.to_string())
        .replace("$YAW", &yaw.to_string());
        let lo = Vec3::new(-10., -50., -50.);
        let hi = Vec3::new(10., 50., 150.);
        let mut runtime = Runtime::from_entities(
            parse_entities(fixture.as_bytes()).unwrap(),
            vec![(Vec3::ZERO, Vec3::ZERO), (lo, hi), (lo, hi)],
        )
        .unwrap();
        runtime.portal_events = Some(vec![]);
        let mut world = World::from_brushes(vec![vec![Plane {
            normal: Vec3::Z,
            distance: 0.,
        }]]);
        let brush: Vec<Plane> = [Vec3::X, Vec3::Y, Vec3::Z]
            .into_iter()
            .enumerate()
            .flat_map(|(axis, normal)| {
                [
                    Plane {
                        normal,
                        distance: hi[axis],
                    },
                    Plane {
                        normal: -normal,
                        distance: -lo[axis],
                    },
                ]
            })
            .collect();
        world.model_brushes.push(vec![brush.clone()]);
        world.model_brushes.push(vec![brush]);
        world.model_solid.extend([true, true]);
        let (origins, rotations) = runtime.model_transforms();
        world.update_transforms(&origins, &runtime.visible, &rotations);
        let mut player = Player::new(Vec3::new(0., 0., -1.), 0.);
        assert!(world.overlaps(player.position, player.height()));
        // Removing an earlier model must preserve the moving brush's identity.
        runtime.input("spare", "Disable", "");
        runtime.input("door", "Open", "");
        runtime.tick(dt, &mut player, &mut world);
        if blocked {
            assert_eq!(runtime.origins[2].x, start);
            runtime.tick(dt, &mut player, &mut world);
            assert_eq!(
                runtime.portal_events.unwrap(),
                vec![("result".into(), false)]
            );
        } else {
            assert!((runtime.origins[2].x - start - 10. * dt).abs() < 0.001);
            assert!(runtime.portal_events.unwrap().is_empty());
        }
    }
}
