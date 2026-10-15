//! Isolate one genuinely decoded chamber prop for visual verification.
use portal_bsp_viewer::{
    assets::Assets, props::static_props, render_scene, scene::Scene, studio::decode,
};
fn main() -> Result<(), String> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or("models/props/door_01_frame_wide_reference.mdl".into());
    let output = std::env::args()
        .nth(2)
        .unwrap_or("evidence/prop-model-preview.png".into());
    let b = std::fs::read("../portal-original/portal/maps/testchmb_a_00.bsp")
        .map_err(|e| e.to_string())?;
    let fs = Assets::from_game("../portal-original/portal", Some(&b))?;
    let props = static_props(&b)?;
    let mut scene = Scene::load(&b, &fs)?;
    if scene.skipped_props != 0 {
        return Err("preview requires fully decoded chamber fixture".into());
    }
    let lengths = props
        .models
        .iter()
        .map(|p| {
            let s = p.trim_end_matches(".mdl");
            decode(
                &fs.read(p)?,
                &fs.read(&format!("{s}.vvd"))?,
                &fs.read(&format!("{s}.dx90.vtx"))?,
            )
            .map(|m| m.vertices.len())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut offset = scene.vertices.len() - scene.prop_triangles * 3;
    let mut found = false;
    for prop in props.instances {
        let n = lengths[prop.model as usize];
        if props.models[prop.model as usize] == path {
            scene.vertices = scene.vertices[offset..offset + n].to_vec();
            found = true;
            println!(
                "{path}: {} triangles at {:?}, angles {:?}",
                n / 3,
                prop.origin,
                prop.angles
            );
            break;
        }
        offset += n;
    }
    if !found {
        return Err("model not used by chamber".into());
    }
    let pixels = render_scene(&scene, 1024, 768, None)?;
    let mut png = png::Encoder::new(
        std::io::BufWriter::new(std::fs::File::create(&output).map_err(|e| e.to_string())?),
        1024,
        768,
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .map_err(|e| e.to_string())?
        .write_image_data(&pixels)
        .map_err(|e| e.to_string())?;
    println!("wrote {output}");
    Ok(())
}
