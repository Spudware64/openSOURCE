//! Read-only Source sound scripts and a bounded, nonblocking runtime audio worker.
use crate::{assets::Assets, kv};
use glam::Vec3;
use rodio::{Sink, Source, SpatialSink};
use std::{
    collections::{HashMap, HashSet},
    io::Cursor,
    sync::{
        mpsc::{sync_channel, SyncSender},
        Arc, Mutex,
    },
    time::Duration,
};
#[derive(Clone, Debug)]
pub struct Sound {
    pub path: String,
    pub volume: f32,
    pub pitch: f32,
}
#[derive(Default)]
pub struct SoundBank {
    scripts: HashMap<String, Vec<Sound>>,
}
impl SoundBank {
    pub fn parse(&mut self, text: &str) -> Result<(), String> {
        for (name, value) in kv::parse(text)? {
            let kv::Value::Block(pairs) = value else {
                continue;
            };
            let number = |key, default| {
                kv::text(&pairs, key)
                    .and_then(|v| v.split(',').next()?.trim().parse::<f32>().ok())
                    .unwrap_or(default)
            };
            let volume = number("volume", 1.).clamp(0., 1.);
            let pitch = number("pitch", 100.).clamp(25., 255.) / 100.;
            let waves = kv::child(&pairs, "rndwave").unwrap_or(&pairs);
            let mut sounds = vec![];
            for (key, value) in waves {
                if key.eq_ignore_ascii_case("wave") {
                    if let kv::Value::Text(wave) = value {
                        sounds.push(Sound {
                            path: wave_path(wave)?,
                            volume,
                            pitch,
                        });
                    }
                }
            }
            if !sounds.is_empty() {
                self.scripts.insert(name.to_ascii_lowercase(), sounds);
            }
        }
        Ok(())
    }
    pub fn load(fs: &Assets) -> Result<Self, String> {
        let mut bank = Self::default();
        let manifest = fs.read("scripts/game_sounds_manifest.txt")?;
        for (key, value) in kv::child(
            &kv::parse(&String::from_utf8_lossy(&manifest))?,
            "game_sounds_manifest",
        )
        .ok_or("sound manifest root")?
        {
            if key.eq_ignore_ascii_case("precache_file") {
                if let kv::Value::Text(path) = value {
                    match fs
                        .read(path)
                        .and_then(|b| bank.parse(&String::from_utf8_lossy(&b)))
                    {
                        Ok(()) => {}
                        Err(e) => eprintln!("audio script {path}: {e}"),
                    };
                }
            }
        }
        eprintln!("audio: {} sound scripts loaded", bank.scripts.len());
        Ok(bank)
    }
    pub fn resolve(&self, name: &str, seed: u64) -> Result<Sound, String> {
        if let Some(waves) = self.scripts.get(&name.to_ascii_lowercase()) {
            return Ok(waves[seed as usize % waves.len()].clone());
        }
        if name.to_ascii_lowercase().ends_with(".wav")
            || name.to_ascii_lowercase().ends_with(".mp3")
        {
            return Ok(Sound {
                path: wave_path(name)?,
                volume: 1.,
                pitch: 1.,
            });
        }
        Err(format!("unknown sound {name}"))
    }
}
fn wave_path(name: &str) -> Result<String, String> {
    let name = name.trim_start_matches(|c: char| "*#@><^)}$!?".contains(c));
    crate::assets::normalise(&format!("sound/{name}"))
}
#[derive(Clone, Debug)]
pub struct Event {
    pub time: f32,
    pub name: String,
    pub position: Option<Vec3>,
    pub owner: Option<usize>,
    pub looping: bool,
    pub stop: bool,
    pub volume: f32,
}
impl Event {
    pub fn one(time: f32, name: impl Into<String>, position: Option<Vec3>) -> Self {
        Self {
            time,
            name: name.into(),
            position,
            owner: None,
            looping: false,
            stop: false,
            volume: 1.,
        }
    }
}
#[derive(Default)]
pub struct MovementAudio {
    previous: Option<(bool, Vec3, u64)>,
    distance: f32,
}
impl MovementAudio {
    pub fn step(&mut self, p: &crate::play::Player, time: f32) -> Vec<Event> {
        let mut events = vec![];
        if let Some((grounded, position, crossings)) = self.previous {
            if !p.noclip {
                if p.portal_crossings != crossings {
                    for name in ["PortalPlayer.EnterPortal", "PortalPlayer.ExitPortal"] {
                        events.push(Event::one(time, name, None));
                    }
                    self.distance = 0.;
                } else {
                    if grounded && !p.grounded && p.velocity.z > 0. {
                        events.push(Event::one(time, "player/footsteps/concrete1.wav", None));
                    }
                    if !grounded && p.grounded {
                        events.push(Event::one(time, "PortalPlayer.FallRecover", None));
                    }
                    if grounded && p.grounded {
                        self.distance += p.position.truncate().distance(position.truncate());
                        if self.distance >= 80. {
                            self.distance %= 80.;
                            events.push(Event::one(time, "Concrete.StepLeft", None));
                        }
                    }
                }
            }
        }
        self.previous = Some((p.grounded, p.position, p.portal_crossings));
        events
    }
}
#[derive(Clone)]
pub struct Clip {
    pub channels: u16,
    pub rate: u32,
    pub samples: Vec<f32>,
}
pub fn decode(bytes: Vec<u8>) -> Result<Clip, String> {
    let source = rodio::Decoder::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let channels = source.channels();
    let rate = source.sample_rate();
    let samples: Vec<f32> = source.convert_samples().take(48_000 * 2 * 600).collect();
    if samples.is_empty() {
        return Err("empty decoded sound".into());
    }
    Ok(Clip {
        channels,
        rate,
        samples,
    })
}
// Listener updates coalesce; they never fill the event queue or block simulation.
#[derive(Clone)]
pub struct Audio {
    tx: SyncSender<Event>,
    listener: Arc<Mutex<(Vec3, Vec3)>>,
}
enum Voice {
    Flat(Sink),
    Spatial(SpatialSink, Vec3),
}
impl Voice {
    fn empty(&self) -> bool {
        match self {
            Self::Flat(s) => s.empty(),
            Self::Spatial(s, _) => s.empty(),
        }
    }
    fn stop(&self) {
        match self {
            Self::Flat(s) => s.stop(),
            Self::Spatial(s, _) => s.stop(),
        }
    }
}
impl Audio {
    pub fn start(fs: Assets) -> Self {
        let (tx, rx) = sync_channel::<Event>(256);
        let listener = Arc::new(Mutex::new((Vec3::ZERO, Vec3::Y)));
        let shared = listener.clone();
        let spawned = std::thread::Builder::new()
            .name("source-audio".into())
            .spawn(move || {
                let volume = std::env::var("SOURCE_AUDIO_VOLUME")
                    .ok()
                    .and_then(|v| v.parse::<f32>().ok())
                    .filter(|v| v.is_finite())
                    .unwrap_or(0.35)
                    .clamp(0., 1.);
                if volume == 0. {
                    eprintln!("audio: muted");
                    return;
                }
                let bank = match SoundBank::load(&fs) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("audio disabled: {e}");
                        return;
                    }
                };
                let (_stream, handle) = match rodio::OutputStream::try_default() {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("audio device unavailable: {e}");
                        return;
                    }
                };
                eprintln!("audio: output device started (rodio 0.20.1), gain {volume}");
                let mut cache: HashMap<String, Clip> = HashMap::new();
                let mut cache_bytes = 0usize;
                let mut failed = HashSet::new();
                let mut voices: Vec<(Option<usize>, Voice)> = vec![];
                let mut seed = 0;
                loop {
                    let event = match rx.recv_timeout(Duration::from_millis(15)) {
                        Ok(e) => Some(e),
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                        Err(_) => break,
                    };
                    voices.retain(|(_, v)| !v.empty());
                    let (eye, right) = *shared.lock().unwrap();
                    for (_, v) in &voices {
                        if let Voice::Spatial(s, p) = v {
                            s.set_left_ear_position(((eye - right * 4.) / 128.).to_array());
                            s.set_right_ear_position(((eye + right * 4.) / 128.).to_array());
                            s.set_volume(volume / (1. + p.distance(eye) / 512.));
                        }
                    }
                    let Some(e) = event else { continue };
                    if e.stop && e.owner.is_none() {
                        for (_, voice) in voices.drain(..) {
                            voice.stop();
                        }
                        continue;
                    }
                    if e.stop || e.looping {
                        voices.retain(|(id, v)| {
                            if e.owner.is_some() && *id == e.owner {
                                v.stop();
                                false
                            } else {
                                true
                            }
                        });
                    }
                    if e.stop {
                        continue;
                    }
                    let result = (|| -> Result<(), String> {
                        let sound = bank.resolve(&e.name, seed)?;
                        seed += 1;
                        if !cache.contains_key(&sound.path) {
                            let clip = decode(fs.read(&sound.path)?)?;
                            let bytes = clip.samples.len() * 4;
                            if cache_bytes + bytes > 64 * 1024 * 1024 {
                                cache.clear();
                                cache_bytes = 0;
                            }
                            cache_bytes += bytes;
                            cache.insert(sound.path.clone(), clip);
                        }
                        if voices.len() >= 64 {
                            return Ok(());
                        }
                        let clip = &cache[&sound.path];
                        let source = rodio::buffer::SamplesBuffer::new(
                            clip.channels,
                            clip.rate,
                            clip.samples.clone(),
                        )
                        .speed(sound.pitch)
                        .amplify(sound.volume * e.volume);
                        let source: Box<dyn Source<Item = f32> + Send> = if e.looping {
                            Box::new(source.repeat_infinite())
                        } else {
                            Box::new(source)
                        };
                        let voice = if let Some(p) = e
                            .position
                            .filter(|_| !sound.path.starts_with("sound/music/"))
                        {
                            let s = SpatialSink::try_new(
                                &handle,
                                (p / 128.).to_array(),
                                ((eye - right * 4.) / 128.).to_array(),
                                ((eye + right * 4.) / 128.).to_array(),
                            )
                            .map_err(|e| e.to_string())?;
                            s.set_volume(volume / (1. + p.distance(eye) / 512.));
                            s.append(source);
                            Voice::Spatial(s, p)
                        } else {
                            let s = Sink::try_new(&handle).map_err(|e| e.to_string())?;
                            s.set_volume(volume);
                            s.append(source);
                            Voice::Flat(s)
                        };
                        if std::env::var_os("SOURCE_AUDIO_TRACE").is_some() {
                            eprintln!(
                                "audio play t={:.6} {} -> {} positional={} loop={}",
                                e.time,
                                e.name,
                                sound.path,
                                e.position.is_some(),
                                e.looping
                            );
                        }
                        voices.push((e.owner, voice));
                        Ok(())
                    })();
                    if let Err(error) = result {
                        if failed.insert(e.name.clone()) {
                            eprintln!("audio {}: {error}", e.name);
                        }
                    }
                }
            });
        if let Err(error) = spawned {
            eprintln!("audio worker unavailable: {error}");
        }
        Self { tx, listener }
    }
    pub fn play(&self, event: Event) {
        let _ = self.tx.try_send(event);
    }
    pub fn listener(&self, eye: Vec3, right: Vec3) {
        if let Ok(mut l) = self.listener.try_lock() {
            *l = (eye, right);
        }
    }
}
