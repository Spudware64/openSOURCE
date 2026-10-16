use crate::play::{Player, World};
use glam::{Mat3, Mat4, Vec3};
use std::collections::{BTreeSet, HashMap};
#[derive(Clone)]
struct Output {
    event: String,
    target: String,
    input: String,
    parameter: String,
    delay: f32,
    remaining: i32,
}
#[derive(Clone)]
struct State {
    timer: Option<(f32, Option<f32>)>,
    counter: Option<(f32, f32, f32)>,
    parent: Option<usize>,
    anchor: Mat4,
    reference: Mat4,
    entity: Entity,
    model: Option<usize>,
    velocity: Option<Vec3>,
    enabled: bool,
    killed: bool,
    touching: bool,
    activator: Option<usize>,
    next_trigger: f32,
    outputs: Vec<Output>,
    motion: Option<Motion>,
    train: Option<Train>,
    bounds: (Vec3, Vec3),
}
#[derive(Clone)]
struct Train {
    next: Option<usize>,
    offset: Vec3,
    speed: f32,
    running: bool,
}
#[derive(Clone)]
struct Motion {
    angular: bool,
    base: Vec3,
    travel: Vec3,
    amount: f32,
    goal: f32,
    speed: f32,
    wait: f32,
    close_at: Option<f32>,
    button: bool,
}
struct Playback {
    clips: Vec<(String, f32, bool)>,
    sequence: String,
    time: f32,
    completed: bool,
    done_count: usize,
    default: Option<String>,
}
struct Pending {
    event: String,
    due: f32,
    target: String,
    input: String,
    parameter: String,
    source: usize,
    activator: Option<usize>,
}
/// A deliberately bounded subset of Source entity I/O, not a gameplay-DLL interpreter.
#[derive(Clone, Debug)]
pub struct ChangeLevel {
    pub map: String,
    pub landmark: String,
    pub source: Vec3,
}
impl ChangeLevel {
    pub fn arrival(&self, entities: &[Entity], position: Vec3) -> Result<Vec3, String> {
        let destination = entities
            .iter()
            .find(|e| {
                e.get("classname") == Some("info_landmark")
                    && e.get("targetname") == Some(self.landmark.as_str())
            })
            .ok_or_else(|| format!("missing destination landmark {}", self.landmark))?
            .origin()?;
        Ok(position + destination - self.source)
    }
}
pub struct Runtime {
    scene_times: HashMap<usize, (f32, Option<f32>)>,
    pub portal_inventory: [bool; 2],
    pub portal_shots: Vec<(usize, Vec3, Vec3)>,
    pub change_level: Option<ChangeLevel>,
    pub cleanse_portals: bool,
    /// Opt-in game-module event sink; None retains unsupported-input diagnostics.
    pub portal_events: Option<Vec<(String, bool)>>,
    states: Vec<State>,
    held: Option<usize>,
    animations: HashMap<usize, Playback>,
    pub origins: Vec<Vec3>,
    pub visible: Vec<bool>,
    pub unknown_inputs: BTreeSet<String>,
    queue: Vec<Pending>,
    time: f32,
    names: HashMap<String, Vec<usize>>,
}
impl Runtime {
    pub fn bind_animations(&mut self, scene: &crate::scene::Scene) {
        self.bind_scene_timing(&scene.scene_durations);
        let mut bounds =
            vec![(Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)); self.origins.len()];
        for v in &scene.vertices {
            if let Some((lo, hi)) = bounds.get_mut(v.model as usize) {
                let p = Vec3::from(v.position);
                *lo = lo.min(p);
                *hi = hi.max(p);
            }
        }
        for state in &mut self.states {
            if state.velocity.is_none() {
                continue;
            }
            if let Some(&(lo, hi)) = state.model.and_then(|m| bounds.get(m)) {
                let size = hi - lo;
                if lo.is_finite()
                    && hi.is_finite()
                    && size.min_element() > 0.01
                    && size.max_element() < 2048.
                {
                    state.bounds = (lo, hi);
                }
            }
        }
        for instance in &scene.animations {
            if let Some(ids) = self.names.get(&instance.name) {
                for &id in ids {
                    self.animations.insert(
                        id,
                        Playback {
                            clips: instance
                                .skeleton
                                .clips
                                .iter()
                                .map(|c| (c.name.clone(), c.duration(), c.looping))
                                .collect(),
                            sequence: instance.initial.clone(),
                            time: 0.,
                            completed: false,
                            done_count: 0,
                            default: None,
                        },
                    );
                }
            }
        }
    }
    pub fn animation_pose(&self, name: &str) -> Option<(&str, f32)> {
        let id = self.names.get(name)?.first()?;
        let p = self.animations.get(id)?;
        Some((&p.sequence, p.time))
    }
    pub fn animation_done_count(&self, name: &str) -> usize {
        self.names
            .get(name)
            .into_iter()
            .flatten()
            .filter_map(|id| self.animations.get(id))
            .map(|p| p.done_count)
            .sum()
    }
    pub fn new(b: &[u8]) -> Result<Self, String> {
        crate::load(b)?;
        let bounds = crate::lump(b, 14, 48)?
            .chunks_exact(48)
            .map(|a| {
                let f = |o| f32::from_le_bytes(a[o..o + 4].try_into().unwrap());
                (Vec3::new(f(0), f(4), f(8)), Vec3::new(f(12), f(16), f(20)))
            })
            .collect();
        Self::from_entities(parse_entities(crate::lump(b, 0, 1)?)?, bounds)
    }
    pub fn from_entities(entities: Vec<Entity>, bounds: Vec<(Vec3, Vec3)>) -> Result<Self, String> {
        if entities
            .iter()
            .filter(|e| e.get("classname") == Some("prop_physics"))
            .count()
            > 128
        {
            return Err("physics population limit (128 authored bodies)".into());
        }
        let mut r = Self {
            scene_times: HashMap::new(),
            portal_inventory: [false; 2],
            portal_shots: vec![],
            change_level: None,
            cleanse_portals: false,
            portal_events: None,
            states: vec![],
            held: None,
            animations: HashMap::new(),
            origins: vec![Vec3::ZERO; bounds.len()],
            visible: vec![true; bounds.len()],
            unknown_inputs: BTreeSet::new(),
            queue: vec![],
            time: 0.,
            names: HashMap::new(),
        };
        let number = |e: &Entity, key: &str, default: f32| -> Result<f32, String> {
            let n = e
                .get(key)
                .map(|s| s.parse::<f32>().map_err(|e| e.to_string()))
                .transpose()?
                .unwrap_or(default);
            if !n.is_finite() {
                return Err(format!("nonfinite {key}"));
            }
            Ok(n)
        };
        for e in entities {
            let id = r.states.len();
            let mut model = e.model()?;
            let physics = e.get("classname") == Some("prop_physics");
            if physics {
                model = Some(r.origins.len());
                r.origins.push(e.origin()?);
                r.visible.push(true);
            }
            let origin = e.origin()?;
            let class = e.get("classname").unwrap_or("");
            let flags = e
                .get("spawnflags")
                .unwrap_or("0")
                .parse::<u32>()
                .map_err(|e| e.to_string())?;
            let mut local = (Vec3::ZERO, Vec3::ZERO);
            if let Some(m) = model {
                let physical_bounds = (Vec3::splat(-20.), Vec3::splat(20.));
                let b = if physics {
                    &physical_bounds
                } else {
                    bounds.get(m).ok_or("runtime model range")?
                };
                if !b.0.is_finite() || !b.1.is_finite() || b.0.cmpgt(b.1).any() {
                    return Err("invalid model bounds".into());
                }
                let rotation = crate::models::rotation(vector(e.get("angles").unwrap_or("0 0 0"))?);
                local = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
                for x in [b.0.x, b.1.x] {
                    for y in [b.0.y, b.1.y] {
                        for z in [b.0.z, b.1.z] {
                            let p = rotation * Vec3::new(x, y, z);
                            local.0 = local.0.min(p);
                            local.1 = local.1.max(p);
                        }
                    }
                }
                r.origins[m] = origin;
            }
            let enabled = e.get("StartDisabled") != Some("1");
            if class == "func_brush" {
                if let Some(m) = model {
                    r.visible[m] = enabled;
                }
            }
            let mut outputs = vec![];
            for (key, value) in &e.pairs {
                if !key.starts_with("On") && key != "OutValue" {
                    continue;
                }
                let fields = value
                    .split(if value.contains('\u{1b}') {
                        '\u{1b}'
                    } else {
                        ','
                    })
                    .collect::<Vec<_>>();
                if fields.len() != 5 {
                    return Err(format!("malformed entity output {value}"));
                }
                let delay = fields[3].trim().parse::<f32>().map_err(|e| e.to_string())?;
                let remaining = fields[4].trim().parse::<i32>().map_err(|e| e.to_string())?;
                if !delay.is_finite() || delay < 0. || remaining < -1 {
                    return Err("invalid output delay/count".into());
                }
                outputs.push(Output {
                    event: key.to_owned(),
                    target: fields[0].to_owned(),
                    input: fields[1].to_owned(),
                    parameter: fields[2].to_owned(),
                    delay,
                    remaining,
                });
            }
            let motion = if let Some(model) = model.filter(|_| {
                [
                    "func_door",
                    "func_door_rotating",
                    "func_button",
                    "func_movelinear",
                ]
                .contains(&class)
            }) {
                let a = vector(e.get("movedir").unwrap_or("0 0 0"))?;
                let dir = if a.x == -1. {
                    Vec3::Z
                } else if a.x == -2. {
                    -Vec3::Z
                } else {
                    Vec3::new(
                        a.y.to_radians().cos() * a.x.to_radians().cos(),
                        a.y.to_radians().sin() * a.x.to_radians().cos(),
                        -a.x.to_radians().sin(),
                    )
                };
                let distance = (dir.abs().dot(local.1 - local.0) - number(&e, "lip", 0.)?).max(0.);
                let angular = class == "func_door_rotating";
                let travel = if angular {
                    let axis = if flags & 64 != 0 {
                        Vec3::Y
                    } else if flags & 128 != 0 {
                        Vec3::X
                    } else {
                        Vec3::Z
                    };
                    axis * number(&e, "distance", 90.)? * if flags & 2 != 0 { -1. } else { 1. }
                } else {
                    dir * distance
                };
                let amount = if class == "func_door" && flags & 1 != 0 {
                    1.
                } else {
                    0.
                };
                r.origins[model] = origin + travel * amount;
                Some(Motion {
                    angular,
                    base: origin,
                    travel,
                    amount,
                    goal: amount,
                    speed: number(&e, "speed", 100.)?.max(1.),
                    wait: number(&e, "wait", 3.)?,
                    close_at: None,
                    button: class == "func_button",
                })
            } else {
                None
            };
            if let Some(name) = e.get("targetname") {
                r.names.entry(name.to_owned()).or_default().push(id);
            }
            r.states.push(State {
                timer: if class == "logic_timer" {
                    let interval = number(&e, "RefireTime", 1.)?.max(1. / 120.);
                    Some((interval, enabled.then_some(interval)))
                } else {
                    None
                },
                counter: if class == "math_counter" {
                    let min = number(&e, "min", 0.)?;
                    let max = number(&e, "max", 0.)?;
                    let value = number(&e, "startvalue", 0.)?;
                    Some((
                        if max > min {
                            value.clamp(min, max)
                        } else {
                            value
                        },
                        min,
                        max,
                    ))
                } else {
                    None
                },
                parent: None,
                anchor: Mat4::from_translation(origin)
                    * Mat4::from_mat3(crate::models::rotation(vector(
                        e.get("angles").unwrap_or("0 0 0"),
                    )?)),
                reference: Mat4::from_translation(origin)
                    * Mat4::from_mat3(crate::models::rotation(vector(
                        e.get("angles").unwrap_or("0 0 0"),
                    )?)),
                entity: e,
                model,
                velocity: physics.then_some(Vec3::ZERO),
                enabled,
                killed: false,
                touching: false,
                activator: None,
                next_trigger: 0.,
                outputs,
                motion,
                train: None,
                bounds: local,
            });
        }
        for id in 0..r.states.len() {
            if let Some(parent) = r.states[id]
                .entity
                .get("parentname")
                .and_then(|n| r.names.get(n))
                .and_then(|v| v.first())
                .copied()
            {
                r.states[id].parent = Some(parent);
                r.states[id].anchor = r.states[parent].reference.inverse() * r.states[id].reference;
            }
        }
        // Reject cycles before fixed-step traversal rather than recursing forever.
        for id in 0..r.states.len() {
            let mut next = r.states[id].parent;
            for depth in 0..=64 {
                let Some(n) = next else { break };
                if n == id || depth == 64 {
                    return Err("entity parent cycle/depth limit".into());
                }
                next = r.states[n].parent;
            }
        }
        let originals = r.states.len();
        let physics_count = r.states.iter().filter(|s| s.velocity.is_some()).count();
        let reserve = if physics_count == 0 {
            0
        } else {
            (256usize.saturating_sub(r.origins.len()) / physics_count).min(8)
        };
        for id in 0..originals {
            if r.states[id].velocity.is_none() {
                continue;
            }
            for _ in 0..reserve {
                let mut copy = r.states[id].clone();
                copy.model = Some(r.origins.len());
                copy.killed = true;
                r.origins.push(r.origins[r.states[id].model.unwrap()]);
                r.visible.push(false);
                if let Some(name) = copy.entity.get("targetname") {
                    r.names
                        .entry(name.to_owned())
                        .or_default()
                        .push(r.states.len());
                }
                r.states.push(copy);
            }
        }
        for id in 0..r.states.len() {
            if r.states[id].entity.get("classname") == Some("func_tracktrain") {
                // Brush carriers retain their authored model slots.
                if r.states[id].model.is_none() {
                    return Err("tracktrain requires brush model".into());
                }
                let e = &r.states[id].entity;
                let target = e
                    .get("target")
                    .and_then(|n| r.names.get(n))
                    .and_then(|v| v.first())
                    .copied()
                    .ok_or("train missing initial track")?;
                if r.states[target].entity.get("classname") != Some("path_track") {
                    return Err("train target not path_track".into());
                }
                let next = r.states[target]
                    .entity
                    .get("target")
                    .and_then(|n| r.names.get(n))
                    .and_then(|v| v.first())
                    .copied();
                let offset = e.origin()? - r.states[target].entity.origin()?;
                let speed = number(e, "startspeed", 100.)?.abs();
                r.states[id].train = Some(Train {
                    next,
                    offset,
                    speed,
                    running: false,
                });
            }
        }
        for id in 0..r.states.len() {
            if r.states[id].entity.get("classname") == Some("weapon_portalgun") {
                if r.origins.len() >= 256 {
                    r.unknown_inputs
                        .insert("weapon world model capacity (256)".into());
                    continue;
                }
                r.states[id].model = Some(r.origins.len());
                r.origins.push(r.states[id].entity.origin()?);
                r.visible.push(true);
            }
            if r.states[id].entity.get("classname") == Some("logic_auto") {
                r.fire(id, "OnMapSpawn");
                r.fire(id, "OnNewGame");
            }
        }
        Ok(r)
    }
    fn transform(&self, id: usize) -> Mat4 {
        let s = &self.states[id];
        let mut raw = s.reference;
        if let Some(m) = s.model {
            raw.w_axis = self.origins[m].extend(1.);
        }
        if let Some(m) = s.motion.as_ref().filter(|m| m.angular) {
            raw *= Mat4::from_axis_angle(
                m.travel.normalize_or_zero(),
                (m.travel.length() * m.amount).to_radians(),
            );
        }
        let local = s.anchor * s.reference.inverse() * raw;
        s.parent.map(|p| self.transform(p) * local).unwrap_or(local)
    }
    pub fn entity_transform(&self, name: &str) -> Option<Mat4> {
        self.names
            .get(name)
            .and_then(|v| v.first())
            .map(|&id| self.transform(id))
    }
    pub fn bind_scene_timing(&mut self, durations: &HashMap<u32, f32>) {
        for (id, s) in self.states.iter().enumerate() {
            if s.entity.get("classname") == Some("logic_choreographed_scene") {
                if let Some(duration) = s
                    .entity
                    .get("SceneFile")
                    .and_then(|p| durations.get(&crate::scenes::scene_crc(p)))
                {
                    self.scene_times.insert(id, (*duration, None));
                }
            }
        }
    }
    pub fn entity_model(&self, name: &str) -> Option<usize> {
        self.names
            .get(name)
            .and_then(|v| v.first())
            .and_then(|&id| self.states[id].model)
    }
    pub fn model_transforms(&self) -> (Vec<Vec3>, Vec<Mat3>) {
        let mut origins = self.origins.clone();
        let mut rotations = vec![Mat3::IDENTITY; origins.len()];
        for (id, s) in self.states.iter().enumerate() {
            if let Some(m) = s.model {
                let t = self.transform(id);
                origins[m] = t.w_axis.truncate();
                rotations[m] = Mat3::from_mat4(t) * Mat3::from_mat4(s.reference).transpose();
            }
        }
        (origins, rotations)
    }
    fn fire(&mut self, id: usize, event: &str) {
        self.fire_value(id, event, None);
    }
    fn fire_value(&mut self, id: usize, event: &str, value: Option<&str>) {
        if self.states[id].killed {
            return;
        }
        let activator = self.states[id].activator;
        for o in &mut self.states[id].outputs {
            if !o.event.eq_ignore_ascii_case(event) || o.remaining == 0 {
                continue;
            }
            if self.queue.len() >= 4096 {
                self.unknown_inputs
                    .insert("entity queue limit reached".into());
                break;
            }
            if o.remaining > 0 {
                o.remaining -= 1;
            }
            self.queue.push(Pending {
                event: event.to_owned(),
                due: self.time + o.delay,
                target: o.target.clone(),
                input: o.input.clone(),
                parameter: if o.parameter.is_empty() {
                    value.unwrap_or("").to_owned()
                } else {
                    o.parameter.clone()
                },
                source: id,
                activator,
            });
        }
    }
    /// Bounded global brush-entity handoff. Names/classes are matched, never map-local IDs.
    /// Parent-rendered models follow the transferred carrier through their authored offsets.
    pub fn enter_transition(&mut self, previous: &Runtime, offset: Vec3) {
        self.portal_inventory = previous.portal_inventory;
        self.queue
            .retain(|p| !p.event.eq_ignore_ascii_case("OnNewGame"));
        for id in 0..self.states.len() {
            let s = &self.states[id];
            if let (Some(global), Some(model)) = (s.entity.get("globalname"), s.model) {
                if !global.is_empty() && s.velocity.is_none() {
                    if let Some(old) = previous.states.iter().find(|old| {
                        old.entity.get("globalname") == Some(global)
                            && old.entity.get("classname") == s.entity.get("classname")
                    }) {
                        if let Some(old_model) = old.model {
                            self.origins[model] = previous.origins[old_model] + offset;
                            self.visible[model] = previous.visible[old_model];
                            println!(
                                "ChangeLevel global handoff {global} -> {:?}",
                                self.origins[model]
                            );
                            self.states[id].killed = old.killed;
                            if let Some(train) = self.states[id].train.as_mut() {
                                train.running = false;
                            }
                        }
                    }
                }
            }
            if self.states[id].entity.get("classname") == Some("logic_auto") {
                self.fire(id, "OnMapTransition");
            }
        }
    }
    pub fn is_enabled(&self, name: &str) -> bool {
        self.names
            .get(name)
            .into_iter()
            .flatten()
            .any(|&n| self.states[n].enabled && !self.states[n].killed)
    }
    pub fn input(&mut self, target: &str, input: &str, parameter: &str) {
        if self.queue.len() < 4096 {
            self.queue.push(Pending {
                event: String::new(),
                due: self.time,
                target: target.into(),
                input: input.into(),
                parameter: parameter.into(),
                source: usize::MAX,
                activator: None,
            });
        }
    }
    fn request_changelevel(&mut self, id: usize) {
        if self.change_level.is_some() {
            return;
        }
        let e = &self.states[id].entity;
        let map = e.get("map").unwrap_or("");
        let landmark = e.get("landmark").unwrap_or("");
        if map.is_empty() || !map.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            self.unknown_inputs
                .insert(format!("invalid changelevel map {map}"));
            return;
        }
        let source = self
            .states
            .iter()
            .find(|s| {
                s.entity.get("classname") == Some("info_landmark")
                    && s.entity.get("targetname") == Some(landmark)
            })
            .and_then(|s| s.entity.origin().ok());
        if let Some(source) = source {
            self.change_level = Some(ChangeLevel {
                map: map.into(),
                landmark: landmark.into(),
                source,
            });
            self.fire(id, "OnChangeLevel");
        } else {
            self.unknown_inputs
                .insert(format!("missing source landmark {landmark}"));
        }
    }
    fn apply(&mut self, id: usize, input: &str, parameter: &str, world: &World) {
        if self.states[id].killed {
            return;
        }
        let class = self.states[id]
            .entity
            .get("classname")
            .unwrap_or("")
            .to_owned();
        match input.to_ascii_lowercase().as_str() {
            "invalue" if class == "logic_case" => {
                let matched = (1..=16).find(|n| {
                    self.states[id]
                        .entity
                        .get(&format!("Case{n:02}"))
                        .is_some_and(|v| v.eq_ignore_ascii_case(parameter))
                });
                if let Some(n) = matched {
                    self.fire(id, &format!("OnCase{n:02}"));
                } else {
                    self.fire_value(id, "OnDefault", Some(parameter));
                }
            }
            "add" | "setvalue" if class == "math_counter" => {
                if !self.states[id].enabled {
                    return;
                }
                let Ok(value) = parameter.parse::<f32>() else {
                    return;
                };
                if !value.is_finite() {
                    return;
                }
                let (old, min, max) = self.states[id].counter.unwrap();
                let raw = if input.eq_ignore_ascii_case("add") {
                    old + value
                } else {
                    value
                };
                if !raw.is_finite() {
                    return;
                }
                let next = if max > min { raw.clamp(min, max) } else { raw };
                self.states[id].counter.as_mut().unwrap().0 = next;
                if max > min && next != old {
                    if next == min {
                        self.fire(id, "OnHitMin");
                    }
                    if next == max {
                        self.fire(id, "OnHitMax");
                    }
                }
                self.fire_value(id, "OutValue", Some(&next.to_string()));
            }
            "enable" | "disable" | "toggle" if class == "logic_timer" => {
                let enabled = match input.to_ascii_lowercase().as_str() {
                    "enable" => true,
                    "disable" => false,
                    _ => !self.states[id].enabled,
                };
                if enabled != self.states[id].enabled {
                    self.states[id].enabled = enabled;
                    let timer = self.states[id].timer.as_mut().unwrap();
                    timer.1 = enabled.then_some(self.time + timer.0);
                }
            }
            "refiretime" if class == "logic_timer" => {
                if let Ok(interval) = parameter.parse::<f32>() {
                    if interval.is_finite() && interval > 0. {
                        let enabled = self.states[id].enabled;
                        self.states[id].timer = Some((
                            interval.max(1. / 120.),
                            enabled.then_some(self.time + interval.max(1. / 120.)),
                        ));
                    }
                }
            }
            "resettimer" if class == "logic_timer" => {
                let enabled = self.states[id].enabled;
                let timer = self.states[id].timer.as_mut().unwrap();
                timer.1 = enabled.then_some(self.time + timer.0);
            }
            "start"
                if class == "logic_choreographed_scene" && self.scene_times.contains_key(&id) =>
            {
                let (duration, finish) = self.scene_times.get_mut(&id).unwrap();
                if finish.is_none() {
                    *finish = Some(self.time + *duration);
                    self.fire(id, "OnStart");
                }
            }
            "cancel"
                if class == "logic_choreographed_scene" && self.scene_times.contains_key(&id) =>
            {
                self.scene_times.get_mut(&id).unwrap().1 = None;
                self.fire(id, "OnCanceled");
            }
            "chargeportal1" | "chargeportal2"
                if class == "weapon_portalgun" && self.portal_events.is_some() => {}
            "fireportal1" | "fireportal2"
                if class == "weapon_portalgun" && self.portal_events.is_some() =>
            {
                let index = usize::from(input.eq_ignore_ascii_case("fireportal2"));
                if self.states[id].entity.get(if index == 0 {
                    "CanFirePortal1"
                } else {
                    "CanFirePortal2"
                }) != Some("0")
                    && self.portal_shots.len() < 128
                {
                    let t = self.transform(id);
                    self.portal_shots.push((
                        index,
                        t.w_axis.truncate(),
                        t.transform_vector3(Vec3::X).normalize_or_zero(),
                    ));
                }
            }
            "setparent" | "clearparent" => {
                let parent = if input.eq_ignore_ascii_case("setparent") {
                    let Some(parent) = self.names.get(parameter).and_then(|v| v.first()).copied()
                    else {
                        return;
                    };
                    let mut next = Some(parent);
                    for depth in 0..=64 {
                        let Some(n) = next else { break };
                        if n == id || depth == 64 {
                            self.unknown_inputs
                                .insert("entity parent cycle/depth limit".into());
                            return;
                        }
                        next = self.states[n].parent;
                    }
                    Some(parent)
                } else {
                    None
                };
                let current = self.transform(id);
                let mut raw = self.states[id].reference;
                if let Some(m) = self.states[id].model {
                    raw.w_axis = self.origins[m].extend(1.);
                }
                self.states[id].anchor = parent
                    .map(|p| self.transform(p).inverse())
                    .unwrap_or(Mat4::IDENTITY)
                    * current
                    * raw.inverse()
                    * self.states[id].reference;
                self.states[id].parent = parent;
            }
            "setspeed" if self.states[id].motion.is_some() => {
                if let Ok(speed) = parameter.parse::<f32>() {
                    if speed.is_finite() && speed > 0. {
                        self.states[id].motion.as_mut().unwrap().speed = speed;
                    }
                }
            }
            "changelevel" if class == "trigger_changelevel" => self.request_changelevel(id),
            "forcespawn" if class == "env_entity_maker" => {
                let template = self.states[id]
                    .entity
                    .get("EntityTemplate")
                    .and_then(|n| self.names.get(n))
                    .and_then(|v| v.first())
                    .copied();
                if let Some(template) = template {
                    let targets = self.states[template]
                        .entity
                        .pairs
                        .iter()
                        .filter(|(k, _)| k.starts_with("Template"))
                        .map(|(_, v)| v.clone())
                        .collect::<Vec<_>>();
                    let mut spawned = false;
                    for target in targets {
                        let available = self
                            .names
                            .get(&target)
                            .into_iter()
                            .flatten()
                            .copied()
                            .find(|&n| self.states[n].velocity.is_some() && self.states[n].killed);
                        if let Some(n) = available {
                            let pos = self.states[id].entity.origin().unwrap_or(Vec3::ZERO);
                            // ForceSpawn checks room only with SF_ENTMAKER_CHECK_FOR_SPACE.
                            // Authored makers may intentionally intersect their supporting brush.
                            let flags = self.states[id]
                                .entity
                                .get("spawnflags")
                                .unwrap_or("0")
                                .parse::<u32>()
                                .unwrap_or(0);
                            let bounds = self.states[n].bounds;
                            let center = pos + (bounds.0 + bounds.1) * 0.5;
                            let extent = (bounds.1 - bounds.0) * 0.5;
                            if flags & 8 != 0 && world.overlaps_cuboid(center, extent) {
                                self.fire(id, "OnEntityFailedSpawn");
                                continue;
                            }
                            let m = self.states[n].model.unwrap();
                            self.origins[m] = pos;
                            self.visible[m] = true;
                            self.states[n].killed = false;
                            self.states[n].enabled = true;
                            self.states[n].velocity = Some(Vec3::ZERO);
                            spawned = true;
                        } else {
                            self.unknown_inputs
                                .insert("physics spawn capacity (8 per prototype)".into());
                        }
                    }
                    if spawned {
                        self.fire(id, "OnEntitySpawned");
                    }
                } else {
                    self.unknown_inputs.insert("maker missing template".into());
                }
            }
            "wake" if self.states[id].velocity.is_some() => {}
            "setactivatedstate" | "fizzle"
                if class == "prop_portal" && self.portal_events.is_some() =>
            {
                let active = input.eq_ignore_ascii_case("setactivatedstate") && parameter == "1";
                if let Some(name) = self.states[id].entity.get("targetname") {
                    self.portal_events
                        .as_mut()
                        .unwrap()
                        .push((name.to_owned(), active));
                }
            }
            "startforward" | "stop" if self.states[id].train.is_some() => {
                self.states[id].train.as_mut().unwrap().running =
                    input.eq_ignore_ascii_case("startforward");
            }
            "setdefaultanimation" => {
                if let Some(p) = self.animations.get_mut(&id) {
                    if parameter.is_empty() {
                        p.default = None;
                    } else if let Some(c) =
                        p.clips.iter().find(|c| c.0.eq_ignore_ascii_case(parameter))
                    {
                        p.default = Some(c.0.clone());
                    } else {
                        self.unknown_inputs
                            .insert(format!("{class}:SetDefaultAnimation:{parameter}"));
                    }
                } else {
                    self.unknown_inputs
                        .insert(format!("{class}:SetDefaultAnimation:{parameter}:unbound"));
                }
            }
            "setanimation" => {
                if let Some(p) = self.animations.get_mut(&id) {
                    if let Some(c) = p.clips.iter().find(|c| c.0.eq_ignore_ascii_case(parameter)) {
                        p.sequence = c.0.clone();
                        p.time = 0.;
                        p.completed = false;
                    } else {
                        self.unknown_inputs
                            .insert(format!("{class}:SetAnimation:{parameter}"));
                    }
                } else {
                    self.unknown_inputs
                        .insert(format!("{class}:SetAnimation:{parameter}:unbound"));
                }
            }
            "enable" => self.states[id].enabled = true,
            "disable" => self.states[id].enabled = false,
            "toggle" if class == "func_brush" => self.states[id].enabled = !self.states[id].enabled,
            "kill" => {
                self.states[id].killed = true;
                self.states[id].enabled = false;
            }
            "trigger" if class == "logic_relay" => {
                if self.states[id].enabled {
                    self.fire(id, "OnTrigger");
                }
            }
            "open" | "close" | "toggle" | "press" | "setposition"
                if self.states[id].motion.is_some() =>
            {
                if !self.states[id].enabled {
                    return;
                }
                let motion = self.states[id].motion.as_mut().unwrap();
                let goal = match input.to_ascii_lowercase().as_str() {
                    "close" => 0.,
                    "toggle" => 1. - motion.goal,
                    "setposition" => parameter
                        .parse::<f32>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .unwrap_or(motion.goal)
                        .clamp(0., 1.),
                    _ => 1.,
                };
                let changed =
                    (motion.goal - goal).abs() > 0.001 || input.eq_ignore_ascii_case("press");
                motion.goal = goal;
                motion.close_at = None;
                let button = motion.button;
                if changed {
                    self.fire(
                        id,
                        if button {
                            "OnPressed"
                        } else if goal > 0. {
                            "OnOpen"
                        } else {
                            "OnClose"
                        },
                    );
                }
            }
            _ => {
                self.unknown_inputs.insert(format!("{class}:{input}"));
            }
        }
    }
    fn bounds(&self, id: usize) -> (Vec3, Vec3) {
        let s = &self.states[id];
        let origin = s
            .model
            .map(|m| self.origins[m])
            .unwrap_or_else(|| s.entity.origin().unwrap_or(Vec3::ZERO));
        (s.bounds.0 + origin, s.bounds.1 + origin)
    }
    pub fn throw_held(&mut self, player: &Player) -> bool {
        let Some(id) = self.held.take() else {
            return false;
        };
        self.states[id].velocity = Some(player.velocity + player.forward() * 400.);
        self.fire(id, "OnPhysGunDrop");
        true
    }
    pub fn use_player(&mut self, player: &Player, world: &World) -> bool {
        if self.portal_events.is_some() {
            let eye = player.position + Vec3::Z * player.eye_height();
            let gun = self.states.iter().enumerate().find_map(|(id, s)| {
                if s.killed || !s.enabled || s.entity.get("classname") != Some("weapon_portalgun") {
                    return None;
                }
                let point = self.transform(id).w_axis.truncate();
                let delta = point - eye;
                let (fraction, _) = world.trace_point(eye, point);
                (delta.length() <= 80.
                    && delta.normalize_or_zero().dot(player.forward()) > 0.85
                    && fraction * delta.length() + 0.1 >= delta.length())
                .then_some(id)
            });
            if let Some(id) = gun {
                self.pickup_portalgun(id);
                return true;
            }
        }
        if let Some(id) = self.held.take() {
            self.states[id].velocity = Some(player.velocity);
            self.fire(id, "OnPhysGunDrop");
            return true;
        }
        let eye = player.position + Vec3::Z * player.eye_height();
        let direction = player.forward();
        let mut best = None;
        let mut distance = 96.;
        for (id, state) in self.states.iter().enumerate() {
            if state.killed
                || !state.enabled
                || (state.motion.is_none() && state.velocity.is_none())
            {
                continue;
            }
            let (lo, hi) = self.bounds(id);
            let mut near = 0f32;
            let mut far = 96f32;
            let mut valid = true;
            for a in 0..3 {
                if direction[a].abs() < 0.00001 {
                    if eye[a] < lo[a] || eye[a] > hi[a] {
                        valid = false;
                        break;
                    }
                } else {
                    let t = (lo[a] - eye[a]) / direction[a];
                    let z = (hi[a] - eye[a]) / direction[a];
                    near = near.max(t.min(z));
                    far = far.min(t.max(z));
                }
            }
            if valid && near <= far && near < distance {
                best = Some(id);
                distance = near;
            }
        }
        if let Some(id) = best {
            // The selected body's contact epsilon is not an occluder. Keep
            // world brushes and every other body in the visibility trace.
            let (fraction, _) = world.trace_use(eye, eye + direction * distance, id);
            if fraction * distance + 0.05 < distance {
                return false;
            }
            if self.states[id].velocity.is_some() {
                self.held = Some(id);
                self.fire(id, "OnPlayerPickup");
                return true;
            }
            let button = self.states[id].motion.as_ref().unwrap().button;
            self.apply(id, if button { "Press" } else { "Toggle" }, "", world);
            true
        } else {
            false
        }
    }
    fn pickup_portalgun(&mut self, id: usize) {
        for index in 0..2 {
            self.portal_inventory[index] |= self.states[id].entity.get(if index == 0 {
                "CanFirePortal1"
            } else {
                "CanFirePortal2"
            }) != Some("0");
        }
        self.fire(id, "OnPlayerPickup");
        self.states[id].killed = true;
        self.states[id].enabled = false;
        if let Some(m) = self.states[id].model {
            self.visible[m] = false;
        }
        println!(
            "Portal gun picked up: channels {:?}; authored OnPlayerPickup queued",
            self.portal_inventory
        );
    }
    pub fn tick(&mut self, dt: f32, player: &mut Player, world: &mut World) {
        if !dt.is_finite() || dt <= 0. || dt > 0.1 {
            return;
        }
        self.time += dt;
        // Stable entity order, one scheduled think per simulation step; no wall clock.
        for id in 0..self.states.len() {
            if self.states[id].killed || !self.states[id].enabled {
                continue;
            }
            if let Some((interval, Some(due))) = self.states[id].timer {
                if due <= self.time {
                    self.states[id].timer.as_mut().unwrap().1 = Some(self.time + interval);
                    self.fire(id, "OnTimer");
                }
            }
        }
        let completed: Vec<_> = self
            .scene_times
            .iter_mut()
            .filter_map(|(&id, (_, finish))| {
                if finish.is_some_and(|t| t <= self.time) {
                    *finish = None;
                    Some(id)
                } else {
                    None
                }
            })
            .collect();
        for id in completed {
            self.fire(id, "OnCompletion");
        }
        let colliders = self
            .states
            .iter()
            .enumerate()
            .filter_map(|(id, s)| {
                let m = s.model?;
                if s.killed || s.velocity.is_none() || self.held == Some(id) {
                    return None;
                }
                let lo = self.origins[m] + s.bounds.0;
                let hi = self.origins[m] + s.bounds.1;
                let planes = (0..3)
                    .flat_map(|a| {
                        let mut normal = Vec3::ZERO;
                        normal[a] = 1.;
                        [
                            crate::play::Plane {
                                normal,
                                distance: hi[a],
                            },
                            crate::play::Plane {
                                normal: -normal,
                                distance: -lo[a],
                            },
                        ]
                    })
                    .collect::<Vec<_>>();
                Some((id, planes))
            })
            .collect::<Vec<_>>();
        let trace = |id, start, end, extent| {
            let mut hit = world.trace_body(id, start, end, extent);
            for (other, brush) in &colliders {
                if *other == id {
                    continue;
                }
                let h = World::trace_brushes(
                    std::slice::from_ref(brush),
                    start,
                    end,
                    Vec3::ZERO,
                    extent,
                );
                if h.0 < hit.0 {
                    hit = h;
                }
            }
            hit
        };
        for (id, s) in self.states.iter_mut().enumerate() {
            if s.killed {
                continue;
            }
            if let (Some(model), Some(velocity)) = (s.model, s.velocity.as_mut()) {
                let offset = (s.bounds.0 + s.bounds.1) * 0.5;
                let extent = (s.bounds.1 - s.bounds.0) * 0.5;
                if self.held == Some(id) {
                    let eye = player.position + Vec3::Z * player.eye_height();
                    let mut target = eye + player.forward() * 64. + offset;
                    let mut position = self.origins[model] + offset;
                    // The grab ray and held body live in different spaces once
                    // the ray passes through a portal. Sweep to the entry before
                    // mapping; never drag the cube through the intervening map.
                    if let [Some(a), Some(b)] = world.portals {
                        for (src, dst) in [(a, b), (b, a)] {
                            let d0 = src.normal.dot(eye - src.center);
                            let d1 = src.normal.dot(target - src.center);
                            if d0 > 0. && d1 <= 0. {
                                let at = eye.lerp(target, d0 / (d0 - d1));
                                if !src.fits(at, extent) {
                                    continue;
                                }
                                let mapped = src.map_point(dst, at) + dst.normal * 0.05;
                                if !dst.fits(mapped, extent)
                                    || world.body_overlap(id, mapped, extent)
                                {
                                    continue;
                                }
                                if position.distance(src.center) < position.distance(dst.center) {
                                    let (f, _) = trace(id, position, at, extent);
                                    if f < 1. {
                                        continue;
                                    }
                                    position = mapped;
                                }
                                target = src.map_point(dst, target);
                                break;
                            }
                        }
                    }
                    let mut delta = target - position;
                    for _ in 0..4 {
                        let (f, n) = trace(id, position, position + delta, extent);
                        position += delta * f;
                        if f >= 1. {
                            break;
                        }
                        delta *= 1. - f;
                        delta -= n * delta.dot(n).min(0.);
                    }
                    self.origins[model] = position - offset;
                    *velocity = Vec3::ZERO;
                    continue;
                }
                velocity.z -= 600. * dt;
                let start = self.origins[model] + offset;
                let mut position = start;
                let mut remaining = dt;
                for _ in 0..4 {
                    let end = position + *velocity * remaining;
                    let (f, n) = trace(id, position, end, extent);
                    let mut crossed = false;
                    if let [Some(a), Some(b)] = world.portals {
                        for (src, dst) in [(a, b), (b, a)] {
                            let d0 = src.normal.dot(position - src.center);
                            let d1 = src.normal.dot(end - src.center);
                            if d0 > 0. && d1 <= 0. {
                                let t = d0 / (d0 - d1);
                                let at = position.lerp(end, t);
                                let mapped = src.map_point(dst, at) + dst.normal * 0.05;
                                if t <= f + 0.00001
                                    && src.fits(at, extent)
                                    && dst.fits(mapped, extent)
                                    && !world.body_overlap(id, mapped, extent)
                                {
                                    position = mapped;
                                    *velocity = src.map_vector(dst, *velocity);
                                    remaining *= 1. - t;
                                    crossed = true;
                                    break;
                                }
                            }
                        }
                    }
                    if crossed {
                        continue;
                    }
                    position = position.lerp(end, f);
                    if f >= 1. {
                        break;
                    }
                    remaining *= 1. - f;
                    *velocity -= n * velocity.dot(n).min(0.);
                    if n.z > 0.7 {
                        velocity.x *= 0.95;
                        velocity.y *= 0.95;
                    }
                }
                self.origins[model] = position - offset;
            }
        }
        let (ids, brushes): (Vec<_>, Vec<_>) = self
            .states
            .iter()
            .enumerate()
            .filter_map(|(id, s)| {
                let m = s.model?;
                if s.killed || s.velocity.is_none() || self.held == Some(id) {
                    return None;
                }
                let lo = self.origins[m] + s.bounds.0;
                let hi = self.origins[m] + s.bounds.1;
                Some((
                    id,
                    (0..3)
                        .flat_map(|a| {
                            let mut normal = Vec3::ZERO;
                            normal[a] = 1.;
                            [
                                crate::play::Plane {
                                    normal,
                                    distance: hi[a],
                                },
                                crate::play::Plane {
                                    normal: -normal,
                                    distance: -lo[a],
                                },
                            ]
                        })
                        .collect(),
                ))
            })
            .unzip();
        world.rigid_ids = ids;
        world.rigid = brushes;
        for id in 0..self.states.len() {
            let class = self.states[id]
                .entity
                .get("classname")
                .unwrap_or("")
                .to_owned();
            if class == "trigger_portal_cleanser" && self.portal_events.is_some() {
                let s = &self.states[id];
                if s.killed || !s.enabled {
                    continue;
                }
                let (lo, hi) = self.bounds(id);
                let pmin = player.position - Vec3::new(16., 16., 0.);
                let pmax = player.position + Vec3::new(16., 16., player.height());
                if pmin.cmplt(hi).all() && pmax.cmpgt(lo).all() {
                    self.cleanse_portals = true;
                }
                let dissolved: Vec<_> = (0..self.states.len())
                    .filter(|&n| {
                        let s = &self.states[n];
                        if s.killed || s.velocity.is_none() {
                            return false;
                        }
                        let (a, b) = self.bounds(n);
                        a.cmplt(hi).all() && b.cmpgt(lo).all()
                    })
                    .collect();
                for n in dissolved {
                    println!(
                        "Portal cleanser {} dissolved {} at {:?}",
                        self.states[id]
                            .entity
                            .get("targetname")
                            .unwrap_or("<unnamed>"),
                        self.states[n]
                            .entity
                            .get("targetname")
                            .unwrap_or("<unnamed>"),
                        self.bounds(n)
                    );
                    self.states[n].killed = true;
                    self.states[n].velocity = Some(Vec3::ZERO);
                    if let Some(m) = self.states[n].model {
                        self.visible[m] = false;
                    }
                    if self.held == Some(n) {
                        self.held = None;
                    }
                    self.states[id].activator = Some(n);
                    self.fire(id, "OnDissolve");
                    self.fire(id, "OnDissolveBox");
                    if let Some(index) = world.rigid_ids.iter().position(|&v| v == n) {
                        world.rigid_ids.remove(index);
                        world.rigid.remove(index);
                    }
                }
                continue;
            }
            if class == "trigger_changelevel" {
                let s = &self.states[id];
                let flags = s
                    .entity
                    .get("spawnflags")
                    .unwrap_or("0")
                    .parse::<u32>()
                    .unwrap_or(0);
                let (lo, hi) = self.bounds(id);
                if !s.killed
                    && s.enabled
                    && flags & 2 == 0
                    && (player.position - Vec3::new(16., 16., 0.)).cmplt(hi).all()
                    && (player.position + Vec3::new(16., 16., player.height()))
                        .cmpgt(lo)
                        .all()
                {
                    self.request_changelevel(id);
                }
                continue;
            }
            if !["trigger_once", "trigger_multiple", "trigger_teleport"].contains(&class.as_str()) {
                continue;
            }
            let s = &self.states[id];
            let flags = s
                .entity
                .get("spawnflags")
                .unwrap_or("0")
                .parse::<u32>()
                .unwrap_or(0);
            let (lo, hi) = self.bounds(id);
            let pmin = player.position - Vec3::new(16., 16., 0.);
            let pmax = player.position + Vec3::new(16., 16., player.height());
            let mut cube_activator = None;
            let cube_inside = flags & 8 != 0
                && self.states.iter().enumerate().any(|(cube_id, cube)| {
                    if cube.velocity.is_none() || cube.killed || self.held == Some(cube_id) {
                        return false;
                    }
                    let matches = s.entity.get("filtername").is_none_or(|name| {
                        self.names
                            .get(name)
                            .and_then(|v| v.first())
                            .is_some_and(|&filter| {
                                let f = &self.states[filter].entity;
                                f.get("classname") == Some("filter_activator_name")
                                    && f.get("filtername") == cube.entity.get("targetname")
                            })
                    });
                    let (a, b) = self.bounds(cube_id);
                    let hit = matches && a.cmplt(hi).all() && b.cmpgt(lo).all();
                    if hit {
                        cube_activator = Some(cube_id);
                    }
                    hit
                });
            let player_inside = flags & 1 != 0
                && s.entity.get("filtername").is_none()
                && pmin.cmplt(hi).all()
                && pmax.cmpgt(lo).all();
            let inside = !s.killed && s.enabled && (player_inside || cube_inside);
            let before = s.touching;
            if cube_activator.is_some() {
                self.states[id].activator = cube_activator;
            }
            self.states[id].touching = inside;
            if inside && !before {
                self.fire(id, "OnStartTouch");
                self.fire(id, "OnStartTouchAll");
                if class == "trigger_teleport" {
                    if let Some(target) = self.states[id].entity.get("target") {
                        if let Some(&destination) = self.names.get(target).and_then(|v| v.first()) {
                            if let Ok(origin) = self.states[destination].entity.origin() {
                                player.position = origin;
                                player.velocity = Vec3::ZERO;
                            }
                        }
                    }
                }
            }
            if inside && self.time >= self.states[id].next_trigger {
                self.fire(id, "OnTrigger");
                let wait = self.states[id]
                    .entity
                    .get("wait")
                    .and_then(|s| s.parse::<f32>().ok())
                    .filter(|x| x.is_finite())
                    .unwrap_or(0.2)
                    .max(0.01);
                self.states[id].next_trigger = self.time + wait;
                if class == "trigger_once" {
                    self.states[id].enabled = false;
                }
            }
            if !inside && before {
                self.fire(id, "OnEndTouch");
                self.fire(id, "OnEndTouchAll");
            }
        }
        for _ in 0..1024 {
            let due = self
                .queue
                .iter()
                .enumerate()
                .filter(|(_, p)| p.due <= self.time + 0.000001)
                .min_by(|(_, a), (_, b)| a.due.total_cmp(&b.due))
                .map(|(n, _)| n);
            let Some(n) = due else {
                break;
            };
            let event = self.queue.remove(n);
            let ids = if event.target == "!self" {
                if event.source < self.states.len() {
                    vec![event.source]
                } else {
                    vec![]
                }
            } else if event.target == "!activator" {
                event
                    .activator
                    .into_iter()
                    .filter(|&n| n < self.states.len())
                    .collect()
            } else {
                self.names.get(&event.target).cloned().unwrap_or_default()
            };
            for id in ids {
                self.states[id].activator = event.activator;
                self.apply(id, &event.input, &event.parameter, world);
            }
        }
        let mut completed = vec![];
        for (&id, p) in &mut self.animations {
            if self.states[id].killed {
                continue;
            }
            p.time += dt;
            if let Some(c) = p.clips.iter().find(|c| c.0 == p.sequence) {
                if !c.2 && !p.completed && p.time + 0.000001 >= c.1 {
                    p.completed = true;
                    p.done_count += 1;
                    p.time = c.1;
                    if let Some(default) = &p.default {
                        p.sequence = default.clone();
                        p.time = 0.;
                    }
                    completed.push(id);
                }
            }
        }
        for id in completed {
            self.fire(id, "OnAnimationDone");
        }
        let mut changed = false;
        let old_origins = self.origins.clone();
        let mut old_amounts = vec![None; self.states.len()];
        let mut events = vec![];
        let support = if player.grounded && !player.noclip && player.velocity.z <= 0.1 {
            world.support_model(player.position, player.height())
        } else {
            None
        };
        let old_player = player.position;
        let mut old_trains = vec![];
        for id in 0..self.states.len() {
            let Some(train) = self.states[id].train.as_ref() else {
                continue;
            };
            if !train.running || self.states[id].killed || !self.states[id].enabled {
                continue;
            }
            let model = self.states[id].model.unwrap();
            let mut next = train.next;
            let offset = train.offset;
            let mut budget = train.speed * dt;
            let mut pos = self.origins[model];
            old_trains.push((id, next, train.running));
            for _ in 0..64 {
                let Some(node) = next else { break };
                let target = self.states[node].entity.origin().unwrap_or(pos) + offset;
                let distance = pos.distance(target);
                if budget + 0.00001 < distance {
                    pos += (target - pos).normalize_or_zero() * budget;
                    break;
                }
                pos = target;
                budget = (budget - distance).max(0.);
                events.push((node, "OnPass"));
                next = self.states[node]
                    .entity
                    .get("target")
                    .and_then(|n| self.names.get(n))
                    .and_then(|v| v.first())
                    .copied();
                if budget <= 0. {
                    break;
                }
            }
            let train = self.states[id].train.as_mut().unwrap();
            train.next = next;
            if next.is_none() {
                train.running = false;
            }
            self.origins[model] = pos;
            changed |= pos != old_origins[model];
        }
        for (id, s) in self.states.iter_mut().enumerate() {
            if let Some(model) = s.model {
                let visible =
                    !s.killed && (s.entity.get("classname") != Some("func_brush") || s.enabled);
                if self.visible[model] != visible {
                    self.visible[model] = visible;
                    changed = true;
                }
            }
            if s.killed {
                continue;
            }
            let Some(m) = s.motion.as_mut() else {
                continue;
            };
            if m.close_at.is_some_and(|time| self.time >= time) {
                m.goal = 0.;
                m.close_at = None;
                events.push((id, "OnClose"));
            }
            if (m.amount - m.goal).abs() < 0.00001 {
                continue;
            }
            old_amounts[id] = Some(m.amount);
            let length = m.travel.length().max(0.001);
            let delta = m.speed * dt / length;
            m.amount = if m.goal > m.amount {
                (m.amount + delta).min(m.goal)
            } else {
                (m.amount - delta).max(m.goal)
            };
            if !m.angular {
                self.origins[s.model.unwrap()] = m.base + m.travel * m.amount;
            }
            changed = true;
            if (m.amount - m.goal).abs() < 0.00001 {
                events.push((
                    id,
                    if m.amount > 0. {
                        "OnFullyOpen"
                    } else {
                        "OnFullyClosed"
                    },
                ));
                if m.amount > 0. && m.wait >= 0. {
                    m.close_at = Some(self.time + m.wait);
                }
            }
        }
        if changed {
            if let Some(model) = support {
                player.position += self.origins[model] - old_origins[model];
            }
            let (origins, rotations) = self.model_transforms();
            world.update_transforms(&origins, &self.visible, &rotations);
            if world.overlaps(player.position, player.height()) {
                // Transactional movement: never crush a carried rider or emit rejected OnPass.
                player.position = old_player;
                for (id, next, running) in old_trains {
                    let t = self.states[id].train.as_mut().unwrap();
                    t.next = next;
                    t.running = running;
                    self.fire(id, "OnBlocked");
                }
                self.origins = old_origins;
                for (id, amount) in old_amounts.into_iter().enumerate() {
                    if let Some(amount) = amount {
                        self.states[id].motion.as_mut().unwrap().amount = amount;
                        self.fire(id, "OnBlocked");
                    }
                }
                let (origins, rotations) = self.model_transforms();
                world.update_transforms(&origins, &self.visible, &rotations);
                events.clear();
            }
        }
        for (id, event) in events {
            self.fire(id, event);
        }
    }
}
#[derive(Clone, Debug)]
pub struct Entity {
    pub pairs: Vec<(String, String)>,
}
impl Entity {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }
    pub fn outputs(&self, key: &str) -> Vec<&str> {
        self.pairs
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
            .collect()
    }
    pub fn origin(&self) -> Result<Vec3, String> {
        vector(self.get("origin").unwrap_or("0 0 0"))
    }
    pub fn model(&self) -> Result<Option<usize>, String> {
        self.get("model")
            .filter(|s| s.starts_with('*'))
            .map(|s| s[1..].parse::<usize>().map_err(|e| e.to_string()))
            .transpose()
    }
}
pub fn vector(s: &str) -> Result<Vec3, String> {
    let v = s
        .split_whitespace()
        .map(|s| s.parse::<f32>().map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    if v.len() != 3 || v.iter().any(|x| !x.is_finite()) {
        return Err("invalid entity vector".into());
    }
    Ok(Vec3::new(v[0], v[1], v[2]))
}
pub fn parse_entities(bytes: &[u8]) -> Result<Vec<Entity>, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    let tokens = crate::kv::tokens(text)?;
    let mut p = 0;
    let mut out = vec![];
    while p < tokens.len() {
        if tokens[p] != "{" {
            return Err("entity missing opening brace".into());
        }
        p += 1;
        let mut pairs = vec![];
        while p < tokens.len() && tokens[p] != "}" {
            let key = tokens[p].clone();
            p += 1;
            let value = tokens.get(p).ok_or("entity missing value")?.clone();
            if matches!(key.as_str(), "{" | "}") || matches!(value.as_str(), "{" | "}") {
                return Err("invalid entity key/value".into());
            }
            p += 1;
            pairs.push((key, value));
        }
        if p == tokens.len() {
            return Err("unterminated entity".into());
        }
        p += 1;
        out.push(Entity { pairs });
    }
    Ok(out)
}
