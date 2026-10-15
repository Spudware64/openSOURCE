//! Opt-in scripted simulation-input route, never normal game rules.
use crate::{
    entity::Runtime,
    play::{Player, World},
    portal::Game,
};
use glam::Vec3;
#[derive(Default)]
pub struct Route {
    pub phase: usize,
    pub ticks: u32,
    pub full: bool,
    phase_ticks: u32,
    launched: bool,
}
impl Route {
    fn advance(&mut self, p: &Player) {
        println!(
            "chamber01 route phase {} complete at {:?}; grounded {} crossings {}",
            self.phase, p.position, p.grounded, p.portal_crossings
        );
        self.phase += 1;
        self.phase_ticks = 0;
        self.launched = false;
    }
    pub fn tick(
        &mut self,
        p: &mut Player,
        r: &mut Runtime,
        w: &mut World,
        g: &mut Game,
    ) -> Result<bool, String> {
        if (self.phase == 15 && !self.full) || self.phase == 32 {
            return Ok(true);
        }
        self.ticks += 1;
        self.phase_ticks += 1;
        if self.phase_ticks > 4000 || self.ticks > 24000 || p.noclip || p.position.z < -300. {
            return Err(format!(
                "chamber01 route failed phase {} tick {} position {:?}",
                self.phase,
                self.ticks,
                (
                    p.position,
                    r.entity_transform("a01-middleelevator_body"),
                    r.unknown_inputs.clone()
                )
            ));
        }
        let targets = [
            Vec3::new(432., 64., 0.),
            Vec3::new(432., 360., 0.),
            Vec3::new(432., 480., 0.),
            Vec3::new(96., 480., 0.),
            Vec3::new(96., 544., 0.),
        ];
        let target = match self.phase {
            0..=4 => Some(targets[self.phase]),
            5 => Some(Vec3::new(96., 600., 0.)),
            6 => Some(Vec3::new(96., 200., 0.)),
            10 => Some(Vec3::new(288., 200., 0.)),
            11 => Some(Vec3::new(350., 200., 0.)),
            12 => Some(Vec3::new(-256., 144., 0.)),
            13 => Some(Vec3::new(-256., 32., 0.)),
            15 => Some(Vec3::new(-256., -384., 0.)),
            17 => Some(Vec3::new(-256., -16., 576.)),
            18 => Some(Vec3::new(-448., -16., 576.)),
            19 => Some(Vec3::new(-608., 128., 576.)),
            21 => Some(Vec3::new(-668., 256., 576.)),
            22 => Some(Vec3::new(-752., 256., 576.)),
            23 => Some(Vec3::new(-608., 896., 576.)),
            25 => Some(Vec3::new(-608., 1048., 576.)),
            26 => Some(Vec3::new(-608., 1120., 576.)),
            27 => Some(Vec3::new(-1472., 896., 576.)),
            28 => Some(Vec3::new(-1664., 896., 576.)),
            29 => Some(Vec3::new(-1920., 896., 576.)),
            _ => None,
        };
        let mut input = Vec3::ZERO;
        let mut jump = false;
        if let Some(target) = target {
            let delta = target - p.position;
            if ![5, 11, 22, 26].contains(&self.phase) && delta.truncate().length() < 4. {
                self.advance(p);
            } else {
                p.yaw = delta.y.atan2(delta.x);
                p.pitch = 0.;
                let ready = self.phase != 5
                    || self.launched
                    || g.portals[0].is_some_and(|q| q.center.y > 570.);
                if self.phase == 5 && !ready {
                    // Hold the run-up mark while waiting for the rotating gun;
                    // friction no longer stops a player in a single tick.
                    let mark = Vec3::new(96., 544., p.position.z);
                    let correction = mark - p.position - p.velocity * 0.15;
                    if correction.truncate().length() > 1. {
                        p.yaw = correction.y.atan2(correction.x);
                        input = Vec3::X;
                    }
                } else if ready {
                    input = Vec3::X;
                }
                jump = (self.phase == 5 && ready && !self.launched)
                    || (self.phase == 11 && self.phase_ticks == 1)
                    || (self.phase == 15 && p.position.y < -250. && !self.launched)
                    || ([22, 26].contains(&self.phase) && self.phase_ticks == 1)
                    || (self.phase == 29 && p.position.x < -1780. && !self.launched);
                if jump {
                    self.launched = true;
                }
            }
        } else if self.phase == 7 {
            let gun = r
                .entity_transform("portalgun")
                .ok_or("gun entity absent")?
                .w_axis
                .truncate();
            let d = (gun - p.position - Vec3::Z * p.eye_height()).normalize();
            p.yaw = d.y.atan2(d.x);
            p.pitch = d.z.asin();
            if self.phase_ticks >= 90 {
                if !r.use_player(p, w) || r.portal_inventory != [true, false] {
                    return Err("actual gun pickup/channel gating failed".into());
                }
                self.advance(p);
            }
        } else if self.phase == 8 && self.phase_ticks >= 120 {
            self.advance(p);
        } else if self.phase == 9 {
            let eye = p.position + Vec3::Z * p.eye_height();
            let direction = (Vec3::new(320., 200., -72.) - eye).normalize();
            p.yaw = direction.y.atan2(direction.x);
            p.pitch = direction.z.asin();
            g.fire_player(0, eye, p.forward(), r, w)?;
            if g.fire_player(1, eye, p.forward(), r, w).is_ok() {
                return Err("unowned orange channel fired".into());
            }
            self.advance(p);
        } else if self.phase == 14 && self.phase_ticks >= 120 {
            if p.portal_crossings != 2
                || !p.grounded
                || p.position.y > 40.
                || r.origins[10].x < -190.
                || r.is_enabled("fire_rotating_portalgun_tca01")
            {
                return Err("chamber01 final route acceptance failed".into());
            }
            p.yaw = std::f32::consts::FRAC_PI_2;
            p.pitch = 0.;
            self.advance(p);
            if !self.full {
                return Ok(true);
            }
        } else if self.phase == 16 && p.position.z > 575. && self.phase_ticks > 1200 {
            self.advance(p);
        } else if [20, 24].contains(&self.phase) {
            let target = if self.phase == 20 {
                Vec3::new(-704., 256., 632.)
            } else {
                Vec3::new(-1600., 768., 632.)
            };
            let eye = p.position + Vec3::Z * p.eye_height();
            let d = (target - eye).normalize();
            p.yaw = d.y.atan2(d.x);
            p.pitch = d.z.asin();
            g.fire_player(0, eye, p.forward(), r, w)?;
            self.advance(p);
        } else if self.phase == 31 && self.phase_ticks >= 240 {
            if !p.grounded || p.noclip || r.portal_inventory != [true, false] {
                return Err("chamber02 arrival acceptance failed".into());
            }
            self.advance(p);
            return Ok(true);
        }
        p.step(w, input, jump, 1. / 120.);
        r.tick(1. / 120., p, w);
        g.sync_runtime(r, w);
        if self.phase == 30 && r.change_level.is_some() {
            self.advance(p);
            return Ok(true);
        }
        if (self.phase == 5 && p.portal_crossings == 1)
            || (self.phase == 11 && p.portal_crossings == 2)
            || (self.phase == 22 && p.portal_crossings == 3)
            || (self.phase == 26 && p.portal_crossings == 4)
        {
            self.advance(p);
        }
        Ok(false)
    }
}
