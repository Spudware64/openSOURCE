use glam::Vec3;
use portal_bsp_viewer::play::{Player, World};

fn grate_world() -> (World, World) {
    let mut bytes = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let solid = World::parse(&bytes).unwrap();
    let offset = i32::from_le_bytes(bytes[296..300].try_into().unwrap()) as usize;
    let length = i32::from_le_bytes(bytes[300..304].try_into().unwrap()) as usize;
    // Retain actual authored hulls/model ownership, changing only contents.
    for brush in bytes[offset..offset + length].chunks_exact_mut(12) {
        let contents = i32::from_le_bytes(brush[8..12].try_into().unwrap());
        let contents: i32 = if contents & (1 | 2 | 8 | 0x4000 | 65536) != 0 { 8 } else { 0 };
        brush[8..12].copy_from_slice(&contents.to_le_bytes());
    }
    (solid, World::parse(&bytes).unwrap())
}

#[test]
fn authored_hulls_with_grate_contents_support_standing_and_walking() {
    let (solid, grate) = grate_world();
    assert_eq!(grate.brushes.len(), solid.brushes.len());
    let start = solid.spawn;
    let end = start - Vec3::Z * 2000.;
    let expected = solid.trace(start, end);
    assert!(expected.0 < 1.);
    assert_eq!(grate.trace(start, end), expected);
    let mut player = Player::new(start.lerp(end, expected.0), solid.yaw);
    let floor = player.position.z;
    for tick in 0..120 {
        player.step(&grate, if tick < 30 { Vec3::X } else { Vec3::ZERO }, false, 1. / 120.);
        assert!(player.position.z >= floor - 0.1, "fell through grate: {:?}", player.position);
        assert!(player.grounded);
    }
}

#[test]
fn portal_point_ray_passes_grate_but_player_hull_does_not() {
    let (solid, grate) = grate_world();
    let start = solid.spawn;
    let end = start - Vec3::Z * 2000.;
    assert!(solid.trace_point(start, end).0 < 1.);
    assert!(grate.trace(start, end).0 < 1.);
    assert_eq!(grate.trace_point(start, end).0, 1.);
}

#[test]
fn original_map_grate_brushes_are_not_discarded() {
    let bytes = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp").unwrap();
    let indices = portal_bsp_viewer::models::model_brush_indices(&bytes).unwrap();
    let offset = i32::from_le_bytes(bytes[296..300].try_into().unwrap()) as usize;
    let expected = indices[0].iter().filter(|&&id| {
        let c = i32::from_le_bytes(bytes[offset + id * 12 + 8..offset + id * 12 + 12].try_into().unwrap());
        c & (1 | 2 | 8 | 0x4000 | 65536) != 0
    }).count();
    assert_eq!(World::parse(&bytes).unwrap().model_brushes[0].len(), expected);
}
