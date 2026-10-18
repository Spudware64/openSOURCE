//! Windowed install and map selection, before any BSP or scene resources exist.
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub fn resolve_game_dir(input: &str) -> Result<PathBuf, String> {
    let raw = input.trim().trim_matches('"');
    if raw.is_empty() {
        return Err("Choose your Portal install directory first.".into());
    }
    let path = Path::new(raw);
    if !path.is_dir() {
        return Err(format!("Directory does not exist: {}", path.display()));
    }
    let root = if path.join("gameinfo.txt").is_file() {
        path.to_path_buf()
    } else {
        path.join("portal")
    };
    if !root.join("gameinfo.txt").is_file() {
        return Err(format!("No gameinfo.txt found in {}. Choose the Portal install folder or its portal content folder.", root.display()));
    }
    root.canonicalize()
        .map_err(|e| format!("Cannot open Portal directory: {e}"))
}

fn saved_path() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    base.map(|p| p.join("openSOURCE").join("portal-directory.txt"))
}
fn save_directory(path: &Path) -> Result<(), String> {
    let Some(file) = saved_path() else {
        return Ok(());
    };
    std::fs::create_dir_all(file.parent().unwrap()).map_err(|e| e.to_string())?;
    let temp = file.with_extension("tmp");
    std::fs::write(&temp, path.as_os_str().to_string_lossy().as_bytes())
        .map_err(|e| e.to_string())?;
    if file.exists() {
        std::fs::remove_file(&file).map_err(|e| e.to_string())?;
    }
    std::fs::rename(temp, file).map_err(|e| e.to_string())
}

