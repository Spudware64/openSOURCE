use opensource::{
    assets::Assets,
    audio::{decode, Audio, Event, SoundBank},
};
fn main() -> Result<(), String> {
    let game = std::env::args().nth(1).ok_or("game root")?;
    let fs = Assets::from_game(&game, None)?;

    let bank = SoundBank::load(&fs)?;
    let names: Vec<String> = std::env::args().skip(2).collect();
    for name in &names {
        let sound = bank.resolve(name, 0)?;
        let bytes = fs.read(&sound.path)?;
        let clip = decode(bytes)?;
        let peak = clip.samples.iter().fold(0f32, |a, b| a.max(b.abs()));
        println!(
            "DECODE {name} {} channels={} rate={} samples={} peak={peak}",
            sound.path,
            clip.channels,
            clip.rate,
            clip.samples.len()
        );
        assert!(peak > 0.001);
    }
    let audio = Audio::start(fs);
    for name in names {
        audio.play(Event::one(0., name, None));
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    Ok(())
}
