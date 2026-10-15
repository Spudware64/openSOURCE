use crate::scene::{MeshVertex, Scene, LIGHT_SIZE, TILE};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
pub(crate) struct GpuScene {
    pipeline: wgpu::RenderPipeline,
    translucent_pipeline: wgpu::RenderPipeline,
    translucent: Vec<(u32, Vec3, usize)>,
    matrices: [std::cell::Cell<Mat4>; 3],
    origins: std::cell::RefCell<Vec<Vec3>>,
    bind: [wgpu::BindGroup; 3],
    vertices: wgpu::Buffer,
    camera: [wgpu::Buffer; 3],
    models: wgpu::Buffer,
    count: u32,
}
impl GpuScene {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Scene,
        format: wgpu::TextureFormat,
    ) -> Result<Self, String> {
        let limits = device.limits();
        if scene.textures.is_empty()
            || scene.textures.len() > limits.max_texture_array_layers as usize
            || LIGHT_SIZE > limits.max_texture_dimension_2d as usize
            || scene.lightmap.len() != LIGHT_SIZE * LIGHT_SIZE * 4
            || scene.textures.iter().any(|t| t.len() != TILE * TILE * 4)
            || scene.vertices.is_empty()
            || (scene.vertices.len() * std::mem::size_of::<MeshVertex>()) as u64
                > limits.max_buffer_size
        {
            return Err(
                "scene exceeds GPU texture/buffer capacity or has invalid upload dimensions".into(),
            );
        }
        let camera = std::array::from_fn::<_, 3, _>(|_| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("camera and world clip plane"),
                contents: bytemuck::cast_slice(&[0f32; 20]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            })
        });
        let models = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("brush translations"),
            contents: bytemuck::cast_slice(&[[0f32; 4]; 256]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let texture = |width, height, layers, format, label| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: layers,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
        };
        let base = texture(
            TILE as u32,
            TILE as u32,
            scene.textures.len() as u32,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "VTF diffuse array",
        );
        for (layer, pixels) in scene.textures.iter().enumerate() {
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &base,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                pixels,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(TILE as u32 * 4),
                    rows_per_image: Some(TILE as u32),
                },
                wgpu::Extent3d {
                    width: TILE as u32,
                    height: TILE as u32,
                    depth_or_array_layers: 1,
                },
            );
        }
        let light = texture(
            LIGHT_SIZE as u32,
            LIGHT_SIZE as u32,
            1,
            wgpu::TextureFormat::Rgba8Unorm,
            "baked irradiance atlas",
        );
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &light,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &scene.lightmap,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(LIGHT_SIZE as u32 * 4),
                rows_per_image: Some(LIGHT_SIZE as u32),
            },
            wgpu::Extent3d {
                width: LIGHT_SIZE as u32,
                height: LIGHT_SIZE as u32,
                depth_or_array_layers: 1,
            },
        );
        let sampler = |address| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: address,
                address_mode_v: address,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            })
        };
        let repeat = sampler(wgpu::AddressMode::Repeat);
        let clamp = sampler(wgpu::AddressMode::ClampToEdge);
        let buffer_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let tex_entry = |binding, view_dimension| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension,
                multisampled: false,
            },
            count: None,
        };
        let sampler_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Source material layout"),
            entries: &[
                buffer_entry(0),
                buffer_entry(1),
                tex_entry(2, wgpu::TextureViewDimension::D2Array),
                tex_entry(3, wgpu::TextureViewDimension::D2),
                sampler_entry(4),
                sampler_entry(5),
            ],
        });
        let base = base.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let light = light.create_view(&Default::default());
        let bind = std::array::from_fn(|view| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera[view].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: models.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&base),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&light),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&repeat),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(&clamp),
                    },
                ],
            })
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Source diffuse/lightmap"),
            source: wgpu::ShaderSource::Wgsl(include_str!("textured.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let make_pipeline = |blend: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{label:Some("textured Source 1"),layout:Some(&pipeline_layout),vertex:wgpu::VertexState{module:&shader,entry_point:"vs",buffers:&[wgpu::VertexBufferLayout{array_stride:std::mem::size_of::<MeshVertex>() as u64,step_mode:wgpu::VertexStepMode::Vertex,attributes:&wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x2,2=>Float32x2,3=>Uint32,4=>Uint32,5=>Float32,6=>Float32x3]}]},fragment:Some(wgpu::FragmentState{module:&shader,entry_point:if blend {"fs_blend"} else {"fs"},targets:&[Some(wgpu::ColorTargetState{format,blend:if blend {Some(wgpu::BlendState::ALPHA_BLENDING)} else {None},write_mask:wgpu::ColorWrites::ALL})]}),primitive:wgpu::PrimitiveState{cull_mode:None,..Default::default()},depth_stencil:Some(wgpu::DepthStencilState{format:wgpu::TextureFormat::Depth32Float,depth_write_enabled:!blend,depth_compare:wgpu::CompareFunction::Less,stencil:Default::default(),bias:Default::default()}),multisample:Default::default(),multiview:None})
        };
        let pipeline = make_pipeline(false);
        let translucent_pipeline = make_pipeline(true);
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BSP textured vertices"),
            contents: bytemuck::cast_slice(&scene.vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let result = Self {
            pipeline,
            translucent_pipeline,
            translucent: scene
                .vertices
                .chunks_exact(3)
                .enumerate()
                .filter(|(_, tri)| tri[0].unlit < 0.)
                .map(|(i, tri)| {
                    (
                        i as u32 * 3,
                        (Vec3::from(tri[0].position)
                            + Vec3::from(tri[1].position)
                            + Vec3::from(tri[2].position))
                            / 3.,
                        tri[0].model as usize,
                    )
                })
                .collect(),
            matrices: std::array::from_fn(|_| std::cell::Cell::new(Mat4::IDENTITY)),
            origins: std::cell::RefCell::new(scene.origins.clone()),
            bind,
            vertices,
            camera,
            models,
            count: scene.vertices.len() as u32,
        };
        result.update(queue, Mat4::IDENTITY, &scene.origins);
        Ok(result)
    }
    pub fn animation_vertices(&self, queue: &wgpu::Queue, scene: &Scene) {
        for a in &scene.animations {
            if !a.updated {
                continue;
            }
            queue.write_buffer(
                &self.vertices,
                (a.range.start * std::mem::size_of::<MeshVertex>()) as u64,
                bytemuck::cast_slice(&scene.vertices[a.range.clone()]),
            );
        }
    }
    pub fn update(&self, queue: &wgpu::Queue, camera: Mat4, origins: &[Vec3]) {
        *self.origins.borrow_mut() = origins.to_vec();
        self.update_view(queue, 0, camera, glam::Vec4::ZERO);
        let mut models = [[0f32; 4]; 256];
        for (a, p) in models.iter_mut().zip(origins) {
            *a = [p.x, p.y, p.z, 0.];
        }
        queue.write_buffer(&self.models, 0, bytemuck::cast_slice(&models));
    }
    pub fn visibility(&self, queue: &wgpu::Queue, origins: &[Vec3], visible: &[bool]) {
        let mut models = [[0f32; 4]; 256];
        for (n, p) in origins.iter().enumerate() {
            models[n] = [
                p.x,
                p.y,
                p.z,
                if visible.get(n).copied().unwrap_or(true) {
                    0.
                } else {
                    1.
                },
            ];
        }
        queue.write_buffer(&self.models, 0, bytemuck::cast_slice(&models));
    }
    pub fn update_view(&self, queue: &wgpu::Queue, index: usize, matrix: Mat4, clip: glam::Vec4) {
        self.matrices[index].set(matrix);
        let mut data = [0f32; 20];
        data[..16].copy_from_slice(&matrix.to_cols_array());
        data[16..].copy_from_slice(&clip.to_array());
        queue.write_buffer(&self.camera[index], 0, bytemuck::cast_slice(&data));
    }
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.draw_view(pass, 0);
    }
    pub fn draw_view<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, view: usize) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind[view], &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..self.count, 0..1);
        pass.set_pipeline(&self.translucent_pipeline);
        let matrix = self.matrices[view].get();
        let origins = self.origins.borrow();
        let mut triangles: Vec<_> = self
            .translucent
            .iter()
            .map(|&(first, center, model)| {
                let clip = matrix
                    * (center + origins.get(model).copied().unwrap_or(Vec3::ZERO)).extend(1.);
                (first, clip.z / clip.w)
            })
            .collect();
        triangles.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (first, _) in triangles {
            pass.draw(first..first + 3, 0..1);
        }
    }
}