pub fn open() -> Result<(), String> {
    let events = EventLoop::new().map_err(|e| e.to_string())?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("openSOURCE — choose Portal install")
            .with_inner_size(winit::dpi::LogicalSize::new(700., 560.))
            .build(&events)
            .map_err(|e| e.to_string())?,
    );
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
            .ok_or("No graphics adapter available")?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("setup"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .map_err(|e| e.to_string())?;
        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("No window surface configuration")?;
        Ok::<_, String>((device, queue, config))
    })?;
    surface.configure(&device, &config);
    let ctx = egui::Context::default();
    let mut state = egui_winit::State::new(
        ctx.clone(),
        egui::ViewportId::ROOT,
        &events,
        Some(window.scale_factor() as f32),
        None,
    );
    let mut renderer = egui_wgpu::Renderer::new(&device, config.format, None, 1);
    let mut directory = saved_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();
    let mut game: Option<PathBuf> = None;
    let mut maps = Vec::<String>::new();
    let mut selected = String::new();
    let mut filter = String::new();
    let mut error = String::new();
    let mut frames = 0u32;
    window.set_cursor_visible(true);
    events.run(move |event, target| {
        target.set_control_flow(ControlFlow::Poll);
        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => {
                let response = state.on_window_event(&window, &event);
                match event {
                    WindowEvent::CloseRequested => target.exit(),
                    WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                        config.width = size.width; config.height = size.height; surface.configure(&device, &config);
                    }
                    WindowEvent::RedrawRequested => {
                        let mut scan = false;
                        let mut launch = false;
                        let input = state.take_egui_input(&window);
                        let output = ctx.run(input, |ctx| {
                            egui::CentralPanel::default().show(ctx, |ui| {
                                ui.heading("openSOURCE");
                                ui.label("Select your Portal installation or its portal content directory.");
                                ui.horizontal(|ui| {
                                    ui.label("Directory");
                                    if ui.add(egui::TextEdit::singleline(&mut directory).desired_width(f32::INFINITY)).changed() {
                                        game = None; maps.clear(); selected.clear(); error.clear();
                                    }
                                });
                                if ui.button("Find maps").clicked() { scan = true; }
                                if !error.is_empty() { ui.colored_label(egui::Color32::RED, &error); }
                                if game.is_some() {
                                    ui.separator();
                                    ui.heading(format!("Maps ({})", maps.len()));
                                    ui.text_edit_singleline(&mut filter);
                                    egui::ScrollArea::vertical().max_height(280.).show(ui, |ui| {
                                        for map in &maps {
                                            if map.contains(&filter.to_ascii_lowercase()) && ui.selectable_label(*map == selected, map).clicked() { selected = map.clone(); }
                                        }
                                    });
                                    if ui.add_enabled(!selected.is_empty(), egui::Button::new("Load selected map")).clicked() { launch = true; }
                                }
                            });
                        });
                        state.handle_platform_output(&window, output.platform_output);
                        if scan {
                            match resolve_game_dir(&directory).and_then(|p| crate::session::GameRoot::new(&p, 4).and_then(|root| root.maps()).map(|maps| (p, maps))) {
                                Ok((p, found)) => {
                                    game = Some(p); maps = found; selected = maps.first().cloned().unwrap_or_default();
                                    error = if maps.is_empty() { "No BSP maps were found through gameinfo.txt SearchPaths.".into() } else { String::new() };
                                }
                                Err(e) => { game = None; maps.clear(); selected.clear(); error = e; }
                            }
                        }
                        if launch {
                            if let Some(path) = &game {
                                if let Err(e) = save_directory(path) { eprintln!("Could not save install directory: {e}"); }
                                match std::env::current_exe().and_then(|exe| std::process::Command::new(exe)
                                    .arg("--game").arg(path).arg("--map").arg(&selected).arg("--play").spawn()) {
                                    Ok(_) => target.exit(),
                                    Err(e) => error = format!("Could not launch map: {e}"),
                                }
                            }
                        }
                        let pixels_per_point = window.scale_factor() as f32;
                        let jobs = ctx.tessellate(output.shapes, pixels_per_point);
                        let screen = egui_wgpu::ScreenDescriptor { size_in_pixels: [config.width, config.height], pixels_per_point };
                        for (id, delta) in &output.textures_delta.set { renderer.update_texture(&device, &queue, *id, delta); }
                        match surface.get_current_texture() {
                            Ok(frame) => {
                                let view = frame.texture.create_view(&Default::default());
                                let mut encoder = device.create_command_encoder(&Default::default());
                                for extra in renderer.update_buffers(&device, &queue, &mut encoder, &jobs, &screen) { queue.submit(Some(extra)); }
                                {
                                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: Some("setup"), color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                            view: &view, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                                        })], depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None,
                                    });
                                    renderer.render(&mut pass, &jobs, &screen);
                                }
                                queue.submit(Some(encoder.finish())); frame.present();
                            }
                            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => surface.configure(&device, &config),
                            Err(e) => { eprintln!("setup surface: {e}"); target.exit(); }
                        }
                        for id in &output.textures_delta.free { renderer.free_texture(id); }
                        frames += 1;
                        if std::env::var_os("SOURCE_SETUP_SMOKE").is_some() && frames >= 30 {
                            println!("setup GUI rendered {frames} frames without a BSP"); target.exit();
                        }
                    }
                    _ => { if response.repaint { window.request_redraw(); } }
                }
            }
            Event::AboutToWait => window.request_redraw(),
            _ => {}
        }
    }).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_root_and_content_directory() {
        let base = std::env::temp_dir().join(format!("opensource-setup-{}", std::process::id()));
        let portal = base.join("portal");
        std::fs::create_dir_all(&portal).unwrap();
        std::fs::write(portal.join("gameinfo.txt"), "GameInfo {}").unwrap();
        assert_eq!(
            resolve_game_dir(base.to_str().unwrap()).unwrap(),
            portal.canonicalize().unwrap()
        );
        assert_eq!(
            resolve_game_dir(&format!("\"{}\"", portal.display())).unwrap(),
            portal.canonicalize().unwrap()
        );
        assert!(resolve_game_dir("").unwrap_err().contains("Choose"));
        assert!(resolve_game_dir(base.join("missing").to_str().unwrap())
            .unwrap_err()
            .contains("does not exist"));
        std::fs::remove_dir_all(base).unwrap();
    }
}
