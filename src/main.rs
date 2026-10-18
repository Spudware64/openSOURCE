fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if starts_setup(&args) {
        return opensource::setup::open().map_err(Into::into);
    }
    if args.get(1).is_some_and(|s| s == "--game") {
        return game_cli(&args[1..]);
    }
    if args.len() == 3 && args[1] == "--play" {
        return opensource::interactive(&std::fs::read(&args[2])?).map_err(Into::into);
    }
    if args.len() != 3 && args.len() != 9 {
        return Err(
            "usage: openSOURCE MAP.bsp OUTPUT.png [eye_x eye_y eye_z target_x target_y target_z]"
                .into(),
        );
    }
    let camera = if args.len() == 9 {
        let n: Vec<f32> = args[3..]
            .iter()
            .map(|s| s.parse())
            .collect::<Result<_, _>>()?;
        Some(([n[0], n[1], n[2]], [n[3], n[4], n[5]]))
    } else {
        None
    };
    let bytes = std::fs::read(&args[1])?;
    let vertices = opensource::load(&bytes)?;
    println!(
        "map: {} bytes, {} triangles",
        bytes.len(),
        vertices.len() / 3
    );
    let pixels = opensource::render(&vertices, 1024, 768, camera)?;
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(std::fs::File::create(&args[2])?),
        1024,
        768,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&pixels)?;
    println!("wrote {} (1024x768)", args[2]);
    Ok(())
}
fn game_cli(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    use opensource::{assets::Assets, scene::Scene};
    let mut game = None;
    let mut map = None;
    let mut output = None;
    let mut play = false;
    let mut workers = 4;
    let mut camera = None;
    let mut p = 0;
    while p < args.len() {
        let key = &args[p];
        p += 1;
        if key == "--play" {
            play = true;
            continue;
        }
        if key == "--camera" {
            let numbers = args
                .get(p..p + 6)
                .ok_or("--camera needs six Source coordinates")?
                .iter()
                .map(|s| s.parse::<f32>())
                .collect::<Result<Vec<_>, _>>()?;
            if numbers.iter().any(|x| !x.is_finite()) {
                return Err("nonfinite camera".into());
            }
            camera = Some((
                [numbers[0], numbers[1], numbers[2]],
                [numbers[3], numbers[4], numbers[5]],
            ));
            p += 6;
            continue;
        }
        let value = args.get(p).ok_or("missing option value")?;
        p += 1;
        match key.as_str() {
            "--game" => game = Some(value),
            "--map" => map = Some(value),
            "--output" => output = Some(value),
            "--workers" => workers = value.parse::<usize>()?,
            _ => return Err(format!("unknown option {key}").into()),
        }
    }
    if !(1..=16).contains(&workers) {
        return Err("asset workers must be 1..16".into());
    }
    let game = game.ok_or("--game must select a directory containing gameinfo.txt")?;
    let map = map.ok_or("--map required")?;
    println!(
        "startup: build={} exe={:?} game={game:?} map={map:?} play={play}",
        env!("SOURCE_BUILD_GIT"),
        std::env::current_exe()
    );
    if play == output.is_some() {
        return Err("select either --play or --output FILE.png".into());
    }
    let fs = Assets::from_game(game, None)?;
    let bytes = if std::path::Path::new(map).is_absolute() {
        std::fs::read(map)?
    } else {
        fs.read(map)?
    };
    let fs = Assets::from_game(game, Some(&bytes))?;
    println!(
        "SearchPaths: {:?}; asset workers {workers}",
        fs.search_paths()
    );
    let scene = Scene::load_with_workers(&bytes, &fs, workers)?;
    println!("Source 1 map: {} bytes, {} visible triangles, {} material layers, {} filtered faces, {} baked-lightmap faces",bytes.len(),scene.vertices.len()/3,scene.textures.len(),scene.filtered_faces,scene.lightmapped_faces);
    println!("terrain: {} displacement faces / {} triangles; static props: {} rendered / {} skipped; entity models: {} rendered / {} skipped; combined prop triangles {}",scene.displacement_faces,scene.displacement_triangles,scene.static_props,scene.skipped_props,scene.dynamic_props,scene.skipped_dynamic_props,scene.prop_triangles);
    for warning in &scene.warnings {
        eprintln!("material warning: {warning}");
    }
    if play {
        let portal_override = std::env::var("SOURCE_PORTALS").ok();
        let surfaces = if opensource::portal::rules_enabled(&bytes, portal_override.as_deref())? {
            Some(opensource::portal::Surfaces::load(&bytes, &fs)?)
        } else {
            None
        };
        let root = opensource::session::GameRoot::new(game, workers)?;
        return opensource::interactive_game(&bytes, scene, surfaces, root).map_err(Into::into);
    }
    let pixels = opensource::render_scene(&scene, 1024, 768, camera)?;
    let output = output.unwrap();
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(std::fs::File::create(output)?),
        1024,
        768,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&pixels)?;
    println!("wrote {output} (1024x768)");
    Ok(())
}

fn starts_setup(args: &[String]) -> bool {
    args.len() == 1
}

#[cfg(test)]
mod tests {
    use super::starts_setup;
    #[test]
    fn no_arguments_enter_setup() {
        assert!(starts_setup(&["openSOURCE".into()]));
        assert!(!starts_setup(&[
            "openSOURCE".into(),
            "--play".into(),
            "test.bsp".into()
        ]));
        assert!(!starts_setup(&[
            "openSOURCE".into(),
            "map.bsp".into(),
            "out.png".into()
        ]));
        assert!(!starts_setup(&["openSOURCE".into(), "--game".into()]));
    }
}
