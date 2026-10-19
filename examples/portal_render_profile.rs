//! Renderer-only diagnostic, not gameplay FPS. Original assets remain read-only.
//! cargo run --release --example portal_render_profile -- GAME MAP [FRAMES]
use glam::{Mat4, Vec3};
pub use opensource::{entity, play, portal, scene};
#[allow(dead_code)]
#[path = "../src/frame_profile.rs"]
mod frame_profile;
#[allow(dead_code)]
#[path = "../src/portal_gpu.rs"]
mod portal_gpu;
#[allow(dead_code)]
#[path = "../src/scene_gpu.rs"]
mod scene_gpu;
#[path = "support/portal_visibility_verify.rs"]
mod visibility_verify;
use visibility_verify::verify_visibility;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejection_keeps_crossing_triangles_and_uses_homogeneous_clip_space() {
        let tri = |p: [[f32; 3]; 3]| p.map(Vec3::from);
        assert!(rejected(
            tri([[2., 0., 0.5], [3., 1., 0.5], [2., -1., 0.5]]),
            Mat4::IDENTITY,
            glam::Vec4::ZERO
        ));
        assert!(!rejected(
            tri([[-2., 0., 0.5], [2., 2., 0.5], [2., -2., 0.5]]),
            Mat4::IDENTITY,
            glam::Vec4::ZERO
        ));
        assert!(rejected(
            tri([[0., 0., 0.5], [0.5, 0., 0.5], [0., 0.5, 0.5]]),
            Mat4::IDENTITY,
            glam::Vec4::new(0., 0., -1., 0.)
        ));
        assert!(!rejected(
            tri([[0., 0., -0.5], [0.5, 0., 0.5], [0., 0.5, 0.5]]),
            Mat4::IDENTITY,
            glam::Vec4::new(0., 0., -1., 0.)
        ));
    }
}
// Diagnostic only: conservative trivial rejection, not occlusion or PVS.
fn rejected(points: [Vec3; 3], matrix: Mat4, plane: glam::Vec4) -> bool {
    if points.iter().all(|p| plane.dot(p.extend(1.)) < 0.) {
        return true;
    }
    let clip = points.map(|p| matrix * p.extend(1.));
    (0..6).any(|n| {
        clip.iter().all(|p| match n {
            0 => p.x < -p.w,
            1 => p.x > p.w,
            2 => p.y < -p.w,
            3 => p.y > p.w,
            4 => p.z < 0.,
            _ => p.z > p.w,
        })
    })
}
fn inventory(scene: &scene::Scene, matrix: Mat4, plane: glam::Vec4, view: &str) {
    let mut frustum = 0;
    let mut exit_plane = 0;
    let mut either = 0;
    for t in scene.vertices.chunks_exact(3) {
        let p =
            std::array::from_fn(|n| Vec3::from(t[n].position) + scene.origins[t[n].model as usize]);
        frustum += usize::from(rejected(p, matrix, glam::Vec4::ZERO));
        exit_plane += usize::from(p.iter().all(|p| plane.dot(p.extend(1.)) < 0.));
        either += usize::from(rejected(p, matrix, plane));
    }
    println!("inventory view={view} triangles={} trivially_outside_frustum={frustum} wholly_behind_exit_plane={exit_plane} rejected_by_either={either}",scene.vertices.len()/3);
}
fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let game = args
        .get(1)
        .ok_or("usage: portal_render_profile GAME MAP [FRAMES]")?;
    let map = args.get(2).ok_or("missing MAP (e.g. escape_02)")?;
    let frames: usize = args
        .get(3)
        .map(|v| v.parse())
        .transpose()
        .map_err(|_| "invalid FRAMES")?
        .unwrap_or(30);
    if frames == 0 || frames > 10000 {
        return Err("FRAMES must be 1..10000".into());
    }
    let bytes = std::fs::read(
        std::path::Path::new(game)
            .join("maps")
            .join(format!("{map}.bsp")),
    )
    .map_err(|e| e.to_string())?;
    let assets = opensource::assets::Assets::from_game(game, Some(&bytes))?;
    let mut scene = scene::Scene::load(&bytes, &assets)?;
    let world = opensource::play::World::parse(&bytes)?;
    let eye = world.spawn + Vec3::Z * 64.;
    let forward = Vec3::new(world.yaw.cos(), world.yaw.sin(), 0.);
    let right = forward.cross(Vec3::Z);
    // Deliberately staged in front of spawn, not claimed to be valid placements
    // or the user's camera. Hold camera/mesh fixed to isolate rendering cost.
    let a = portal::Portal::new(eye + forward * 128., -forward, Vec3::Z);
    let b = portal::Portal::new(eye + right * 256., forward, Vec3::Z);
    let translucent = scene
        .vertices
        .chunks_exact(3)
        .filter(|t| t[0].unlit < 0.)
        .count();
    println!("map={map} vertices={} triangles={} translucent_triangles={} reference_scene_draws_per_view={} eye={eye:?} forward={forward:?}", scene.vertices.len(), scene.vertices.len()/3, translucent, 3+translucent);
    frame_profile::init();
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .ok_or("no GPU adapter")?;
        println!("adapter={:?}; build_debug_assertions={}; wait_each_frame=true; no simulation, surface, capture or presentation", adapter.get_info(), cfg!(debug_assertions));
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("portal workload diagnostic"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits {
                        max_texture_array_layers: 1024
                            .min(adapter.limits().max_texture_array_layers),
                        ..wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits())
                    },
                },
                None,
            )
            .await
            .map_err(|e| e.to_string())?;
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let gpu = scene_gpu::GpuScene::new(&device, &queue, &scene, format)?;
        if std::env::var_os("SOURCE_VERIFY_VISIBILITY").is_some() {
            verify_visibility(&device, &queue, &gpu, &mut scene, eye, forward, [a, b])?;
        }
        gpu.set_reference(std::env::var_os("SOURCE_REFERENCE_RENDER").is_some());
        for (width, height) in [(640, 360), (1280, 720)] {
            let texture = |format| {
                device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("profile target"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
            };
            let color = texture(format).create_view(&Default::default());
            let depth = texture(wgpu::TextureFormat::Depth32Float).create_view(&Default::default());
            let mut portals = portal_gpu::PortalGpu::new(&device, format, width, height);
            let projection = Mat4::perspective_rh(
                55f32.to_radians(),
                width as f32 / height as f32,
                0.1,
                100000.,
            );
            let matrix = projection * Mat4::look_at_rh(eye, eye + forward, Vec3::Z);
            gpu.update(&queue, matrix, &scene.origins);
            inventory(&scene, matrix, glam::Vec4::ZERO, "main");
            let (e, f, u, clip) = a.linked_view(b, eye, forward, Vec3::Z);
            inventory(
                &scene,
                projection * Mat4::look_at_rh(e, e + f, u),
                clip,
                "remote",
            );
            for (phase, pair) in [
                ("none", [None; 2]),
                ("visible", [Some(a), Some(b)]),
                (
                    "offscreen",
                    [
                        Some(portal::Portal::new(eye - forward * 128., forward, Vec3::Z)),
                        Some(b),
                    ],
                ),
            ] {
                portals.update(&queue, pair, eye, matrix);
                let remote_count = (0..2).filter(|&n| portals.needs_remote(n)).count();
                assert_eq!(
                    remote_count,
                    usize::from(phase == "visible"),
                    "diagnostic source aperture state"
                );
                if let [Some(src), Some(dst)] = pair {
                    for (index, (src, dst)) in [(src, dst), (dst, src)].into_iter().enumerate() {
                        let (e, f, u, clip) = src.linked_view(dst, eye, forward, Vec3::Z);
                        gpu.update_view(
                            &queue,
                            index + 1,
                            projection * Mat4::look_at_rh(e, e + f, u),
                            clip,
                        );
                    }
                }
                println!("work {width}x{height} phase={phase} main={:?} remote={:?} (opaque vertices, draws) translucent_main={} translucent_remote={}", gpu.submitted_work(0), gpu.submitted_work(1), gpu.translucent_work(0), gpu.translucent_work(1));
                if phase == "visible" {
                    let mut updates = Vec::new();
                    for _ in 0..200 {
                        let start = std::time::Instant::now();
                        gpu.update(&queue, matrix, &scene.origins);
                        for (i, (src, dst)) in [(a,b), (b,a)].into_iter().enumerate() {
                            let (e,f,u,clip) = src.linked_view(dst, eye, forward, Vec3::Z);
                            gpu.update_view(&queue, i+1, projection*Mat4::look_at_rh(e,e+f,u), clip);
                        }
                        updates.push(start.elapsed().as_secs_f64()*1000.);
                    }
                    updates.sort_by(f64::total_cmp);
                    println!("selection {width}x{height} main_plus_two_remote_update_median_ms={:.3}", updates[updates.len()/2]);
                }
                device.poll(wgpu::Maintain::Wait);
                let mut encode = Vec::new();
                let mut complete = Vec::new();
                for frame in 0..frames + 5 {
                    frame_profile::begin();
                    let start = std::time::Instant::now();
                    let mut encoder = device.create_command_encoder(&Default::default());
                    for index in 0..2 {
                        portals.remote(&mut encoder, &gpu, index);
                    }
                    frame_profile::mark(5);
                    {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("profile main"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &color,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: &depth,
                                    depth_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(1.),
                                        store: wgpu::StoreOp::Store,
                                    }),
                                    stencil_ops: None,
                                },
                            ),
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });
                        gpu.draw(&mut pass);
                        portals.draw(&mut pass);
                    }
                    frame_profile::mark(6);
                    let command = encoder.finish();
                    let encoded = start.elapsed().as_secs_f64() * 1000.;
                    queue.submit(Some(command));
                    device.poll(wgpu::Maintain::Wait);
                    frame_profile::mark(7);
                    frame_profile::finish();
                    if frame >= 5 {
                        encode.push(encoded);
                        complete.push(start.elapsed().as_secs_f64() * 1000.);
                    }
                }
                encode.sort_by(f64::total_cmp);
                complete.sort_by(f64::total_cmp);
                println!("{width}x{height} phase={phase} remote_views={remote_count} samples={frames} encode_median_ms={:.3} completed_median_ms={:.3} completed_max_ms={:.3}", encode[frames/2],complete[frames/2],complete[frames-1]);
            }
        }
        Ok(())
    })
}
