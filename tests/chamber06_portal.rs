use glam::Vec3;
use opensource::{play::Player, portal::Portal};

#[test]
fn third_room_floor_portal_exits_through_parented_panel() {
    let root = opensource::session::GameRoot::new("../portal-original/portal", 4).unwrap();
    let mut state = root.load_map("testchmb_a_06", true).unwrap();
    let surfaces = state.surfaces.as_ref().unwrap();
    let (runtime, world) = (&mut state.runtime, &mut state.world);
    let mut observer = Player::new(Vec3::new(704., 128., 88.), 0.);
    observer.noclip = true;
    for moved in [false, true] {
        if moved {
            runtime.input("fling_pillar3_door_bottom_bottom_a2", "Close", "");
            for _ in 0..60 {
                runtime.tick(1. / 120., &mut observer, world);
            }
        }
        let posed = surfaces.posed(runtime);
        let orange = posed
            .fit_authored(Portal::new(
                Vec3::new(600., 127.713, -192.),
                Vec3::Z,
                -Vec3::X,
            ))
            .unwrap();
        let model = 22;
        let face = surfaces
            .faces
            .iter()
            .find(|f| f.model == model && f.allowed && f.normal.x > 0.9)
            .unwrap();
        let (origins, rotations) = runtime.model_transforms();
        let center = origins[model]
            + rotations[model]
                * (face.points.iter().copied().sum::<Vec3>() / face.points.len() as f32);
        let blue = posed
            .place(center + face.normal * 100., -face.normal)
            .unwrap();
        world.portals = [Some(blue), Some(orange)];
        // Video: player stops at (606.9, 123.7, -192) above this aperture.
        let mut player = Player::new(Vec3::new(606.9, 123.7, -180.), 1.20);
        for _ in 0..180 {
            player.step(world, Vec3::ZERO, false, 1. / 120.);
            if player.portal_crossings > 0 {
                break;
            }
        }
        assert_eq!(
            player.portal_crossings, 1,
            "model={model} moved={moved}: stuck at {:?}, blue={blue:?}, orange={orange:?}",
            player.position
        );
        assert!(
            !world.overlaps(player.position, player.height()),
            "exit must remain clear"
        );
    }
}
