use crate::Vertex;
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
pub fn render(
    vertices: &[Vertex],
    w: u32,
    h: u32,
    camera: Option<([f32; 3], [f32; 3])>,
) -> Result<Vec<u8>, String> {
    render_internal(vertices, None, w, h, camera, [None; 2])
}
pub fn render_scene(
    scene: &crate::scene::Scene,
    w: u32,
    h: u32,
    camera: Option<([f32; 3], [f32; 3])>,
) -> Result<Vec<u8>, String> {
    render_portal_scene(scene, w, h, camera, [None; 2])
}
pub fn render_portal_scene(
    scene: &crate::scene::Scene,
    w: u32,
    h: u32,
    camera: Option<([f32; 3], [f32; 3])>,
    portals: [Option<crate::portal::Portal>; 2],
) -> Result<Vec<u8>, String> {
    let vertices = scene
        .vertices
        .iter()
        .map(|v| Vertex {
            position: (Vec3::from(v.position) + scene.origins[v.model as usize]).to_array(),
            color: [1.; 3],
        })
        .collect::<Vec<_>>();
    render_internal(&vertices, Some(scene), w, h, camera, portals)
}
fn render_internal(
    vertices: &[Vertex],
    scene: Option<&crate::scene::Scene>,
    w: u32,
    h: u32,
    camera: Option<([f32; 3], [f32; 3])>,
    portals: [Option<crate::portal::Portal>; 2],
) -> Result<Vec<u8>, String> {
    pollster::block_on(async {
        if w == 0 || h == 0 || w > 4096 || h > 4096 || vertices.is_empty() {
            return Err("invalid render size/geometry".into());
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .ok_or("no wgpu adapter")?;
        println!("adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Portal BSP"),
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
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for v in vertices {
            lo = lo.min(Vec3::from(v.position));
            hi = hi.max(Vec3::from(v.position));
        }
        let center = (lo + hi) * 0.5;
        let radius = (hi - lo).length().max(1.);
        let (eye, target) = camera
            .map(|(e, t)| (Vec3::from(e), Vec3::from(t)))
            .unwrap_or((center + Vec3::new(0.8, -1.2, 0.9) * radius, center));
        let m = Mat4::perspective_rh(55f32.to_radians(), w as f32 / h as f32, 0.5, radius * 20.)
            * Mat4::look_at_rh(eye, target, Vec3::Z);
        let textured = scene
            .map(|s| {
                crate::scene_gpu::GpuScene::new(
                    &device,
                    &queue,
                    s,
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                )
            })
            .transpose()?;
        if let (Some(gpu), Some(scene)) = (&textured, scene) {
            gpu.update(&queue, m, &scene.origins);
        }
        let mut portal_gpu =
            crate::portal_gpu::PortalGpu::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb, w, h);
        portal_gpu.update(&queue, portals, eye, m);
        if let (Some(gpu), [Some(a), Some(b)]) = (&textured, portals) {
            for (index, (src, dst)) in [(a, b), (b, a)].into_iter().enumerate() {
                let (e, f, u, clip) =
                    src.linked_view(dst, eye, (target - eye).normalize(), Vec3::Z);
                let matrix = Mat4::perspective_rh(
                    55f32.to_radians(),
                    w as f32 / h as f32,
                    0.1,
                    radius * 20.,
                ) * Mat4::look_at_rh(e, e + f, u);
                gpu.update_view(&queue, index + 1, matrix, clip);
            }
        }
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&m.to_cols_array()),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 24,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
        });
        let size = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let pitch = (w * 4).div_ceil(256) * 256;
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (pitch * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let cv = color.create_view(&Default::default());
            let dv = depth.create_view(&Default::default());
            if let Some(gpu) = &textured {
                if portals.iter().all(Option::is_some) {
                    for index in 0..2 {
                        portal_gpu.remote(&mut enc, gpu, index);
                    }
                }
            }
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &cv,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.018,
                            g: 0.025,
                            b: 0.04,
                            a: 1.,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &dv,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if let Some(gpu) = &textured {
                gpu.draw(&mut pass);
                portal_gpu.draw(&mut pass);
            } else {
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.draw(0..vertices.len() as u32, 0..1);
            }
        }
        enc.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &color,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &read,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(pitch),
                    rows_per_image: Some(h),
                },
            },
            size,
        );
        queue.submit(Some(enc.finish()));
        let slice = read.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        let mapped = slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((w * h * 4) as usize);
        for row in mapped.chunks_exact(pitch as usize) {
            pixels.extend_from_slice(&row[..w as usize * 4]);
        }
        drop(mapped);
        read.unmap();
        Ok(pixels)
    })
}
