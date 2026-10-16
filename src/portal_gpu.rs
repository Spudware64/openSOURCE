//! One-hop linked views. Shared scene geometry/materials, two bounded color targets; no recursive rendering.
use crate::{
    portal::{Portal, HALF_HEIGHT, HALF_WIDTH},
    scene_gpu::GpuScene,
};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
    uv: [f32; 2],
    index: u32,
    linked: f32,
}
pub(crate) struct PortalGpu {
    pub views: [wgpu::TextureView; 2],
    depth: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    count: u32,
}
#[cfg(test)]
mod tests {
    #[test]
    fn bounded_dimensions_resize_preserve_projection_aspect() {
        assert_eq!(super::target_size(3840,2160,1.,1280), (1280,720));
        assert_eq!(super::target_size(800,600,0.5,1280), (400,300));
        assert_eq!(super::target_size(1920,1080,1.,4096), (1920,1080));
        assert_eq!(super::target_size(1,1,0.5,1280), (1,1));
    }
}
fn target_size(width: u32, height: u32, scale: f32, cap: u32) -> (u32,u32) {
    let factor = scale.min(cap as f32 / width.max(height).max(1) as f32);
    (((width as f32 * factor).round() as u32).max(1),
     ((height as f32 * factor).round() as u32).max(1))
}
impl PortalGpu {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        // Main projection and framebuffer remain full resolution. Only remote views are bounded.
        let scale = std::env::var("SOURCE_PORTAL_SCALE").ok().and_then(|v| v.parse::<f32>().ok())
            .filter(|s| s.is_finite() && *s > 0. && *s <= 1.).unwrap_or(1.);
        let cap = std::env::var("SOURCE_PORTAL_MAX_EDGE").ok().and_then(|v| v.parse::<u32>().ok())
            .filter(|c| *c > 0).unwrap_or(1280).min(device.limits().max_texture_dimension_2d);
        let viewport = [width as f32, height as f32, 0., 0.];
        let (width,height) = target_size(width,height,scale,cap);
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bounded portal depth"),
            size: wgpu::Extent3d { width,height,depth_or_array_layers:1 },
            mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,
            format:wgpu::TextureFormat::Depth32Float,usage:wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats:&[],
        }).create_view(&Default::default());
        let viewport = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("main viewport for normalized portal sampling"),
            contents: bytemuck::cast_slice(&viewport), usage:wgpu::BufferUsages::UNIFORM,
        });
        let views = std::array::from_fn(|_| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("one-hop portal view"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        });
        let entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("linked view textures"),
            entries: &[entry(0), entry(1), wgpu::BindGroupLayoutEntry {
                binding:2,visibility:wgpu::ShaderStages::FRAGMENT,
                ty:wgpu::BindingType::Buffer { ty:wgpu::BufferBindingType::Uniform,has_dynamic_offset:false,min_binding_size:None },count:None,
            }],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding:2,resource:viewport.as_entire_binding() },
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&views[1]),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("elliptical portal composite"),
            source: wgpu::ShaderSource::Wgsl(include_str!("portal.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline=device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{label:Some("depth-tested portal aperture"),layout:Some(&layout),vertex:wgpu::VertexState{module:&shader,entry_point:"vs",buffers:&[wgpu::VertexBufferLayout{array_stride:32,step_mode:wgpu::VertexStepMode::Vertex,attributes:&wgpu::vertex_attr_array![0=>Float32x4,1=>Float32x2,2=>Uint32,3=>Float32]}]},fragment:Some(wgpu::FragmentState{module:&shader,entry_point:"fs",targets:&[Some(wgpu::ColorTargetState{format,blend:None,write_mask:wgpu::ColorWrites::ALL})]}),primitive:wgpu::PrimitiveState{cull_mode:None,..Default::default()},depth_stencil:Some(wgpu::DepthStencilState{format:wgpu::TextureFormat::Depth32Float,depth_write_enabled:true,depth_compare:wgpu::CompareFunction::LessEqual,stencil:Default::default(),bias:Default::default()}),multisample:Default::default(),multiview:None});
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal quads"),
            contents: &[0u8; 384],
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            depth,
            views,
            pipeline,
            bind,
            vertices,
            count: 0,
        }
    }
    pub fn update(
        &mut self,
        queue: &wgpu::Queue,
        portals: [Option<Portal>; 2],
        eye: Vec3,
        matrix: Mat4,
    ) {
        let mut vertices = vec![];
        for (index, p) in portals.iter().enumerate() {
            if let Some(p) = p {
                if p.normal.dot(eye - p.center) <= 0. {
                    continue;
                }
                for [x, y] in [
                    [-1., -1.],
                    [1., -1.],
                    [1., 1.],
                    [-1., -1.],
                    [1., 1.],
                    [-1., 1.],
                ] {
                    let position = p.center
                        + p.normal * 0.08
                        + p.right() * x * HALF_WIDTH
                        + p.up * y * HALF_HEIGHT;
                    vertices.push(Vertex {
                        position: (matrix * position.extend(1.)).to_array(),
                        uv: [x, y],
                        index: index as u32,
                        linked: if portals.iter().all(Option::is_some) {
                            1.
                        } else {
                            0.
                        },
                    });
                }
            }
        }
        self.count = vertices.len() as u32;
        if !vertices.is_empty() {
            queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&vertices));
        }
    }
    pub fn remote(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        gpu: &GpuScene,
        index: usize,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("linked portal scene, recursion capped at one hop"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.views[index],
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        gpu.draw_view(&mut pass, index + 1);
    }
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..self.count, 0..1);
    }
}
