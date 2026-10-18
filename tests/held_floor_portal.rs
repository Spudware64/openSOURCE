use glam::Vec3;
use opensource::{
    entity::{parse_entities, Runtime},
    play::{Plane, Player, World},
    portal::Portal,
};

#[test]
fn actual_chamber00_floor_portal_keeps_held_cube_above_backing_floor() {
    use opensource::{
        assets::Assets,
        portal::{Game, Surfaces},
        scene::Scene,
    };
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let scene = Scene::load(&b, &fs).unwrap();
    let mut r = Runtime::new(&b).unwrap();
    let mut w = World::parse(&b).unwrap();
    r.bind_animations(&scene);
    w.bind_scene(&scene);
    w.update_models(&r.origins, &r.visible);
    r.portal_events = Some(vec![]);
    let mut g = Game::load(&b).unwrap();
    g.automatic_surfaces = Some(Surfaces::load(&b, &fs).unwrap());
    let mut p = Player::spawn(&w);
    for _ in 0..8100 {
        r.tick(1. / 120., &mut p, &mut w);
        g.sync_runtime(&mut r, &mut w);
    }
    let [Some(_), Some(dst)] = w.portals else {
        panic!("authored pair")
    };
    r.input("logic_drop_box", "Trigger", "");
    for _ in 0..480 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    let cube = r
        .origins
        .iter()
        .enumerate()
        .filter(|(i, _)| *i >= 84 && r.visible[*i])
        .min_by(|(_, a), (_, b)| {
            a.distance(Vec3::new(-800., -1056., 160.))
                .total_cmp(&b.distance(Vec3::new(-800., -1056., 160.)))
        })
        .unwrap()
        .0;
    let top = Vec3::new(-800., -1056., 200.);
    let end = top - Vec3::Z * 300.;
    let (f, n) = w.trace_point(top, end);
    assert!(f < 1. && n.z > 0.7);
    let floor = top.lerp(end, f);
    // Staged approach on actual room floor and actual maker-spawned posed model.
    for degrees in [0f32, 45., 90.] {
        let angle = degrees.to_radians();
        let src = Portal::new(floor, Vec3::Z, Vec3::new(angle.cos(), angle.sin(), 0.));
        w.portals = [Some(src), Some(dst)];
        r.origins[cube] = floor + Vec3::Z * 8.;
        p.position = floor - Vec3::X * 40. - Vec3::Z * 56.;
        p.yaw = 0.;
        p.pitch = 0.;
        assert!(r.use_player(&p, &w));
        p.position = floor + src.right() * 100. - Vec3::X * 64. - Vec3::Z * 56.;
        for tick in 0..120 {
            r.tick(1. / 120., &mut p, &mut w);
            assert!(
                (r.origins[cube] - floor).dot(src.right()).abs() < 30.,
                "roll {degrees} tick {tick} {:?}",
                r.origins[cube]
            );
        }
        assert!(r.use_player(&p, &w));
        r.tick(1. / 120., &mut p, &mut w);
        assert!(
            r.rapier_collider_center(cube)
                .unwrap()
                .distance(r.origins[cube])
                < 0.01
        );
    }
}

