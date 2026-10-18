//! Orientation probes: no portal ownership, authored angles, or gameplay state is changed.
use glam::{Mat4, Vec3};
use opensource::portal::{Game, Portal, Surface, Surfaces, HALF_HEIGHT, HALF_WIDTH};

fn normals() -> [Vec3; 6] {
    [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z]
}
fn frame(normal: Vec3) -> Portal {
    Portal::new(
        Vec3::new(120., -80., 45.),
        normal,
        if normal.z.abs() > 0.99 {
            Vec3::Y
        } else {
            Vec3::Z
        },
    )
}

#[test]
fn placement_preserves_all_six_surface_normals_and_upright_frames() {
    for normal in normals() {
        let p = frame(normal);
        let surfaces = Surfaces {
            faces: vec![Surface {
                points: [(-200., -200.), (200., -200.), (200., 200.), (-200., 200.)]
                    .map(|(x, y)| p.center + p.right() * x + p.up * y)
                    .to_vec(),
                normal,
                allowed: true,
                material: "concrete/concrete_modular_fixture".into(),
            }],
        };
        let placed = surfaces.place(p.center + normal * 100., -normal).unwrap();
        assert!(placed.center.distance(p.center) < 0.001, "{normal:?}");
        assert!(placed.normal.distance(normal) < 0.001, "{normal:?}");
        assert!(placed.up.distance(p.up) < 0.001, "{normal:?}");
        assert!((placed.basis().determinant() - 1.).abs() < 0.001);
        assert!(
            surfaces.place(p.center - normal * 100., normal).is_err(),
            "back face {normal:?}"
        );
        // Same world-space corners and winding used by PortalGpu::update.
        let corner = |x, y| {
            placed.center
                + normal * 0.08
                + placed.right() * x * HALF_WIDTH
                + placed.up * y * HALF_HEIGHT
        };
        let [a, b, c] = [corner(-1., -1.), corner(1., -1.), corner(1., 1.)];
        assert!((b - a).cross(c - a).normalize().dot(normal) > 0.999);
    }
}

fn placement_fixture(normal: Vec3) -> Surfaces {
    let p = frame(normal);
    Surfaces {
        faces: vec![Surface {
            points: [(-500., -500.), (500., -500.), (500., 500.), (-500., 500.)]
                .map(|(x, y)| p.center + p.right() * x + p.up * y)
                .to_vec(),
            normal,
            allowed: true,
            material: "concrete/concrete_modular_fixture".into(),
        }],
    }
}

#[test]
fn horizontal_placement_roll_follows_four_firing_headings() {
    for normal in [Vec3::Z, -Vec3::Z] {
        let surfaces = placement_fixture(normal);
        for heading in [Vec3::X, Vec3::Y, -Vec3::X, -Vec3::Y] {
            let direction = (heading - normal).normalize();
            let p = surfaces
                .place(frame(normal).center - direction * 100., direction)
                .unwrap();
            assert!(
                p.up.distance(heading) < 0.001,
                "{normal:?} {heading:?}: {p:?}"
            );
            assert!((p.basis().determinant() - 1.).abs() < 0.001);
            assert!(p.right().distance(heading.cross(normal)) < 0.001);
        }
    }
}

#[test]
fn vertical_and_near_vertical_placement_uses_facing_without_nan() {
    for normal in [Vec3::Z, -Vec3::Z] {
        let surfaces = placement_fixture(normal);
        for heading in [Vec3::X, Vec3::Y, -Vec3::X, -Vec3::Y] {
            for direction in [-normal, (-normal + heading * 1e-7).normalize()] {
                let p = surfaces
                    .place_with_facing(frame(normal).center - direction * 100., direction, heading)
                    .unwrap();
                assert!(p.up.distance(heading) < 0.001);
                assert!(p.basis().is_finite());
                assert!((p.basis().determinant() - 1.).abs() < 0.001);
            }
        }
        let p = surfaces
            .place(frame(normal).center + normal * 100., -normal)
            .unwrap();
        assert_eq!(p.up, Vec3::Y); // Deterministic fallback for scripted rays without yaw.
    }
}

#[test]
fn wall_placement_remains_upright_for_oblique_firing_headings() {
    for normal in [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y] {
        let surfaces = placement_fixture(normal);
        for sideways in [-1., 1.] {
            let direction =
                (-normal + frame(normal).right() * sideways + Vec3::Z * 0.2).normalize();
            let p = surfaces
                .place(frame(normal).center - direction * 100., direction)
                .unwrap();
            assert!(p.up.distance(Vec3::Z) < 0.001);
            assert!((p.basis().determinant() - 1.).abs() < 0.001);
        }
    }
}

