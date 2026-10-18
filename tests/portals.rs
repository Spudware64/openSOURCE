use glam::Vec3;
use opensource::portal::{Portal, Surfaces};
#[test]
fn actual_chamber_placement_fits_only_portalable_world_faces() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = opensource::assets::Assets::from_game("../portal-original/portal", Some(&b))
        .unwrap();
    let surfaces = Surfaces::load(&b, &fs).unwrap();
    let mut accepted = 0;
    let mut rejected = 0;
    for s in &surfaces.faces {
        let center = s.points.iter().copied().sum::<Vec3>() / s.points.len() as f32;
        let result = surfaces.place(center + s.normal * 8., -s.normal);
        if let Ok(p) = result {
            if p.center.distance(center) < 0.1
                && p.normal.dot(s.normal) > 0.999
                && !s.material.starts_with("tools/")
            {
                assert!(s.allowed, "{} {:?}", s.material, center);
                accepted += 1;
            }
        } else if !s.allowed {
            rejected += 1;
        }
    }
    assert!(accepted > 0);
    assert!(rejected > 0);
    println!("fitting faces {accepted}, rejected material faces {rejected}");
}
#[test]
fn traverses_wall_aperture_but_not_its_rim() {
    use opensource::play::{Plane, Player, World};
    let wall = vec![
        Plane {
            normal: Vec3::X,
            distance: 0.,
        },
        Plane {
            normal: -Vec3::X,
            distance: 10.,
        },
        Plane {
            normal: Vec3::Y,
            distance: 500.,
        },
        Plane {
            normal: -Vec3::Y,
            distance: 500.,
        },
        Plane {
            normal: Vec3::Z,
            distance: 500.,
        },
        Plane {
            normal: -Vec3::Z,
            distance: 500.,
        },
    ];
    let mut w = World::from_brushes(vec![wall]);
    let a = Portal::new(Vec3::new(0., 0., 54.), Vec3::X, Vec3::Z);
    let b = Portal::new(Vec3::new(200., 0., 54.), -Vec3::X, Vec3::Z);
    w.portals = [Some(a), Some(b)];
    let mut p = Player::new(Vec3::new(30., 0., 18.), std::f32::consts::PI);
    p.velocity = -Vec3::X * 190.;
    for _ in 0..30 {
        p.step(&w, Vec3::X, false, 1. / 120.);
    }
    assert!(p.position.x > 150., "{:?}", p.position);
    assert!(p.velocity.x < -189.);
    let mut rim = Player::new(Vec3::new(30., 31., 18.), std::f32::consts::PI);
    for _ in 0..30 {
        rim.step(&w, Vec3::X, false, 1. / 120.);
    }
    assert!(
        rim.position.x > 15. && rim.position.x < 31.,
        "rim {:?}",
        rim.position
    );
}
#[test]
fn actual_placed_floor_to_wall_preserves_fall_speed_and_orientation() {
    use opensource::{
        assets::Assets,
        play::{Player, World},
    };
    let bytes = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&bytes)).unwrap();
    let s = Surfaces::load(&bytes, &fs).unwrap();
    let mut w = World::parse(&bytes).unwrap();
    let floor = s.place(Vec3::new(-416., -320., 178.), -Vec3::Z).unwrap();
    let wall = s.place(Vec3::new(-1088., -600., 640.), -Vec3::Y).unwrap();
    w.portals = [Some(floor), Some(wall)];
    let mut p = Player::new(floor.center + Vec3::Z * 40., 0.);
    p.velocity = -Vec3::Z * 300.;
    p.pitch = -1.;
    let mut last = Vec3::ZERO;
    for _ in 0..120 {
        last = p.velocity;
        p.step(&w, Vec3::ZERO, false, 1. / 120.);
        if p.portal_crossings > 0 {
            break;
        }
    }
    assert_eq!(p.portal_crossings, 1, "{:?}", p.position);
    let expected = floor.map_vector(wall, last - Vec3::Z * 5.);
    assert!(
        p.velocity.distance(expected) < 0.1,
        "{:?} vs {expected:?}",
        p.velocity
    );
    assert!(p.velocity.y > 300.);
    let mapped_up = floor.map_vector(wall, Vec3::Z);
    let expected_up = (mapped_up - p.forward() * mapped_up.dot(p.forward())).normalize();
    assert!(p.view_up.distance(expected_up) < 0.001);
    assert!(!w.overlaps(p.position, p.height()));
    let speed = p.velocity.y;
    p.step(&w, Vec3::X, false, 1. / 120.);
    assert!((p.velocity.y - speed).abs() < 0.1, "air momentum lost");
}
#[test]
fn floor_exit_roll_settles_without_changing_forward_or_velocity() {
    use opensource::play::{Player, World};
    let w = World::from_brushes(vec![]);
    let mut p = Player::new(Vec3::ZERO, 0.);
    p.view_up = Vec3::Y;
    p.velocity = Vec3::X * 350.;
    let forward = p.forward();
    for _ in 0..120 {
        p.step(&w, Vec3::ZERO, false, 1. / 120.);
    }
    assert!(p.view_up.dot(Vec3::Z) > 0.99);
    assert!(p.forward().distance(forward) < 0.001);
    assert!(p.velocity.x > 349.);
}
#[test]
fn actual_placed_wall_to_floor_launches_upward_without_speed_reset() {
    use opensource::{
        assets::Assets,
        play::{Player, World},
    };
    let bytes = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&bytes)).unwrap();
    let s = Surfaces::load(&bytes, &fs).unwrap();
    let mut w = World::parse(&bytes).unwrap();
    let floor = s.place(Vec3::new(-416., -320., 178.), -Vec3::Z).unwrap();
    let wall = s.place(Vec3::new(-1088., -600., 640.), -Vec3::Y).unwrap();
    w.portals = [Some(wall), Some(floor)];
    let mut p = Player::new(
        wall.center + wall.normal * 20. - Vec3::Z * 36.,
        -std::f32::consts::FRAC_PI_2,
    );
    p.velocity = -Vec3::Y * 350.;
    let mut before = Vec3::ZERO;
    for _ in 0..60 {
        before = p.velocity;
        p.step(&w, Vec3::ZERO, false, 1. / 120.);
        if p.portal_crossings > 0 {
            break;
        }
    }
    assert_eq!(p.portal_crossings, 1, "{:?}", p.position);
    assert!(p.velocity.z > 349.);
    assert!(
        p.velocity
            .distance(wall.map_vector(floor, before - Vec3::Z * 5.))
            < 0.1
    );
    assert!(!w.overlaps(p.position, p.height()));
}
#[test]
fn actual_gpu_portal_samples_linked_scene_only_inside_aperture() {
    use opensource::{assets::Assets, render_portal_scene, scene::Scene};
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = Assets::from_game("../portal-original/portal", Some(&b)).unwrap();
    let scene = Scene::load(&b, &fs).unwrap();
    let camera = Some(([-740., -320., 192.], [-832., -320., 192.]));
    let a = Portal::new(Vec3::new(-832., -320., 180.), Vec3::X, Vec3::Z);
    let b = Portal::new(Vec3::new(-591., -412., 212.), Vec3::Y, Vec3::Z);
    let plain = render_portal_scene(&scene, 256, 192, camera, [None, None]).unwrap();
    let linked = render_portal_scene(&scene, 256, 192, camera, [Some(a), Some(b)]).unwrap();
    let center = (96 * 256 + 128) * 4;
    assert_ne!(&plain[center..center + 4], &linked[center..center + 4]);
    assert_eq!(&plain[..64], &linked[..64]);
    let mut clipped_scene = scene;
    // Paint only triangles wholly behind the destination plane; they must not enter the linked view.
    for tri in clipped_scene.vertices.chunks_exact_mut(3) {
        if tri.iter().all(|v| v.position[1] < -412.1) {
            for v in tri {
                v.layer = 0;
                v.unlit = 1.;
                v.light_uv = [0., 0.];
            }
        }
    }
    clipped_scene.textures[0] =
        vec![255; opensource::scene::TILE * opensource::scene::TILE * 4];
    let clipped =
        render_portal_scene(&clipped_scene, 256, 192, camera, [Some(a), Some(b)]).unwrap();
    assert_eq!(
        &linked[center..center + 4],
        &clipped[center..center + 4],
        "geometry behind exit plane leaked into remote view"
    );
    let moved = render_portal_scene(
        &clipped_scene,
        256,
        192,
        Some(([-730., -310., 192.], [-832., -310., 192.])),
        [Some(a), Some(b)],
    )
    .unwrap();
    assert_ne!(linked, moved);
}
#[test]
fn noportal_surface_flag_rejects_an_otherwise_fitting_concrete_floor() {
    let mut b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = opensource::assets::Assets::from_game("../portal-original/portal", Some(&b))
        .unwrap();
    let eye = Vec3::new(-544., -320., 210.);
    assert!(Surfaces::load(&b, &fs)
        .unwrap()
        .place(eye, -Vec3::Z)
        .is_ok());
    let off = i32::from_le_bytes(b[104..108].try_into().unwrap()) as usize;
    let len = i32::from_le_bytes(b[108..112].try_into().unwrap()) as usize;
    for a in b[off..off + len].chunks_exact_mut(72) {
        let flags = i32::from_le_bytes(a[64..68].try_into().unwrap()) | 0x20;
        a[64..68].copy_from_slice(&flags.to_le_bytes());
    }
    assert!(Surfaces::load(&b, &fs)
        .unwrap()
        .place(eye, -Vec3::Z)
        .is_err());
}
#[test]
fn chamber_ceiling_and_opposed_walls_face_the_air_not_solid() {
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let fs = opensource::assets::Assets::from_game("../portal-original/portal", Some(&b))
        .unwrap();
    let s = Surfaces::load(&b, &fs).unwrap();
    let p = s.place(Vec3::new(-512., -640., 300.), Vec3::Z).unwrap();
    assert!(p.normal.z < -0.99);
    assert!((p.center.z - 384.).abs() < 0.1);
}
#[test]
fn blocked_exit_does_not_let_player_fall_through_source_wall() {
    use opensource::play::{Plane, Player, World};
    let box_brush = |lo: Vec3, hi: Vec3| {
        vec![
            Plane {
                normal: Vec3::X,
                distance: hi.x,
            },
            Plane {
                normal: -Vec3::X,
                distance: -lo.x,
            },
            Plane {
                normal: Vec3::Y,
                distance: hi.y,
            },
            Plane {
                normal: -Vec3::Y,
                distance: -lo.y,
            },
            Plane {
                normal: Vec3::Z,
                distance: hi.z,
            },
            Plane {
                normal: -Vec3::Z,
                distance: -lo.z,
            },
        ]
    };
    let mut w = World::from_brushes(vec![
        box_brush(Vec3::new(-10., -500., -500.), Vec3::new(0., 500., 500.)),
        box_brush(Vec3::new(175., -100., -100.), Vec3::new(195., 100., 200.)),
    ]);
    w.portals = [
        Some(Portal::new(Vec3::new(0., 0., 54.), Vec3::X, Vec3::Z)),
        Some(Portal::new(Vec3::new(200., 0., 54.), -Vec3::X, Vec3::Z)),
    ];
    let mut p = Player::new(Vec3::new(30., 0., 18.), std::f32::consts::PI);
    p.velocity = -Vec3::X * 190.;
    for _ in 0..30 {
        p.step(&w, Vec3::X, false, 1. / 120.);
    }
    assert_eq!(p.portal_crossings, 0);
    assert!(p.position.x > 15., "blocked {:?}", p.position);
}
#[test]
fn actual_authored_chamber_pair_crosses_with_solid_exit_clearance() {
    use opensource::{
        entity::Runtime,
        play::{Player, World},
    };
    let bytes = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let mut r = Runtime::new(&bytes).unwrap();
    r.portal_events = Some(vec![]);
    let mut w = World::parse(&bytes).unwrap();
    let mut p = Player::new(Vec3::new(-740., -320., 128.03125), std::f32::consts::PI);
    let mut game = opensource::portal::Game::load(&bytes).unwrap();
    for _ in 0..120 {
        r.tick(1. / 120., &mut p, &mut w);
    }
    r.input("relay_portal_cancel_room1", "Trigger", "");
    r.tick(1. / 120., &mut p, &mut w);
    game.apply_events(r.portal_events.as_mut().unwrap());
    w.portals = game.portals;
    for _ in 0..90 {
        p.step(&w, Vec3::X, false, 1. / 120.);
    }
    assert_eq!(p.portal_crossings, 1, "{:?}", p.position);
    assert!(p.position.x > -650.);
    assert!(!w.overlaps(p.position, p.height()), "exit {:?}", p.position);
}
#[test]
fn portal_camera_basis_recovers_when_mouse_look_aligns_with_old_up() {
    use opensource::play::{Player, World};
    let w = World::from_brushes(vec![]);
    let mut p = Player::new(Vec3::ZERO, 0.);
    p.view_up = Vec3::X;
    p.step(&w, Vec3::ZERO, false, 1. / 120.);
    assert!(p.view_up.dot(p.forward()).abs() < 0.001);
    assert!(p.view_up.is_finite());
}
#[test]
fn manually_replaced_portal_is_not_fizzled_by_old_authored_name() {
    let bytes = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let mut game = opensource::portal::Game::load(&bytes).unwrap();
    game.apply_events(&mut vec![("portal_blue_0".into(), true)]);
    let replacement = Portal::new(Vec3::new(-416., -320., 128.), Vec3::Z, Vec3::Y);
    game.place(0, replacement);
    game.apply_events(&mut vec![("portal_blue_0".into(), false)]);
    assert_eq!(game.portals[0].unwrap().center, replacement.center);
}
#[test]
fn chamber_script_activates_and_fizzles_named_portals() {
    use opensource::{
        entity::Runtime,
        play::{Player, World},
    };
    let bytes = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let mut runtime = Runtime::new(&bytes).unwrap();
    let mut world = World::parse(&bytes).unwrap();
    let mut player = Player::spawn(&world);
    let mut game = opensource::portal::Game::load(&bytes).unwrap();
    runtime.portal_events = Some(vec![]);
    runtime.input("relay_portal_cancel_room1", "Trigger", "");
    runtime.tick(1. / 120., &mut player, &mut world);
    game.apply_events(runtime.portal_events.as_mut().unwrap());
    assert_eq!(
        game.portals[0].unwrap().center,
        Vec3::new(-832., -320., 180.)
    );
    assert_eq!(
        game.portals[1].unwrap().center,
        Vec3::new(-591., -412., 212.)
    );
    runtime.input("portal_blue_0", "Fizzle", "");
    runtime.tick(1. / 120., &mut player, &mut world);
    game.apply_events(runtime.portal_events.as_mut().unwrap());
    assert!(game.portals[0].is_none());
    assert!(!runtime
        .unknown_inputs
        .iter()
        .any(|i| i.contains("prop_portal:")));
}
#[test]
fn linked_view_clips_geometry_behind_exit_plane() {
    let a = Portal::new(Vec3::ZERO, Vec3::X, Vec3::Z);
    let b = Portal::new(Vec3::new(200., 0., 54.), Vec3::Y, Vec3::Z);
    let (eye, forward, up, clip) = a.linked_view(b, Vec3::new(30., 0., 0.), -Vec3::X, Vec3::Z);
    assert!((eye - b.center).dot(b.normal) < 0.);
    assert!(forward.dot(b.normal) > 0.99);
    assert!(up.dot(Vec3::Z) > 0.99);
    assert!(clip.dot((b.center + b.normal).extend(1.)) > 0.);
    assert!(clip.dot((b.center - b.normal).extend(1.)) < 0.);
}
#[test]
fn wall_to_floor_transform_preserves_speed_and_camera_basis() {
    let a = Portal::new(Vec3::ZERO, Vec3::X, Vec3::Z);
    let b = Portal::new(Vec3::new(100., 200., 0.), Vec3::Z, Vec3::Y);
    let v = Vec3::new(-300., 12., 30.);
    let mapped = a.map_vector(b, v);
    assert!((mapped.length() - v.length()).abs() < 0.001);
    assert!(mapped.z > 299.);
    let p = Vec3::new(8., 4., 5.);
    assert!(b.map_point(a, a.map_point(b, p)).distance(p) < 0.001);
    assert!(a.map_vector(b, Vec3::Z).dot(mapped.normalize()).abs() < 0.11);
}