#[test]
fn floor_to_wall_carry_survives_both_transfer_orders() {
    for player_first in [false, true] {
        for degrees in [0f32, 45., 90.] {
            let z = if player_first { 168. } else { 40. };
            let es = parse_entities(
                format!("{{\"classname\" \"prop_physics\" \"origin\" \"0 0 {z}\"}}").as_bytes(),
            )
            .unwrap();
            let mut r = Runtime::from_entities(es, vec![(Vec3::ZERO, Vec3::ZERO)]).unwrap();
            let mut w = World::from_brushes(vec![vec![
                Plane {
                    normal: Vec3::Z,
                    distance: 0.,
                },
                Plane {
                    normal: -Vec3::Z,
                    distance: 32.,
                },
                Plane {
                    normal: Vec3::X,
                    distance: 500.,
                },
                Plane {
                    normal: -Vec3::X,
                    distance: 500.,
                },
                Plane {
                    normal: Vec3::Y,
                    distance: 500.,
                },
                Plane {
                    normal: -Vec3::Y,
                    distance: 500.,
                },
            ]]);
            let a = degrees.to_radians();
            let src = Portal::new(Vec3::ZERO, Vec3::Z, Vec3::new(a.cos(), a.sin(), 0.));
            let dst = Portal::new(Vec3::new(300., 0., 100.), -Vec3::X, Vec3::Z);
            w.portals = [Some(src), Some(dst)];
            let mut p = Player::new(Vec3::Z * 40., 0.);
            p.pitch = if player_first {
                std::f32::consts::FRAC_PI_2
            } else {
                -std::f32::consts::FRAC_PI_2
            };
            assert!(r.use_player(&p, &w));
            let mut first = None;
            for _ in 0..240 {
                p.step(&w, Vec3::ZERO, false, 1. / 120.);
                r.tick(1. / 120., &mut p, &mut w);
                if first.is_none() {
                    if p.portal_crossings > 0 {
                        first = Some(true);
                    } else if r.origins[1].x > 200. {
                        first = Some(false);
                    }
                }
                if p.portal_crossings > 0 {
                    break;
                }
            }
            assert_eq!(first, Some(player_first), "roll {degrees}");
            assert_eq!(p.portal_crossings, 1);
            // Ordinary retreat after player-first traversal retrieves the remote cube.
            for _ in 0..70 {
                p.step(
                    &w,
                    if player_first { -Vec3::X } else { Vec3::X },
                    false,
                    1. / 120.,
                );
                r.tick(1. / 120., &mut p, &mut w);
            }
            for tick in 0..120 {
                r.tick(1. / 120., &mut p, &mut w);
                let target = p.position + Vec3::Z * p.eye_height() + p.forward() * 64.;
                assert!(
                    r.origins[1].distance(target) < 2.,
                    "order {player_first} roll {degrees} tick {tick}: cube {:?} target {target:?}",
                    r.origins[1]
                );
            }
            assert!(r.use_player(&p, &w));
            r.tick(1. / 120., &mut p, &mut w);
            assert!(r.rapier_collider_center(1).unwrap().distance(r.origins[1]) < 0.01);
        }
    }
}

#[test]
fn partly_inserted_held_cube_cannot_slide_out_through_floor_rim() {
    for degrees in [0f32, 45., 90.] {
        let es = parse_entities(br#"{"classname" "prop_physics" "origin" "0 0 8"}"#).unwrap();
        let mut r = Runtime::from_entities(es, vec![(Vec3::ZERO, Vec3::ZERO)]).unwrap();
        let mut w = World::from_brushes(vec![vec![
            Plane {
                normal: Vec3::Z,
                distance: 0.,
            },
            Plane {
                normal: -Vec3::Z,
                distance: 32.,
            },
            Plane {
                normal: Vec3::X,
                distance: 500.,
            },
            Plane {
                normal: -Vec3::X,
                distance: 500.,
            },
            Plane {
                normal: Vec3::Y,
                distance: 500.,
            },
            Plane {
                normal: -Vec3::Y,
                distance: 500.,
            },
        ]]);
        let angle = degrees.to_radians();
        let src = Portal::new(Vec3::ZERO, Vec3::Z, Vec3::new(angle.cos(), angle.sin(), 0.));
        let dst = Portal::new(Vec3::new(300., 0., 100.), -Vec3::X, Vec3::Z);
        w.portals = [Some(src), Some(dst)];
        let mut p = Player::new(Vec3::new(-40., 0., -56.), 0.);
        assert!(r.use_player(&p, &w));
        // Centre is above the opening but the lower half is inside its backing floor.
        // Move the grab laterally: restoring a whole convex floor has no entry hit.
        p.position = src.right() * 100. - Vec3::X * 64. - Vec3::Z * 56.;
        for tick in 0..120 {
            r.tick(1. / 120., &mut p, &mut w);
            let center = r.origins[1];
            assert!(
                center.dot(src.right()).abs() < 30.,
                "roll {degrees} tick {tick}: escaped rim into floor {center:?}"
            );
        }
        assert!(r.use_player(&p, &w));
        r.tick(1. / 120., &mut p, &mut w);
        assert!(r.rapier_collider_center(1).unwrap().distance(r.origins[1]) < 0.01);
    }
}