#[test]
fn linked_camera_and_screen_projection_agree_for_all_36_normal_pairs() {
    for source in normals().map(frame) {
        for destination in normals().map(frame) {
            let destination = Portal {
                center: Vec3::new(-250., 400., 160.),
                ..destination
            };
            let eye = source.center + source.normal * 120. + source.right() * 13. + source.up * 7.;
            let forward = (source.center - eye).normalize();
            let (remote_eye, remote_forward, remote_up, clip) =
                source.linked_view(destination, eye, forward, source.up);
            assert!(remote_eye.is_finite() && remote_forward.is_finite() && remote_up.is_finite());
            assert!((remote_eye - destination.center).dot(destination.normal) < 0.);
            assert!(
                source
                    .map_vector(destination, -source.normal)
                    .distance(destination.normal)
                    < 0.001
            );
            assert!(
                source
                    .map_vector(destination, source.up)
                    .distance(destination.up)
                    < 0.001
            );
            assert!(
                source
                    .map_vector(destination, source.right())
                    .distance(-destination.right())
                    < 0.001
            );
            assert!(clip.dot((destination.center + destination.normal).extend(1.)) > 0.);
            assert!(clip.dot((destination.center - destination.normal).extend(1.)) < 0.);
            let projection = Mat4::perspective_rh(55f32.to_radians(), 4. / 3., 0.5, 2000.);
            let main = projection * Mat4::look_at_rh(eye, eye + forward, source.up);
            let remote =
                projection * Mat4::look_at_rh(remote_eye, remote_eye + remote_forward, remote_up);
            for (x, y) in [(-25., -40.), (25., -40.), (25., 40.), (-25., 40.), (0., 0.)] {
                let point = source.center + source.right() * x + source.up * y;
                let mapped = source.map_point(destination, point);
                assert!(destination.map_point(source, mapped).distance(point) < 0.001);
                let a = main.project_point3(point);
                let b = remote.project_point3(mapped);
                assert!(
                    a.distance(b) < 0.001,
                    "{:?} -> {:?}: {a:?} != {b:?}",
                    source.normal,
                    destination.normal
                );
            }
        }
    }
}

#[test]
fn gpu_aperture_is_visible_from_front_for_all_six_normals() {
    use opensource::{assets::Assets, render_portal_scene, scene::Scene};
    let bytes = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let assets = Assets::from_game("../portal-original/portal", Some(&bytes)).unwrap();
    let scene = Scene::load(&bytes, &assets).unwrap();
    for normal in normals() {
        // Isolate orientation from map occlusion without modifying any map assets.
        let portal = Portal {
            center: Vec3::splat(20000.),
            ..frame(normal)
        };
        // Avoid the offscreen helper's fixed-Z look_at singularity for floor views.
        let eye = portal.center + normal * 120. + portal.up * 25.;
        let pixels = render_portal_scene(
            &scene,
            128,
            128,
            Some((eye.to_array(), portal.center.to_array())),
            [Some(portal), None],
        )
        .unwrap();
        let blue_pixels = pixels
            .chunks_exact(4)
            .filter(|p| p[2] > 40 && p[2] > p[0].saturating_mul(2) && p[2] > p[1])
            .count();
        assert!(
            blue_pixels > 100,
            "front aperture {normal:?}: {blue_pixels} blue pixels"
        );
    }
}

#[test]
fn authored_portal_angles_keep_source_forward_and_roll() {
    // Use the real BSP header, substituting only the in-memory entity lump.
    let original = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    for (angles, normal, up) in [
        ("0 0 0", Vec3::X, Vec3::Z),
        ("0 90 0", Vec3::Y, Vec3::Z),
        ("0 180 0", -Vec3::X, Vec3::Z),
        ("0 270 0", -Vec3::Y, Vec3::Z),
        ("-90 0 0", Vec3::Z, -Vec3::X),
        ("90 0 0", -Vec3::Z, Vec3::X),
        ("0 0 90", Vec3::X, -Vec3::Y),
    ] {
        let entity = format!("{{\n\"classname\" \"prop_portal\"\n\"origin\" \"10 20 30\"\n\"angles\" \"{angles}\"\n\"targetname\" \"fixture\"\n\"Activated\" \"1\"\n}}\n\0");
        let mut bytes = original.clone();
        let offset = bytes.len() as i32;
        bytes[8..12].copy_from_slice(&offset.to_le_bytes());
        bytes[12..16].copy_from_slice(&(entity.len() as i32).to_le_bytes());
        bytes.extend_from_slice(entity.as_bytes());
        let game = Game::load(&bytes).unwrap();
        let portal = game.portals[0].unwrap();
        assert!(game.portals[1].is_none());
        assert!(
            portal.normal.distance(normal) < 0.001,
            "angles {angles}: {portal:?}"
        );
        assert!(
            portal.up.distance(up) < 0.001,
            "angles {angles}: {portal:?}"
        );
    }
}
