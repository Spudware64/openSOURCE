use crate::play::{Player, World};
use glam::{Mat4, Vec3};
use std::{sync::Arc, time::Instant};
use wgpu::util::DeviceExt;
use winit::{
    event::{DeviceEvent, ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, WindowBuilder},
};
pub fn interactive(bytes: &[u8]) -> Result<(), String> {
    interactive_impl(bytes, None, None, None)
}
pub fn interactive_scene(bytes: &[u8], scene: crate::scene::Scene) -> Result<(), String> {
    interactive_impl(bytes, Some(scene), None, None)
}
pub fn interactive_portals(
    bytes: &[u8],
    scene: crate::scene::Scene,
    surfaces: crate::portal::Surfaces,
) -> Result<(), String> {
    interactive_impl(bytes, Some(scene), Some(surfaces), None)
}
pub fn interactive_game(
    bytes: &[u8],
    scene: crate::scene::Scene,
    surfaces: Option<crate::portal::Surfaces>,
    root: crate::session::GameRoot,
) -> Result<(), String> {
    interactive_impl(bytes, Some(scene), surfaces, Some(root))
}
fn interactive_impl(
    bytes: &[u8],
    mut scene: Option<crate::scene::Scene>,
    mut surfaces: Option<crate::portal::Surfaces>,
    root: Option<crate::session::GameRoot>,
) -> Result<(), String> {
    let audio = root.as_ref().map(|r| r.audio(bytes)).transpose()?;
    let mut controller_trace = crate::controller_trace::open()?;
    let trace_exe = if controller_trace.is_some() {
        std::env::current_exe()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        String::new()
    };
    let mut trace_map = if controller_trace.is_some() {
        let args: Vec<_> = std::env::args().collect();
        args.windows(2)
            .find(|a| a[0] == "--map")
            .map(|a| a[1].clone())
            .or_else(|| args.get(2).cloned())
            .unwrap_or_default()
    } else {
        String::new()
    };
    let mut movement_audio = crate::audio::MovementAudio::default();
    let mut audio_time = 0.;
    let mut trace_tick = 0u64;
    let mut world = World::parse(bytes)?;
    let mut runtime = crate::entity::Runtime::new(bytes)?;
    let mut portal_game = crate::portal::Game::load(bytes)?;
    portal_game.automatic_surfaces = surfaces.clone();
    if surfaces.is_some() {
        runtime.portal_events = Some(vec![]);
        runtime.direct_launch_loadout();
    }
    let portal_smoke_mode = std::env::var("SOURCE_PORTAL_SMOKE").unwrap_or_default();
    let portal_smoke = !portal_smoke_mode.is_empty();
    if portal_smoke && surfaces.is_none() {
        return Err("SOURCE_PORTAL_SMOKE requires SOURCE_PORTALS=1".into());
    }
    if let Some(scene) = &scene {
        runtime.bind_animations(scene);
        world.bind_scene(scene);
    }
    world.update_models(&runtime.origins, &runtime.visible);
    let cube_smoke = std::env::var_os("SOURCE_CUBE_SMOKE").is_some();
    let progression_smoke = std::env::var_os("SOURCE_PROGRESSION_SMOKE").is_some();
    let cleanser_smoke = std::env::var_os("SOURCE_CLEANSER_SMOKE").is_some();
    let mut chamber_route = std::env::var_os("SOURCE_CHAMBER01_SMOKE").map(|_| {
        let mut route = crate::chamber01_route::Route::default();
        route.full = std::env::var_os("SOURCE_CHAMBER01_FULL").is_some();
        route
    });
    let mut chamber_capture = usize::MAX;
    if chamber_route.is_some()
        && (surfaces.is_none() || runtime.entity_model("portalgun").is_none())
    {
        return Err("chamber01 smoke requires actual chamber01 and SOURCE_PORTALS=1".into());
    }
    let mut cleanser_saw_removed = false;
    let cleanser_cube = if cleanser_smoke {
        let entities = crate::entity::parse_entities(crate::lump(bytes, 0, 1)?)?;
        Some(
            crate::lump(bytes, 14, 48)?.len() / 48
                + entities
                    .iter()
                    .filter(|e| e.get("classname") == Some("prop_physics"))
                    .position(|e| e.get("targetname") == Some("box"))
                    .ok_or("cleanser smoke requires chamber00 box")?,
        )
    } else {
        None
    };
    let mut transitioned_at: Option<u64> = None;
    let parity = std::env::var("SOURCE_PARITY_SMOKE").ok();
    let parity_dir =
        std::env::var("SOURCE_PARITY_CAPTURE").unwrap_or_else(|_| "artifacts/native-parity".into());
    if parity.is_some() {
        std::fs::create_dir_all(&parity_dir).map_err(|e| e.to_string())?;
    }
    // Normal chamber recording: authored spawn and outputs, ordinary walking only.
    let normal_capture = std::env::var("SOURCE_NORMAL_CAPTURE").ok();
    if let Some(dir) = &normal_capture {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut normal_encoder = if let Some(dir) = &normal_capture {
        Some(
            std::process::Command::new("ffmpeg")
                .args([
                    "-y",
                    "-loglevel",
                    "error",
                    "-f",
                    "image2pipe",
                    "-framerate",
                    "30",
                    "-i",
                    "pipe:0",
                    "-c:v",
                    "libx264",
                    "-preset",
                    "fast",
                    "-crf",
                    "23",
                    "-pix_fmt",
                    "yuv420p",
                    "-movflags",
                    "+faststart",
                    &format!("{dir}/../chamber04-normal.mp4"),
                ])
                .stdin(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };
    let mut normal_launched = false;
    let pellet_demo = std::env::var("SOURCE_PELLET_DEMO").ok();
    // Bounded real-window diagnostic; staged camera/inventory, not physical input.
    let shot_demo = std::env::var("SOURCE_SHOT_DEMO").ok();
    if let Some(dir) = &shot_demo { std::fs::create_dir_all(dir).map_err(|e| e.to_string())?; }
    if let Some(dir) = &pellet_demo {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut pellet_crossed = false;
    let mut pellet_caught = false;
    let demo = std::env::var("SOURCE_DOOR_DEMO").ok();
    if let Some(dir) = &demo {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let vertices = crate::load(bytes)?;
    let events = EventLoop::new().map_err(|e| e.to_string())?;
    let window=Arc::new(WindowBuilder::new().with_title("Source 1 exploration | click to look | WASD Space Ctrl crouch E use | Esc release | R reset | N noclip").with_inner_size(if demo.is_some() || pellet_demo.is_some() || normal_capture.is_some() { winit::dpi::Size::Physical(winit::dpi::PhysicalSize::new(1024, 720)) } else { winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(1100., 800.)) }).build(&events).map_err(|e|e.to_string())?);
    if surfaces.is_some() {
        window.set_title("Portal subset | click capture | LMB blue / RMB orange | WASD Space Ctrl E | Esc release R respawn N noclip");
    }
    let instance = wgpu::Instance::default();
    let surface = instance
        .create_surface(window.clone())
        .map_err(|e| e.to_string())?;
    let (device, queue, mut config) = pollster::block_on(async {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .ok_or("no adapter")?;
        println!("interactive adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("interactive"),
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
        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or("no surface config")?;
        Ok::<_, String>((device, queue, config))
    })?;
    config.usage |= wgpu::TextureUsages::COPY_SRC;
    surface.configure(&device, &config);
    let mut textured = scene
        .as_ref()
        .map(|s| crate::scene_gpu::GpuScene::new(&device, &queue, s, config.format))
        .transpose()?;
    // Own GPU scene/uniforms: never upload this camera into linked world views.
    let mut weapon_scene = root.as_ref().map(|r| r.weapon_viewmodel()).transpose()?;
    let weapon_gpu = weapon_scene
        .as_ref()
        .map(|s| crate::scene_gpu::GpuScene::new(&device, &queue, s, config.format))
        .transpose()?;
    let mut weapon_sequence = "draw";
    let mut weapon_started = Instant::now();
    let mut weapon_owned = false;
    let m = Mat4::IDENTITY;
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&m.to_cols_array()),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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
    let make_pipeline = |effects: bool| device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                format: config.format,
                blend: if effects { Some(wgpu::BlendState { color:wgpu::BlendComponent {src_factor:wgpu::BlendFactor::One,dst_factor:wgpu::BlendFactor::One,operation:wgpu::BlendOperation::Add},alpha:wgpu::BlendComponent::OVER }) } else { None },
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: !effects,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview: None,
    });

    let pipeline = make_pipeline(false);
    let shot_pipeline = make_pipeline(true);
    let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let depth_for = |device: &wgpu::Device, w, h| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default())
    };
    let mut depth = depth_for(&device, config.width, config.height);
    let mut portal_gpu = surfaces.as_ref().map(|_| {
        crate::portal_gpu::PortalGpu::new(&device, config.format, config.width, config.height)
    });
    let mut player = Player::new(world.spawn, world.yaw);
    let egui_ctx = egui::Context::default();
    let mut egui_state = egui_winit::State::new(
        egui_ctx.clone(),
        egui::ViewportId::ROOT,
        &*window,
        Some(window.scale_factor() as f32),
        None,
    );
    let mut egui_renderer = egui_wgpu::Renderer::new(&device, config.format, None, 1);
    let gui_smoke = std::env::var_os("SOURCE_GUI_SMOKE").is_some();
    let mut gui_open = gui_smoke;
    let mut maps = root
        .as_ref()
        .map(|r| r.maps())
        .transpose()?
        .unwrap_or_default();
    let mut selected_map = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .find(|a| a[0] == "--map")
        .and_then(|a| std::path::Path::new(&a[1]).file_stem())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if selected_map.is_empty() && !maps.is_empty() {
        selected_map = maps[0].clone();
    }
    let mut map_filter = String::new();
    let mut gui_error = String::new();
    let mut controls = crate::play::Controls::default();
    let mut last = Instant::now();
    let mut accumulator = 0f32;
    let mut render_position = crate::play::RenderPosition::default();
    crate::frame_profile::init();
    let mut frames = 0u64;
    let smoke = std::env::var_os("SOURCE_SMOKE")
        .or_else(|| std::env::var_os("PORTAL_SMOKE"))
        .is_some();
    let stationary_smoke = std::env::var("SOURCE_SMOKE_POSITION")
        .ok()
        .map(|s| crate::entity::vector(&s))
        .transpose()?;
    if let Some(position) = stationary_smoke {
        player.position = position;
    }
    if let Ok(angles) = std::env::var("SOURCE_SMOKE_ANGLES") {
        let a = crate::entity::vector(&angles)?;
        player.yaw = a.y.to_radians();
        player.pitch = -a.x.to_radians();
    }
    if matches!(parity.as_deref(), Some("elevator" | "support-boundary")) {
        player.position = Vec3::new(-1472., -864., 200.);
        player.yaw = if parity.as_deref() == Some("support-boundary") {
            std::f32::consts::FRAC_PI_2
        } else {
            0.
        };
        player.pitch = if parity.as_deref() == Some("support-boundary") {
            -0.45
        } else {
            0.
        };
    }
    if parity.as_deref() == Some("elevator-entry") {
        player.position = Vec3::new(-1360., -864., 144.1);
        player.yaw = std::f32::consts::PI;
        player.pitch = -0.65;
    }
    if parity.as_deref() == Some("dropper") {
        player.position = Vec3::new(-832., -960., 150.);
        player.yaw = -std::f32::consts::FRAC_PI_2;
        player.pitch = 0.55;
        player.noclip = true;
    }
    let initial_models = runtime.origins.clone();
    println!(
        "spawn {:?}, yaw {}, solid brushes {}, triangles {}",
        world.spawn,
        world.yaw,
        world.brushes.len(),
        scene
            .as_ref()
            .map_or(vertices.len() / 3, |s| s.vertices.len() / 3)
    );
    events
        .run(move |event, target| {
            target.set_control_flow(ControlFlow::Poll);
            match event {
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta },
                    ..
                } if controls.captured && !gui_open => {
                    player.yaw -= delta.0 as f32 * 0.002;
                    player.pitch = (player.pitch - delta.1 as f32 * 0.002).clamp(-1.5, 1.5);
                }
                Event::WindowEvent { event, .. } => {
                    if let WindowEvent::KeyboardInput { event: key_event, .. } = &event {
                        if key_event.physical_key == PhysicalKey::Code(KeyCode::F1) && key_event.state == ElementState::Pressed && !key_event.repeat {
                            gui_open = !gui_open;
                            controls.keys.clear();
                            controls.captured = false;
                            let _ = window.set_cursor_grab(CursorGrabMode::None);
                            window.set_cursor_visible(true);
                            window.request_redraw();
                            return;
                        }
                    }
                    if gui_open { let _ = egui_state.on_window_event(&window, &event); }
                    match event {
                    WindowEvent::CloseRequested => target.exit(),
                    WindowEvent::Focused(false) => {
                        controls.keys.clear();
                        controls.captured = false;
                        let _ = window.set_cursor_grab(CursorGrabMode::None);
                        window.set_cursor_visible(true);
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button,
                        ..
                    } if !gui_open && (button==MouseButton::Left || button==MouseButton::Right) => {
                        if controls.captured {
                            let eye=player.position+Vec3::Z*player.eye_height();
                            let channel=usize::from(button==MouseButton::Right);
                            if runtime.portal_inventory[channel] {weapon_sequence="fire1";weapon_started=Instant::now();if let Some(audio)=&audio {audio.play(crate::audio::Event::one(audio_time,if channel==0 {"Weapon_Portalgun.fire_blue"} else {"Weapon_Portalgun.fire_red"},None));}}
                            let forward = player.forward();
                            let right = forward.cross(player.view_up).normalize_or_zero();
                            let muzzle = eye + forward * 24. + right * 8. - player.view_up * 7.;
                            if let Err(error)=portal_game.launch_player(channel,eye,forward,Vec3::new(player.yaw.cos(),player.yaw.sin(),0.),muzzle,&runtime,&world){println!("portal placement rejected: {error}");}
                            return;
                        }
                        controls.captured = window
                            .set_cursor_grab(CursorGrabMode::Locked)
                            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
                            .is_ok();
                        window.set_cursor_visible(!controls.captured);
                        println!("cursor controls.captured: {}", controls.captured);
                    }
                    WindowEvent::KeyboardInput { event, .. } if !gui_open => {
                        if let PhysicalKey::Code(key) = event.physical_key {
                            if event.state == ElementState::Pressed {
                                controls.keys.insert(key);
                                if !event.repeat {
                                    match controls.press(key) {
                                        crate::play::Action::ReleaseCursor => {
                                            controls.captured = false;
                                            controls.keys.clear();
                                            let _ = window.set_cursor_grab(CursorGrabMode::None);
                                            window.set_cursor_visible(true);
                                            println!("cursor released");
                                        }
                                        crate::play::Action::Reset => {
                                            portal_game.cancel_shots();
                                            player = Player::new(world.spawn, world.yaw);
                                            render_position = crate::play::RenderPosition::default();
                                            println!("reset");
                                        }
                                        crate::play::Action::Throw => { if controls.captured { runtime.throw_held(&player); } }
                                        crate::play::Action::Use => {
                                            if controls.captured {
                                                println!(
                                                    "use {}",
                                                    {let used=runtime.use_player(&player, &world); if let Some(audio)=&audio {audio.play(crate::audio::Event::one(audio_time,if used {"HL2Player.Use"} else {"HL2Player.UseDeny"},None));} used}
                                                );
                                            }
                                        }
                                        crate::play::Action::ToggleNoclip => {
                                            player.noclip = !player.noclip;
                                            println!("noclip {}", player.noclip);
                                        }
                                        _ => (),
                                    }
                                }
                            } else {
                                controls.keys.remove(&key);
                            }
                        }
                    }
                    WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                        config.width = size.width;
                        config.height = size.height;
                        surface.configure(&device, &config);
                        depth = depth_for(&device, config.width, config.height);
                        if surfaces.is_some() {portal_gpu=Some(crate::portal_gpu::PortalGpu::new(&device,config.format,config.width,config.height));}
                    }
                    WindowEvent::RedrawRequested => {
                        crate::frame_profile::begin();
                        let now = Instant::now();
                        let trace_frame_dt = (now - last).as_secs_f32();
                        accumulator += trace_frame_dt.min(0.1);
                        last = now;
                        let down = |k| {
                            if controls.keys.contains(&k) && controls.captured {
                                1.
                            } else {
                                0.
                            }
                        };
                        let input = Vec3::new(
                            down(KeyCode::KeyW) - down(KeyCode::KeyS),
                            down(KeyCode::KeyD) - down(KeyCode::KeyA),
                            down(KeyCode::Space) - down(KeyCode::ShiftLeft),
                        );
                        if shot_demo.is_some() {
                            if frames == 0 {
                                player = Player::new(Vec3::new(-900.,-224.,576.), std::f32::consts::PI);
                                runtime.portal_inventory = [true;2];
                                portal_game.portals = [None;2]; world.portals = [None;2];
                            }
                            if frames == 90 { player.yaw = std::f32::consts::PI + 0.45; }
                            if frames == 30 || frames == 90 {
                                let channel = usize::from(frames == 90);
                                let eye = player.position + Vec3::Z * player.eye_height();
                                let forward = player.forward();
                                let muzzle = eye + forward*24. + forward.cross(player.view_up).normalize_or_zero()*8. - player.view_up*7.;
                                portal_game.launch_player(channel,eye,forward,forward,muzzle,&runtime,&world).unwrap();
                                weapon_sequence = "fire1"; weapon_started = Instant::now();
                                println!("SHOT_DEMO launch {channel} {:?}",portal_game.shots);
                            }
                            portal_game.advance_shots(1./60., &runtime, &mut world);
                            accumulator = 0.;
                        } else if let Some(route)=chamber_route.as_mut() {
                            for _ in 0..4 {match route.tick(&mut player,&mut runtime,&mut world,&mut portal_game) {
                                Ok(true)=>break,Ok(false)=>{},Err(error)=>{eprintln!("{error}");target.exit();return;}
                            }}
                            accumulator=0.;
                        } else if cleanser_smoke && frames<=75 {
                            let cube=cleanser_cube.unwrap();
                            if frames==0 {
                                player=Player::new(Vec3::new(-1160.,-864.,128.1),std::f32::consts::PI);
                                runtime.origins[cube]=Vec3::new(-1224.,-864.,180.);
                                assert!(runtime.use_player(&player,&world),"must pick up actual cube before crossing cleanser");
                                let s=surfaces.as_ref().expect("cleanser smoke requires SOURCE_PORTALS=1");
                                portal_game.place(0,s.place(Vec3::new(-416.,-320.,178.),-Vec3::Z).unwrap());
                                portal_game.place(1,s.place(Vec3::new(-1088.,-600.,640.),-Vec3::Y).unwrap());
                            }
                            for _ in 0..4 {
                                runtime.tick(1./120.,&mut player,&mut world);portal_game.sync_runtime(&mut runtime,&mut world);
                                if !runtime.visible[cube] {cleanser_saw_removed=true;}
                                player.step(&world,if frames<40 {Vec3::X}else{Vec3::ZERO},false,1./120.);
                            }
                            if frames==60 {
                                println!("cleanser target slot {cube} visible {} origin {:?}; unknown {:?}",runtime.visible[cube],runtime.origins[cube],runtime.unknown_inputs);
                                assert!(cleanser_saw_removed,"held cube must become invisible at dissolution before authored maker may reuse the slot");
                                assert!(!runtime.visible[cube] || runtime.origins[cube].x> -1000.,"a replacement must be at the authored dropper, never survive at the cleanser");
                                assert!(world.portals.iter().all(Option::is_none));
                                assert!(!runtime.throw_held(&player),"dissolved held handle must clear");
                                assert!(!runtime.is_enabled("show_drop_hudhint"),"authored OnDissolveBox output");
                                assert!(player.position.x< -1280.,"must walk across actual cleanser {:?}",player.position);
                                player.yaw=0.;println!("cleanser smoke verified held cube removal, handle reset, portal fizzle, actual output; player {:?}",player.position);
                            }
                            accumulator=0.;
                        } else if progression_smoke && transitioned_at.is_none_or(|f| frames <= f+300) {
                            if frames==0 {
                                player=Player::new(Vec3::new(-384.,-256.,600.),std::f32::consts::PI);
                                println!("progression smoke: start on actual final elevator; no direct ChangeLevel input");
                            }
                            for _ in 0..4 {
                                runtime.tick(1./120.,&mut player,&mut world);
                                portal_game.sync_runtime(&mut runtime,&mut world);
                                let walk = if transitioned_at.is_some_and(|f| frames>f+180 && frames<f+240) {Vec3::X} else {Vec3::ZERO};
                                player.step(&world,walk,false,1./120.);
                                if runtime.change_level.is_some() {break;}
                            }
                            if transitioned_at.is_some_and(|f| frames==f+240) {player.yaw=0.;}
                            accumulator=0.;
                            assert!(frames<1800 || transitioned_at.is_some(),"authored final lift never requested ChangeLevel");
                        } else if cube_smoke && frames <= 520 {
                            if frames==0 { player=Player::new(Vec3::new(-740.,-1056.,128.1),std::f32::consts::PI);player.pitch = -0.45;runtime.input("logic_drop_box","Trigger",""); }
                            if frames==120 {assert!(runtime.use_player(&player,&world),"cube pickup failed");}
                            for _ in 0..4 {
                                let mut walk=Vec3::ZERO;
                                if (125..215).contains(&frames) {
                                    let delta=Vec3::new(-572.,-928.,player.position.z)-player.position;
                                    player.yaw=delta.y.atan2(delta.x);
                                    if delta.truncate().length()>2. {walk=Vec3::X;}
                                }
                                if frames>=215 {player.yaw=0.;player.pitch = -0.6;}
                                if (280..460).contains(&frames) {
                                    let target = if frames < 370 {Vec3::new(-970.,-864.,player.position.z)} else {Vec3::new(-572.,-928.,player.position.z)};
                                    let delta=target-player.position;
                                    player.yaw=delta.y.atan2(delta.x);player.pitch=0.;
                                    if delta.truncate().length()>2. {walk=Vec3::X;}
                                }
                                runtime.tick(1./120.,&mut player,&mut world);
                                player.step(&world,walk,false,1./120.);
                            }
                            if frames==220 {assert!(runtime.use_player(&player,&world),"cube drop failed");}
                            if frames==369 {assert!(player.position.truncate().distance(Vec3::new(-970.,-864.,0.).truncate())<3.,"must walk through open exit");}
                            if frames==465 {assert!(runtime.use_player(&player,&world),"pick cube back up off plate");}
                            accumulator=0.;
                        } else if portal_smoke && frames<=180 {
                            if frames==30 && portal_smoke_mode!="floor" {runtime.input("relay_portal_cancel_room1","Trigger","");}
                            if frames==0 {player.position=if portal_smoke_mode=="floor" {Vec3::new(-416.,-320.,208.)}else{Vec3::new(-740.,-320.,144.)};player.yaw=if portal_smoke_mode=="floor" {0.}else{std::f32::consts::PI};player.pitch=if portal_smoke_mode=="floor" {-1.}else{0.};}
                            for _ in 0..4 {runtime.tick(1./120.,&mut player,&mut world);portal_game.apply_events(runtime.portal_events.as_mut().unwrap());
                                if portal_smoke_mode=="floor" && frames==30 {let s=surfaces.as_ref().unwrap();portal_game.portals=[Some(s.place(Vec3::new(-416.,-320.,178.),-Vec3::Z).unwrap()),Some(s.place(Vec3::new(-1088.,-600.,640.),-Vec3::Y).unwrap())];}
                                world.portals=portal_game.portals;player.step(&world,if portal_smoke_mode!="floor" && frames>60 && frames<100 {Vec3::X}else{Vec3::ZERO},false,1./120.);}
                            accumulator=0.;
                        } else if let Some(kind) = parity.as_ref().filter(|kind| frames <= if kind.as_str()=="dropper" {220} else {570}) {
                            if kind=="dropper" {
                                 if frames==30 {runtime.input("logic_drop_box","Trigger","");}
                                if frames==140 {runtime.input("box_dropper_model_cover","SetAnimation","close");}
                            }
                            for _ in 0..4 {
                                runtime.tick(1./120.,&mut player,&mut world);
                                if kind=="elevator-entry" {
                                    player.step(&world, if frames>=10 && player.position.x > -1470. {Vec3::X} else {Vec3::ZERO}, false, 1./120.);
                                    if frames>40 {assert!(player.grounded && player.position.z > runtime.origins[37].z-68., "entry support {:?}", player.position);}
                                }
                                if kind=="elevator" || kind=="support-boundary" {
                                    let walk = if kind=="support-boundary" {(90..105).contains(&frames)} else {frames>510};
                                    player.step(&world,if walk {Vec3::X} else {Vec3::ZERO},false,1./120.);
                                }
                            }
                            accumulator=0.;
                        } else if normal_capture.is_some() {
                            // Four 120 Hz gameplay steps per 30 fps presented frame.
                            // Walk out of the real entry and across the authored launcher trigger.
                            for _ in 0..4 {
                                let walk = if player.position.x < -370. && frames < 150 {
                                    player.yaw = 0.;
                                    Vec3::X
                                } else {
                                    let eye = player.position + Vec3::Z * player.eye_height();
                                    let target = if frames < 360 { Vec3::new(-170.,318.,61.46) } else { Vec3::new(-1210.,884.,-55.) };
                                    let d = target-eye;
                                    let desired = d.y.atan2(d.x);
                                    let delta = (desired-player.yaw).sin().atan2((desired-player.yaw).cos());
                                    player.yaw += delta.clamp(-0.012,0.012);
                                    let pitch = d.z.atan2(d.truncate().length());
                                    player.pitch += (pitch-player.pitch).clamp(-0.008,0.008);
                                    Vec3::ZERO
                                };
                                runtime.tick(1./120., &mut player, &mut world);
                                portal_game.sync_runtime(&mut runtime,&mut world);
                                player.step(&world,walk,false,1./120.);
                                if !runtime.pellets.is_empty() { normal_launched=true; }
                            }
                            if frames%30==0 {println!("NORMAL_CAPTURE frame {frames} player {:?} pellets {:?}",player.position,runtime.pellets);}
                            accumulator=0.;
                        } else if pellet_demo.is_some() {
                            // Diagnostic portal placements, not a solved authored chamber route.
                            // No projectile position/velocity is ever written by this driver.
                            if frames==0 {
                                world.portals=[Some(crate::portal::Portal::new(Vec3::new(-100.,318.,61.46),-Vec3::X,Vec3::Z)),Some(crate::portal::Portal::new(Vec3::new(-1360.,878.,-56.),Vec3::X,Vec3::Z))];
                                player.position=Vec3::new(-200.,50.,66.);player.yaw=std::f32::consts::FRAC_PI_2;player.pitch=-0.25;
                                runtime.input("energy_ball_launcher_lvl5","Disable","");
                            }
                            if frames==30 {runtime.input("energy_ball_launcher_lvl5","Enable","");}
                            // One genuine 120 Hz step per video frame: quarter-speed playback.
                            runtime.tick(1./120.,&mut player,&mut world);
                            if !pellet_crossed && runtime.pellets.first().is_some_and(|p|p.position.x < -1000.) {
                                pellet_crossed=true;println!("PELLET_DEMO portal-cross frame {frames} {:?}",runtime.pellets);
                            }
                            if pellet_crossed {
                                player.position=Vec3::new(-1320.,660.,60.);player.yaw=1.40;player.pitch=-0.55;
                            }
                            if !pellet_caught && !runtime.is_enabled("ball_trap_10") {
                                pellet_caught=true;println!("PELLET_DEMO receiver-output frame {frames}; launcher enabled {}; pellets {:?}",runtime.is_enabled("energy_ball_launcher_lvl5"),runtime.pellets);
                            }
                            println!("PELLET_DEMO frame {frames} {:?}",runtime.pellets);
                            accumulator=0.;
                        } else if demo.is_some() {
                            if let Some(command) = match frames {
                                0 => Some("Close"),
                                30 => Some("Open"),
                                105 => Some("Close"),
                                _ => None,
                            } {
                                for name in ["exit_door_right_a00", "exit_door_left_a00"] {
                                    runtime.input(name, command, "");
                                }
                                println!("door demo frame {frames} input {command}");
                            }
                            // The fixed camera is outside the floor-button trigger. Four real
                            // 120 Hz entity simulation steps per independently rendered 30 Hz frame.
                            for _ in 0..4 {
                                runtime.tick(1. / 120., &mut player, &mut world);
                                portal_game.advance_shots(1. / 120., &runtime, &mut world);
                            }
                            accumulator = 0.;
                            println!(
                                "door demo frame {frames} origins {:?} {:?}",
                                runtime.origins[16], runtime.origins[17]
                            );
                        } else if smoke && frames < 600 {
                            if frames == 0 {
                                controls.captured = window
                                    .set_cursor_grab(CursorGrabMode::Locked)
                                    .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
                                    .is_ok();
                                window.set_cursor_visible(!controls.captured);
                                println!("smoke native cursor grab {}", controls.captured);
                            }
                            runtime.tick(1. / 120., &mut player, &mut world);
                            portal_game.advance_shots(1. / 120., &runtime, &mut world);
                            if stationary_smoke.is_some() {
                                player.step(&world, Vec3::ZERO, false, 1. / 120.);
                            } else {
                                crate::play::smoke_tick(frames, &mut player, &world);
                            }
                            accumulator = 0.;
                            if [119, 359, 390, 599].contains(&frames) {
                                println!(
                                    "smoke frame {} position {:?} grounded {}",
                                    frames, player.position, player.grounded
                                );
                            }
                        } else {
                            while accumulator >= 1. / 120. {
                                render_position.begin_step(&player);
                                player.set_crouch(&world, down(KeyCode::ControlLeft) > 0.);
                                let trace_before = controller_trace.as_ref().map(|_| {
                                    let support = if player.grounded && !player.noclip && player.velocity.z <= 0.1 { world.support_model(player.position, player.height()) } else { None };
                                    let resolved = runtime.model_transforms().0;
                                    (player.position, support, support.and_then(|i| runtime.origins.get(i).copied()), support.and_then(|i| resolved.get(i).copied()))
                                });
                                runtime.tick(1. / 120., &mut player, &mut world);
                                portal_game.advance_shots(1. / 120., &runtime, &mut world);
                                portal_game.sync_runtime(&mut runtime,&mut world);
                                let trace_runtime_position = player.position;
                                player.step(&world, input, down(KeyCode::Space) > 0., 1. / 120.);
                                audio_time=runtime.sound_time();
                                if let Some(audio)=&audio {for event in movement_audio.step(&player,audio_time){audio.play(event);}}
                                accumulator -= 1. / 120.;
                                crate::controller_trace::record(&mut controller_trace, || {
                                    use crate::controller_trace::csv;
                                    let (before, support, raw_before, resolved_before) = trace_before.unwrap();
                                    let resolved = runtime.model_transforms().0;
                                    let support_after = world.support_model(player.position, player.height());
                                    let raw_after = support.and_then(|i| runtime.origins.get(i).copied());
                                    let resolved_after = support.and_then(|i| resolved.get(i).copied());
                                    let alpha = (accumulator * 120.).clamp(0., 1.);
                                    let fields = vec![trace_tick.to_string(), frames.to_string(), (1f32/120.).to_string(), trace_frame_dt.to_string(), csv(&trace_map), csv(&trace_exe), csv(env!("SOURCE_BUILD_GIT")), csv(&format!("{:?}", player.position)), csv(&format!("{:?}", player.velocity)), player.grounded.to_string(), player.crouched.to_string(), player.eye_height().to_string(), csv(&format!("{:?}", player.position + Vec3::Z * player.eye_height())), csv(&format!("{:?}", render_position.sample(&player, alpha) + Vec3::Z * player.eye_height())), alpha.to_string(), csv(&format!("{input:?}")), (down(KeyCode::Space)>0.).to_string(), (down(KeyCode::ControlLeft)>0.).to_string(), controls.captured.to_string(), player.noclip.to_string(), player.yaw.to_string(), player.pitch.to_string(), csv(&format!("{support:?}")), csv(&format!("{support_after:?}")), csv(&format!("{raw_before:?}")), csv(&format!("{raw_after:?}")), csv(&format!("{resolved_before:?}")), csv(&format!("{resolved_after:?}")), csv(&format!("{:?}", trace_runtime_position-before))];
                                    trace_tick += 1;
                                    // Diagnostic-only downward sweep; not a controller decision.
                                    let (drop_fraction, drop_normal) = world.trace_height(player.position, player.position - Vec3::Z * 18., player.height());
                                    let mut fields = fields;
                                    fields.extend([csv(&format!("{before:?}")), csv(&format!("{trace_runtime_position:?}")), csv(&format!("{:?}", player.position - trace_runtime_position)), player.height().to_string(), drop_fraction.to_string(), csv(&format!("{drop_normal:?}")), csv(&format!("{:?}", support_after.and_then(|i| runtime.origins.get(i).copied()))), csv(&format!("{:?}", support_after.and_then(|i| resolved.get(i).copied())))]);
                                    fields.join(",")
                                });
                            }
                        }
                        if let Some(audio)=&audio {
                            audio.listener(player.position+Vec3::Z*player.eye_height(),Vec3::new(player.yaw.sin(),-player.yaw.cos(),0.));
                            for event in runtime.sound_events.drain(..) {audio.play(event);}
                        }
                        if pellet_demo.is_none() { portal_game.sync_runtime(&mut runtime, &mut world); }
                        if let Some(request) = runtime.change_level.take() {
                            println!("ChangeLevel detected: {} landmark {} player {:?}",request.map,request.landmark,player.position);
                            let prepared = root.as_ref().ok_or_else(|| "ChangeLevel needs --game asset root (legacy window cannot transition)".to_string()).and_then(|r|r.prepare(&request,&player,surfaces.is_some()));
                            match prepared {
                                Ok(mut next) => {
                                    for warning in &next.scene.warnings {eprintln!("ChangeLevel asset warning: {warning}");}
                                    if let Some(audio)=&audio {let mut event=crate::audio::Event::one(audio_time,"",None);event.stop=true;audio.play(event);}
                                    movement_audio=crate::audio::MovementAudio::default();
                                    next.runtime.enter_transition(&runtime,next.player.position-player.position);
                                    next.world.update_models(&next.runtime.origins,&next.runtime.visible);
                                    let gpu=match crate::scene_gpu::GpuScene::new(&device,&queue,&next.scene,config.format) {
                                        Ok(gpu)=>gpu,
                                        Err(e)=> {eprintln!("ChangeLevel GPU preparation failed: {e}");target.exit();return;}
                                    };
                                    device.poll(wgpu::Maintain::Wait);
                                    // Old scene buffers/textures/bind groups are dropped together after submitted work completes.
                                    drop(textured.take()); drop(scene.take());
                                    textured=Some(gpu);scene=Some(next.scene);world=next.world;runtime=next.runtime;
                                    if controller_trace.is_some() { trace_map = request.map.clone(); }
                                    selected_map = request.map.clone();
                                    player=next.player;portal_game=next.game;surfaces=next.surfaces;
                                    render_position = crate::play::RenderPosition::default();
                                    controls=crate::play::Controls::default();accumulator=0.;last=Instant::now();
                                    transitioned_at=Some(frames);
                                    let _=window.set_cursor_grab(CursorGrabMode::None);window.set_cursor_visible(true);
                                    window.set_title(&format!("Source 1 | {} | click capture | WASD Space Ctrl E | LMB/RMB portals | Esc release",request.map));
                                    println!("ChangeLevel committed: {} player {:?}; old GPU scene dropped; held bodies / keys / capture / portals reset",request.map,player.position);
                                }
                                Err(e)=> {eprintln!("ChangeLevel failed explicitly: {e}");target.exit();return;}
                            }
                        }
                        if portal_smoke && portal_smoke_mode!="floor" && frames==120 {player.yaw= -std::f32::consts::FRAC_PI_2;player.pitch=0.;}
                        if parity.as_deref()==Some("elevator") && frames==570 {player.yaw=std::f32::consts::FRAC_PI_2;}
                        if smoke && frames == 600 {
                            let action = controls.press(KeyCode::Escape);
                            assert_eq!(action, crate::play::Action::ReleaseCursor);
                            let released = window.set_cursor_grab(CursorGrabMode::None);
                            window.set_cursor_visible(true);
                            let moved = runtime
                                .origins
                                .iter()
                                .zip(&initial_models)
                                .enumerate()
                                .filter(|(_, (a, b))| a.distance(**b) > 0.01)
                                .map(|(n, (a, _))| (n, *a))
                                .collect::<Vec<_>>();
                            println!(
                                "smoke moved brush models {:?}; unsupported entity inputs {:?}",
                                moved, runtime.unknown_inputs
                            );
                            println!(
                                "smoke native cursor release {:?}; grounded {} position {:?}",
                                released, player.grounded, player.position
                            );
                        }
                        if player.position.z < world.spawn.z - 2000. {
                            portal_game.cancel_shots();
                            player = Player::new(world.spawn, world.yaw);
                            render_position = crate::play::RenderPosition::default();
                        }
                        // Scripted routes retain exact deterministic capture cameras.
                        // The interactive camera interpolates translation only; mouse
                        // look remains immediate and portal transfers snap atomically.
                        let position = if smoke || parity.is_some() || portal_smoke || pellet_demo.is_some() || normal_capture.is_some() {
                            player.position
                        } else {
                            render_position.sample(&player, accumulator * 120.)
                        };
                        let eye = position + Vec3::Z * player.eye_height();
                        let m = Mat4::perspective_rh(
                            75f32.to_radians(),
                            config.width as f32 / config.height as f32,
                            1.,
                            20000.,
                        ) * Mat4::look_at_rh(eye, eye + player.forward(), player.view_up);
                        crate::frame_profile::mark(0);
                        queue.write_buffer(&uniform, 0, bytemuck::cast_slice(&m.to_cols_array()));
                        if let (Some(scene), Some(gpu)) = (&mut scene, &textured) {
                            if let Err(error) = scene.animate(&runtime) {
                                eprintln!("animation update: {error}");
                                target.exit();
                                return;
                            }
                            crate::frame_profile::mark(1);
                            gpu.animation_vertices(&queue, scene);
                            gpu.update_pellets(&queue, &runtime.pellets);
                            crate::frame_profile::mark(2);
                        }
                        if let Some(gpu) = &textured {
                            gpu.update(&queue, m, &runtime.origins);
                            gpu.visibility(&queue, &runtime.origins, &runtime.visible);
                        }
                        if let (Some(portal_gpu),Some(gpu))=(&mut portal_gpu,&textured) {
                            portal_gpu.update(&queue,world.portals,eye,m);
                            if let [Some(a),Some(b)]=world.portals {for (index,(src,dst)) in [(a,b),(b,a)].into_iter().enumerate() {
                                if !portal_gpu.needs_remote(index) {continue;}
                                let (e,f,u,clip)=src.linked_view(dst,eye,player.forward(),player.view_up);
                                let matrix=Mat4::perspective_rh(75f32.to_radians(),config.width as f32/config.height as f32,0.1,20000.)*Mat4::look_at_rh(e,e+f,u);
                                gpu.update_view(&queue,index+1,matrix,clip);
                            }}
                        }
                        crate::frame_profile::mark(3);
                        let frame = match surface.get_current_texture() {
                            Ok(f) => f,
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                target.exit();
                                return;
                            }
                            Err(_) => {
                                surface.configure(&device, &config);
                                return;
                            }
                        };
                        crate::frame_profile::mark(4);
                        let shot_vertices = portal_game.shot_vertices(eye);
                        let shot_buffer = (!shot_vertices.is_empty()).then(|| device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label:Some("travelling portal energy ribbons"),contents:bytemuck::cast_slice(&shot_vertices),usage:wgpu::BufferUsages::VERTEX }));
                        let cv = frame.texture.create_view(&Default::default());
                        let dv = &depth;
                        let mut enc = device.create_command_encoder(&Default::default());
                        if let (Some(portals),Some(gpu))=(&portal_gpu,&textured) {if world.portals.iter().all(Option::is_some) {for index in 0..2 {portals.remote(&mut enc,gpu,index);}}}
                        {
                            crate::frame_profile::mark(5);
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
                                depth_stencil_attachment: Some(
                                    wgpu::RenderPassDepthStencilAttachment {
                                        view: dv,
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
                            if let Some(gpu) = &textured {
                                gpu.draw(&mut pass);
                                if let Some(portals)=&portal_gpu {portals.draw(&mut pass);}
                            } else {
                                pass.set_pipeline(&pipeline);
                                pass.set_bind_group(0, &bind, &[]);
                                pass.set_vertex_buffer(0, vb.slice(..));
                                pass.draw(0..vertices.len() as u32, 0..1);
                            }
                            if let Some(buffer) = &shot_buffer {
                                pass.set_pipeline(&shot_pipeline);
                                pass.set_bind_group(0, &bind, &[]);
                                pass.set_vertex_buffer(0, buffer.slice(..));
                                pass.draw(0..shot_vertices.len() as u32, 0..1);
                            }
                        }

                        let owned=runtime.portal_inventory.iter().any(|owned| *owned);
                        if owned && !weapon_owned {weapon_sequence="draw";weapon_started=Instant::now();}
                        weapon_owned=owned;
                        if std::env::var_os("SOURCE_WEAPON_DEMO").is_some() && frames==150 {weapon_sequence="fire1";weapon_started=Instant::now();}
                        if owned {
                            if let (Some(s),Some(gpu))=(&mut weapon_scene,&weapon_gpu) {
                                if let Err(e)=s.pose_weapon(weapon_sequence,weapon_started.elapsed().as_secs_f32()) {eprintln!("weapon animation: {e}");target.exit();return;}
                                gpu.animation_vertices(&queue,s);
                            }
                            if let Some(gpu) = &weapon_gpu {
                                // Source authored viewmodel: X forward, Y left, Z up.
                                // 54 degree horizontal FOV at Source's reference 4:3.
                                let projection = Mat4::perspective_rh(2. * ((54f32.to_radians()/2.).tan() / (4./3.)).atan(), config.width as f32/config.height as f32, 0.1, 200.);
                                gpu.update(&queue, projection * Mat4::look_at_rh(Vec3::ZERO, Vec3::X, Vec3::Z), &[Vec3::ZERO]);
                                let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("main camera ownership-gated weapon"),
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment { view:&cv, resolve_target:None, ops:wgpu::Operations { load:wgpu::LoadOp::Load, store:wgpu::StoreOp::Store } })],
                                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view:dv, depth_ops:Some(wgpu::Operations { load:wgpu::LoadOp::Clear(1.), store:wgpu::StoreOp::Discard }), stencil_ops:None }),
                                    timestamp_writes:None, occlusion_query_set:None,
                                });
                                gpu.draw(&mut pass);
                            }
                        }
                        let mut requested_map = None;
                        if gui_open {
                            let raw = egui_state.take_egui_input(&window);
                            let output = egui_ctx.run(raw, |ctx| {
                                egui::Window::new("openSOURCE debug").default_width(370.).show(ctx, |ui| {
                                    ui.label("F1 closes this panel. Click the game to recapture mouse.");
                                    ui.separator();
                                    ui.heading("Map");
                                    if root.is_some() {
                                        ui.horizontal(|ui| {
                                            ui.label(format!("{} available", maps.len()));
                                            if ui.button("Refresh").clicked() {
                                                match root.as_ref().unwrap().maps() { Ok(found) => maps = found, Err(e) => gui_error = e }
                                            }
                                        });
                                        ui.text_edit_singleline(&mut map_filter);
                                        egui::ScrollArea::vertical().max_height(150.).show(ui, |ui| {
                                            for name in &maps {
                                                if name.contains(&map_filter.to_ascii_lowercase()) && ui.selectable_label(selected_map == *name, name).clicked() {
                                                    selected_map = name.clone();
                                                }
                                            }
                                        });
                                        if ui.button("Load / reload selected map").clicked() { requested_map = Some(selected_map.clone()); }
                                    } else { ui.label("Map loading requires --game and --map."); }
                                    if !gui_error.is_empty() { ui.colored_label(egui::Color32::RED, &gui_error); }
                                    ui.separator();
                                    ui.heading("Player");
                                    ui.label(format!("Position: {:.1}, {:.1}, {:.1}", player.position.x, player.position.y, player.position.z));
                                    ui.label(format!("Velocity: {:.1}, {:.1}, {:.1} | grounded {}", player.velocity.x, player.velocity.y, player.velocity.z, player.grounded));
                                    ui.label(format!("Yaw {:.2} | pitch {:.2}", player.yaw, player.pitch));
                                    if ui.button("Respawn (R)").clicked() { player = Player::new(world.spawn, world.yaw); render_position = crate::play::RenderPosition::default(); }
                                    ui.checkbox(&mut player.noclip, "Noclip (N)");
                                    ui.separator();
                                    ui.heading("Renderer / runtime");
                                    ui.label(format!("Frame {} | {:.1} ms | {:.0} FPS", frames, trace_frame_dt * 1000., 1. / trace_frame_dt.max(0.0001)));
                                    ui.label(format!("Surface {}x{} {:?}", config.width, config.height, config.format));
                                    ui.label(format!("Triangles {} | materials {} | lightmapped faces {}", scene.as_ref().map_or(vertices.len()/3, |s| s.vertices.len()/3), scene.as_ref().map_or(0, |s| s.textures.len()), scene.as_ref().map_or(0, |s| s.lightmapped_faces)));
                                    ui.label(format!("Brushes {} | models {} | visible {}", world.brushes.len(), runtime.origins.len(), runtime.visible.iter().filter(|v| **v).count()));
                                    ui.label(format!("Portals {}/2 | portal inventory {:?} | pellets {}", world.portals.iter().filter(|p| p.is_some()).count(), runtime.portal_inventory, runtime.pellets.len()));
                                    ui.label(format!("Unknown entity inputs {}", runtime.unknown_inputs.len()));
                                });
                            });
                            egui_state.handle_platform_output(&window, output.platform_output);
                            let pixels_per_point = window.scale_factor() as f32;
                            let paint_jobs = egui_ctx.tessellate(output.shapes, pixels_per_point);
                            let screen = egui_wgpu::ScreenDescriptor { size_in_pixels: [config.width, config.height], pixels_per_point };
                            for (id, delta) in &output.textures_delta.set { egui_renderer.update_texture(&device, &queue, *id, delta); }
                            let extra = egui_renderer.update_buffers(&device, &queue, &mut enc, &paint_jobs, &screen);
                            for buffer in extra { queue.submit(Some(buffer)); }
                            {
                                let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("egui overlay"),
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &cv, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
                                    depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None,
                                });
                                egui_renderer.render(&mut pass, &paint_jobs, &screen);
                            }
                            for id in &output.textures_delta.free { egui_renderer.free_texture(id); }
                        }
                        crate::frame_profile::mark(6);
                        queue.submit(Some(enc.finish()));
                        if let Some(dir) = &shot_demo {
                            if [0,30,32,36,42,55,90,92,96,102,120].contains(&frames) {
                                let path = format!("{dir}/shot-{frames:04}.png");
                                capture_surface(&device,&queue,&frame.texture,config.width,config.height,config.format,&path).unwrap();
                                println!("SHOT_DEMO frame {frames} pending {} portals {:?} capture {path}",portal_game.shots.len(),world.portals);
                            }
                            if frames == 120 { assert!(portal_game.shots.is_empty()); target.exit(); }
                        }
                        if std::env::var_os("SOURCE_WEAPON_DEMO").is_some() && [30,120,150,160,175,220].contains(&frames) {
                            let path=format!("evidence/weapon-{frames:04}.png");
                            if let Err(e)=capture_surface(&device,&queue,&frame.texture,config.width,config.height,config.format,&path){eprintln!("weapon capture: {e}");}
                            if frames==220 {target.exit();}
                        }
                        crate::frame_profile::mark(7);
                        if frames == if smoke { 600 } else { 120 } {
                            if let Ok(path) = std::env::var("SOURCE_SCREENSHOT")
                                .or_else(|_| std::env::var("PORTAL_SCREENSHOT"))
                            {
                                match capture_surface(
                                    &device,
                                    &queue,
                                    &frame.texture,
                                    config.width,
                                    config.height,
                                    config.format,
                                    &path,
                                ) {
                                    Ok(()) => println!("surface screenshot: {}", path),
                                    Err(e) => eprintln!("screenshot failed: {}", e),
                                }
                            }
                        }
                        if let Some(dir) = &normal_capture {
                            use std::io::Write;
                            let path=format!("{dir}/current.png");
                            capture_surface(&device,&queue,&frame.texture,config.width,config.height,config.format,&path).unwrap();
                            let encoder=normal_encoder.as_mut().unwrap();
                            encoder.stdin.as_mut().unwrap().write_all(&std::fs::read(&path).unwrap()).unwrap();
                            if frames==449 {
                                drop(encoder.stdin.take());
                                assert!(encoder.wait().unwrap().success(),"video encoding failed");
                                assert!(normal_launched,"authored trigger never launched pellet");
                                println!("NORMAL_CAPTURE complete: authored spawn, normal walking, natural launch; no puzzle-solve claim");target.exit();
                            }
                        }
                        if let Some(dir) = &pellet_demo {
                            let path=format!("{dir}/frame_{frames:04}.png");
                            capture_surface(&device,&queue,&frame.texture,config.width,config.height,config.format,&path).unwrap();
                            if frames==359 {assert!(pellet_crossed && pellet_caught,"continuous pellet demo did not reach receiver");println!("PELLET_DEMO VERIFIED continuous launcher -> portal -> authored receiver");target.exit();}
                        }
                        if let Some(dir) = &demo {
                            let path = format!("{dir}/frame_{frames:04}.png");
                            if let Err(e) = capture_surface(
                                &device,
                                &queue,
                                &frame.texture,
                                config.width,
                                config.height,
                                config.format,
                                &path,
                            ) {
                                eprintln!("door demo capture failed: {e}");
                                target.exit();
                                return;
                            }
                        }
                        if let Some(route)=chamber_route.as_ref() {
                            let capture_phase=if route.phase==5 && world.portals[0].is_some_and(|p|p.center.y>570.) {40}else{route.phase};
                            if chamber_capture!=capture_phase && [0,5,6,7,8,10,12,15,16,17,19,21,23,25,27,29,31,32,40].contains(&capture_phase) {
                                let dir=if route.full {"artifacts/chamber01-full"}else{"artifacts/chamber01"};std::fs::create_dir_all(dir).unwrap();
                                let path=format!("{dir}/phase-{capture_phase:02}.png");
                                capture_surface(&device,&queue,&frame.texture,config.width,config.height,config.format,&path).unwrap();
                                println!("chamber01 presented capture {path}; tick {} position {:?} portals {:?}",route.ticks,player.position,world.portals);
                                chamber_capture=capture_phase;
                            }
                            if (route.phase==15 && !route.full) || route.phase==32 {
                                assert!(!controls.captured && controls.keys.is_empty());
                                println!("chamber01 native route complete (full={}): real spawn, normal movement/use/portal controls; unknown {:?}",route.full,runtime.unknown_inputs);
                                if std::env::var_os("SOURCE_CHAMBER01_EXIT").is_some(){target.exit();}
                                chamber_route=None;
                            }
                        }
                        if cleanser_smoke && [0,10,25,60,75].contains(&frames) {
                            let dir="artifacts/progression";std::fs::create_dir_all(dir).unwrap();
                            let path=format!("{dir}/cleanser-{frames:04}.png");
                            capture_surface(&device,&queue,&frame.texture,config.width,config.height,config.format,&path).unwrap();
                            if frames==75 && std::env::var_os("SOURCE_CLEANSER_EXIT").is_some() {target.exit();}
                        }
                        if progression_smoke && (frames==0 || transitioned_at.is_some_and(|f| [f,f+90,f+300].contains(&frames))) {
                            let dir="artifacts/progression";std::fs::create_dir_all(dir).unwrap();
                            let path=format!("{dir}/transition-{frames:04}.png");
                            capture_surface(&device,&queue,&frame.texture,config.width,config.height,config.format,&path).unwrap();
                            println!("progression capture {path} player {:?} grounded {} unknown {:?}",player.position,player.grounded,runtime.unknown_inputs);
                            if transitioned_at.is_some_and(|f| frames==f+300) {
                                assert!(controls.keys.is_empty() && !controls.captured);
                                assert!(!player.noclip && player.grounded && player.position.x<600. && player.position.z>0.,"must walk out of destination elevator {:?}",player.position);
                                println!("progression smoke complete; next map rendered; normal controls ready, click to capture");
                                if std::env::var_os("SOURCE_PROGRESSION_EXIT").is_some(){target.exit();}
                            }
                        }
                        if cube_smoke && [0,100,120,160,215,240,320,369,460,465,520].contains(&frames) {
                            let dir="artifacts/cubes";let _=std::fs::create_dir_all(dir);
                            let path=format!("{dir}/cube-{frames:04}.png");
                            if let Err(e)=capture_surface(&device,&queue,&frame.texture,config.width,config.height,config.format,&path){eprintln!("cube capture {e}");target.exit();}
                            println!("cube smoke frame {frames} player {:?} door {:?} slots {:?}",player.position,runtime.origins[16],&runtime.origins[92..100]);
                            if frames==320 {assert!(runtime.origins[16].y> -800.,"cube weighting did not open door");}
                            if frames==520 {assert!((runtime.origins[16].y+836.).abs()<0.01,"pickup must release plate and close exit");println!("cube smoke complete: actual spawned model pickup, brush-colliding walk, drop, weighting, walk through exit and back, re-pickup and door closure; unsupported {:?}",runtime.unknown_inputs);if std::env::var_os("SOURCE_CUBE_EXIT").is_some(){target.exit();}}
                        }
                        if let Some(kind) = &parity {
                            let capture = if kind=="elevator-entry" {[0,10,15,20,25,30,180,330,570].contains(&frames)} else if kind=="dropper" {[0,40,100,180,220].contains(&frames)} else {[30,180,330,470,510,570].contains(&frames)};
                            if capture {
                                let path=format!("{parity_dir}/{kind}-{frames:04}.png");
                                if let Err(e)=capture_surface(&device,&queue,&frame.texture,config.width,config.height,config.format,&path) {eprintln!("parity capture: {e}");target.exit();return;}
                                println!("parity {kind} frame {frames} player {:?} grounded {} train {:?} animation {:?}",player.position,player.grounded,runtime.origins.get(37),runtime.animation_pose(if kind=="dropper" {"box_dropper_model_cover"} else {"elevator_door_model_middle"}));
                            }
                            if frames==if kind=="dropper" {220} else {570} {
                                if kind=="support-boundary" {
                                    assert!(runtime.origins[37].z>780. && player.grounded && !player.noclip);
                                    assert!(player.position.truncate().distance(Vec3::new(-1472.,-864.,0.).truncate())<70.);
                                }
                                println!("parity smoke finished; unsupported {:?}",runtime.unknown_inputs);
                                if std::env::var_os("SOURCE_PARITY_EXIT").is_some() {target.exit();}
                            }
                        }
                        if portal_smoke && [0,30,40,50,60,80,95,120,180].contains(&frames) {
                            let dir="artifacts/portals";let _=std::fs::create_dir_all(dir);
                            let path=format!("{dir}/{}-{frames:04}.png",if portal_smoke_mode=="floor" {"floor"}else{"portal"});
                            if let Err(e)=capture_surface(&device,&queue,&frame.texture,config.width,config.height,config.format,&path) {eprintln!("portal capture failed {e}");target.exit();}
                            println!("portal pair {:?}",world.portals);
                            println!("portal smoke frame {frames} player {:?} velocity {:?} crossings {} grounded {}",player.position,player.velocity,player.portal_crossings,player.grounded);
                            if frames==180 {assert!(player.portal_crossings>0,"scripted portal crossing failed");if std::env::var_os("SOURCE_PORTAL_EXIT").is_some(){target.exit();}}
                        }
                        crate::frame_profile::mark(8);
                        frame.present();
                            if let Some(map) = requested_map {
                                match root.as_ref().unwrap().load_map(&map, surfaces.is_some()) {
                                    Ok(next) => match crate::scene_gpu::GpuScene::new(&device, &queue, &next.scene, config.format) {
                                        Ok(gpu) => {
                                            device.poll(wgpu::Maintain::Wait);
                                            textured = Some(gpu); scene = Some(next.scene); world = next.world; runtime = next.runtime;
                                            player = next.player; portal_game = next.game; surfaces = next.surfaces;
                                            render_position = crate::play::RenderPosition::default();
                                            accumulator = 0.; last = Instant::now();
                                            controls.keys.clear();
                                            selected_map = map.clone();
                                            gui_error.clear();
                                            window.set_title(&format!("openSOURCE | {map} | F1 debug"));
                                        }
                                        Err(e) => gui_error = e,
                                    },
                                    Err(e) => gui_error = e,
                                }
                            }
                        crate::frame_profile::mark(9);
                        crate::frame_profile::finish();
                        frames += 1;
                        if gui_smoke && frames == 120 { println!("egui overlay smoke rendered 120 frames"); target.exit(); }
                        if demo.is_some() && frames >= 180 {
                            println!(
                                "door demo complete: 180 GPU-rendered frames; unsupported {:?}",
                                runtime.unknown_inputs
                            );
                            target.exit();
                        }
                        if frames.is_multiple_of(600) {
                            println!(
                                "frame {} position {:?} grounded {} controls.captured {}",
                                frames, player.position, player.grounded, controls.captured
                            );
                        }
                    }
                    _ => (),
                }
                },
                Event::AboutToWait => window.request_redraw(),
                _ => (),
            }
        })
        .map_err(|e| e.to_string())
}
fn capture_surface(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    w: u32,
    h: u32,
    format: wgpu::TextureFormat,
    path: &str,
) -> Result<(), String> {
    let pitch = (w * 4).div_ceil(256) * 256;
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("surface capture"),
        size: (pitch * h) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture,
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
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
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
    let mut pixels = Vec::new();
    for row in mapped.chunks_exact(pitch as usize) {
        pixels.extend_from_slice(&row[..w as usize * 4]);
    }
    drop(mapped);
    read.unmap();
    let rgba = crate::surface_rgba(
        &pixels,
        matches!(
            format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        ),
    );
    let mut png = png::Encoder::new(
        std::io::BufWriter::new(std::fs::File::create(path).map_err(|e| e.to_string())?),
        w,
        h,
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .map_err(|e| e.to_string())?
        .write_image_data(&rgba)
        .map_err(|e| e.to_string())?;
    Ok(())
}
