//! Actual loaded runtime/controller/animation path, without GPU or presentation.
//! Legal wall pairs and approach poses are staged, not the user's input route.
//! Chamber06 must really cross; the escape02 attempted crossing remains blocked.
//! This is a CPU separation probe, not gameplay FPS or an Intel reproduction.
use glam::Vec3;
use opensource::session::GameRoot;
use std::time::Instant;
fn main() -> Result<(), String> {
    let root = GameRoot::new(std::env::args().nth(1).ok_or("game root required")?, 4)?;
    opensource::frame_profile::init();
    for map in ["escape_02", "testchmb_a_06"] {
        let mut state = root.load_map(map, true)?;
        let mut weapon = root.weapon_viewmodel()?;
        println!("map={map} spawn={:?} brushes={}", state.player.position, state.world.brushes.len());
        let surfaces = state.surfaces.as_ref().unwrap();
        let mut candidates = Vec::new();
        for face in surfaces.faces.iter().filter(|f| f.model == 0 && f.allowed && f.normal.z.abs() < 0.1) {
            let center = face.points.iter().copied().sum::<Vec3>() / face.points.len() as f32;
            if let Ok(portal) = surfaces.place(center + face.normal * 64., -face.normal) {
                if candidates.iter().all(|p: &opensource::portal::Portal| p.center.distance(portal.center) > 128.) {
                    candidates.push(portal);
                }
            }
        }
        if candidates.len() < 2 { return Err(format!("{map}: fewer than two legal wall placements")); }
        candidates.sort_by(|a,b| a.center.distance_squared(state.player.position).total_cmp(&b.center.distance_squared(state.player.position)));
        println!("legal wall placement candidates={}", candidates.len());
        for phase in ["cold", "warm", "unlinked_far", "linked_far", "linked_near", "crossing_attempt"] {
            if phase == "unlinked_far" { state.game.portals = [candidates.first().copied(), None]; }
            if phase == "linked_far" { state.game.portals = [candidates.first().copied(), candidates.get(1).copied()]; }
            if phase == "linked_near" || phase == "crossing_attempt" {
                let src = candidates[0];
                state.player.position = src.center - Vec3::Z * 36. + src.normal * 48.;
                state.player.velocity = Vec3::ZERO;
                state.player.yaw = (-src.normal.y).atan2(-src.normal.x);
            }
            state.game.sync_runtime(&mut state.runtime, &mut state.world);
            let mut times = Vec::new();
            for frame in 0..240 {
                opensource::frame_profile::begin();
                let t = Instant::now();
                for _ in 0..2 {
                    state.runtime.tick(1. / 120., &mut state.player, &mut state.world);
                    state.game.advance_shots(1. / 120., &state.runtime, &mut state.world);
                    state.game.sync_runtime(&mut state.runtime, &mut state.world);
                    let input = if phase == "crossing_attempt" && frame < 60 { Vec3::X } else { Vec3::ZERO };
                    state.player.step(&state.world, input, phase == "crossing_attempt" && frame == 30, 1. / 120.);
                }
                let sim = t.elapsed().as_secs_f64() * 1000.;
                opensource::frame_profile::mark(0);
                let t = Instant::now();
                state.scene.animate(&state.runtime)?;
                let anim = t.elapsed().as_secs_f64() * 1000.;
                opensource::frame_profile::mark(1);
                let t = Instant::now();
                {
                    let _profile = opensource::frame_profile::scope(17);
                    weapon.pose_weapon("idle", frame as f32 / 60.)?;
                }
                times.push([sim, anim, t.elapsed().as_secs_f64() * 1000.]);
                opensource::frame_profile::mark(6);
                opensource::frame_profile::context(&state.player, &state.world, 0, 0);
                opensource::frame_profile::finish();
            }
            for (i, name) in ["simulation", "animation", "viewmodel"].iter().enumerate() {
                let mut values: Vec<_> = times.iter().map(|v| v[i]).collect();
                values.sort_by(f64::total_cmp);
                println!("{phase} {name}: median={:.3} p99={:.3} max={:.3} ms", values[120], values[237], values[239]);
            }
            println!("portals={:?} crossings={} player={:?}", state.world.portals, state.player.portal_crossings, state.player.position);
            if map == "testchmb_a_06" && phase == "crossing_attempt" && state.player.portal_crossings == 0 {
                return Err("chamber06 probe failed to perform a real controller crossing".into());
            }
        }
    }
    Ok(())
}
