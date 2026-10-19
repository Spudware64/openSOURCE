use glam::Vec3;
use opensource::portal::{Portal, Surface, Surfaces, HALF_HEIGHT, HALF_WIDTH};

fn panel(p: Portal, width: f32, height: f32) -> Surface {
    Surface {
        model: 0,
        points: [
            (-width, -height),
            (width, -height),
            (width, height),
            (-width, height),
        ]
        .map(|(x, y)| p.center + p.right() * x + p.up * y)
        .to_vec(),
        normal: p.normal,
        allowed: true,
        material: "concrete/concrete_modular_fixture".into(),
    }
}

#[test]
fn fitting_rejects_small_disconnected_forbidden_and_occluded_panels() {
    let p = Portal::new(Vec3::ZERO, Vec3::X, Vec3::Z);
    let eye = p.normal * 100.;
    let mut small = panel(p, 31., 100.);
    let remote = panel(
        Portal {
            center: p.right() * 80.,
            ..p
        },
        40.,
        100.,
    );
    assert!(Surfaces {
        faces: vec![small.clone(), remote]
    }
    .place(eye, -p.normal)
    .is_err());
    small.allowed = false;
    assert!(Surfaces {
        faces: vec![small.clone(), panel(p, 100., 100.)]
    }
    .place(eye, -p.normal)
    .is_err());
    let blocker = Surface {
        allowed: false,
        material: "metal/grate".into(),
        ..panel(
            Portal {
                center: p.normal * 50.,
                ..p
            },
            100.,
            100.,
        )
    };
    assert!(Surfaces {
        faces: vec![panel(p, 100., 100.), blocker]
    }
    .place(eye, -p.normal)
    .is_err());
    // A tiny forbidden island between the old 4-unit samples must still reject.
    let island = Surface {
        allowed: false,
        material: "metal/noportal".into(),
        ..panel(
            Portal {
                center: p.right() * 1.5,
                ..p
            },
            0.2,
            100.,
        )
    };
    assert!(Surfaces {
        faces: vec![panel(p, HALF_WIDTH, HALF_HEIGHT), island]
    }
    .place(eye, -p.normal)
    .is_err());
}

#[test]
fn connected_seams_cover_the_whole_aperture_without_requiring_one_face() {
    let p = Portal::new(Vec3::ZERO, Vec3::X, Vec3::Z);
    let faces = [-20., 20.]
        .map(|x| {
            panel(
                Portal {
                    center: p.right() * x,
                    ..p
                },
                20.,
                70.,
            )
        })
        .to_vec();
    let fitted = Surfaces { faces }
        .place(p.normal * 100., -p.normal)
        .unwrap();
    assert!(fitted.center.length() < 0.001);
}

#[test]
fn actual_chamber_edge_shot_fits_and_player_replacement_keeps_channel_rules() {
    use opensource::{assets::Assets, entity::Runtime, play::World, portal::Game};
    let bytes = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let assets = Assets::from_game("../portal-original/portal", Some(&bytes)).unwrap();
    let surfaces = Surfaces::load(&bytes, &assets).unwrap();
    let mut world = World::parse(&bytes).unwrap();
    let mut runtime = Runtime::new(&bytes).unwrap();
    runtime.portal_inventory = [true, true];
    let mut game = Game::load(&bytes).unwrap();
    game.portals = [None, None];
    game.automatic_surfaces = Some(surfaces.clone());
    // Search authored face edges; require an actual shifted placement via production fire.
    for face in surfaces.faces.iter().filter(|s| s.allowed) {
        let center = face.points.iter().copied().sum::<Vec3>() / face.points.len() as f32;
        for vertex in &face.points {
            let aim = vertex.lerp(center, 0.02);
            let eye = aim + face.normal * 8.;
            if let Ok(p) = game.fire_player(0, eye, -face.normal, &runtime, &mut world) {
                if p.center.distance(aim) > 1. {
                    let replacement = game
                        .fire_player(0, eye, -face.normal, &runtime, &mut world)
                        .unwrap();
                    assert!(replacement.center.distance(p.center) < 0.01);
                    assert!(game
                        .fire_player(1, eye, -face.normal, &runtime, &mut world)
                        .is_err());
                    assert_eq!(world.portals[0].unwrap().center, p.center);
                    return;
                }
            }
        }
    }
    panic!("no real authored panel edge was fitted");
}

#[test]
fn near_edge_shots_fit_wall_floor_ceiling_and_corners() {
    for normal in [Vec3::X, Vec3::Z, -Vec3::Z] {
        let p = Portal::new(
            Vec3::ZERO,
            normal,
            if normal.z.abs() > 0.99 {
                Vec3::Y
            } else {
                Vec3::Z
            },
        );
        let s = Surfaces {
            faces: vec![panel(p, 80., 100.)],
        };
        for (x, y) in [(79., 0.), (0., 99.), (79., 99.), (-79., -99.)] {
            let aim = p.right() * x + p.up * y;
            let fitted = s
                .place(aim + normal * 100., -normal)
                .expect("edge shot should bump inward");
            let expected = p.right() * x.clamp(-80. + HALF_WIDTH, 80. - HALF_WIDTH)
                + p.up * y.clamp(-100. + HALF_HEIGHT, 100. - HALF_HEIGHT);
            assert!(
                fitted.center.distance(expected) < 0.05,
                "{normal:?} {aim:?}: {fitted:?}"
            );
            assert_eq!(fitted.normal, p.normal);
            assert_eq!(fitted.up, p.up);
        }
    }
}
