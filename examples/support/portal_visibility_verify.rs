//! Shared real-asset differential verification for portal_render_profile.
use super::*;
pub(super) fn verify_visibility(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    gpu: &scene_gpu::GpuScene,
    scene: &mut scene::Scene,
    eye: Vec3,
    forward: Vec3,
    a: portal::Portal,
    b: portal::Portal,
) -> Result<(), String> {
    let (width, height) = (640, 360);
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("visibility differential"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let color_view = color.create_view(&Default::default());
    let depth = device
        .create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default());
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (width * height * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let projection = Mat4::perspective_rh(
        55f32.to_radians(),
        width as f32 / height as f32,
        0.1,
        100000.,
    );
    let right = forward.cross(Vec3::Z);
    // Center, both aperture edges, close to the near plane, and looking away.
    for (case, offset, yaw) in [
        ("center", Vec3::ZERO, 0f32),
        ("left", -right * 65., 0.),
        ("right", right * 65., 0.),
        ("near", forward * 125., 0.),
        ("turned", Vec3::ZERO, 0.5),
        ("away", Vec3::ZERO, std::f32::consts::PI),
        ("moved", Vec3::ZERO, 0.),
    ] {
        let e = eye + offset;
        let f = glam::Mat3::from_rotation_z(yaw) * forward;
        let matrix = projection * Mat4::look_at_rh(e, e + f, Vec3::Z);
        let mut origins = scene.origins.clone();
        let mut saved = Vec::new();
        if case == "moved" {
            for p in origins.iter_mut().skip(1) {
                *p += right * 512.;
            }
            for anim in &mut scene.animations {
                saved.push((
                    anim.range.clone(),
                    scene.vertices[anim.range.clone()].to_vec(),
                    anim.updated,
                ));
                for v in &mut scene.vertices[anim.range.clone()] {
                    v.position = (Vec3::from(v.position) + right * 512.).to_array();
                }
                anim.updated = true;
            }
            gpu.animation_vertices(queue, scene);
            println!(
                "differential moving_models={} animated_ranges={}",
                origins.len().saturating_sub(1),
                saved.len()
            );
        }
        gpu.update(queue, matrix, &origins);
        let mut portals = portal_gpu::PortalGpu::new(device, format, width, height);
        portals.update(queue, [Some(a), Some(b)], e, matrix);
        for (i, (src, dst)) in [(a, b), (b, a)].into_iter().enumerate() {
            let (re, rf, ru, clip) = src.linked_view(dst, e, f, Vec3::Z);
            gpu.update_view(
                queue,
                i + 1,
                projection * Mat4::look_at_rh(re, re + rf, ru),
                clip,
            );
        }
        let mut images = Vec::new();
        for reference in [true, false] {
            gpu.set_reference(reference);
            let mut encoder = device.create_command_encoder(&Default::default());
            for i in 0..2 {
                portals.remote(&mut encoder, gpu, i);
            }
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("differential main"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &color_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                gpu.draw(&mut pass);
                portals.draw(&mut pass);
            }
            encoder.copy_texture_to_buffer(
                color.as_image_copy(),
                wgpu::ImageCopyBuffer {
                    buffer: &read,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(width * 4),
                        rows_per_image: Some(height),
                    },
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(Some(encoder.finish()));
            let (tx, rx) = std::sync::mpsc::channel();
            read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            device.poll(wgpu::Maintain::Wait);
            rx.recv()
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            images.push(read.slice(..).get_mapped_range().to_vec());
            read.unmap();
        }
        let changed = images[0]
            .chunks_exact(4)
            .zip(images[1].chunks_exact(4))
            .filter(|(a, b)| a != b)
            .count();
        println!(
            "differential case={case} changed_pixels={changed} main={:?} remote={:?}",
            gpu.submitted_work(0),
            gpu.submitted_work(1)
        );
        if changed != 0 {
            return Err(format!("visibility changed {changed} pixels in {case}"));
        }
        if case == "moved" {
            for (range, vertices, _) in &saved {
                scene.vertices[range.clone()].copy_from_slice(vertices);
            }
            gpu.animation_vertices(queue, scene);
            for (anim, (_, _, updated)) in scene.animations.iter_mut().zip(saved) {
                anim.updated = updated;
            }
        }
    }
    gpu.set_reference(false);
    Ok(())
}
