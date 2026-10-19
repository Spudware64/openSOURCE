//! Opt-in CPU wall-clock profiling. No GPU timestamps or waits are inserted.
use std::{
    cell::RefCell,
    fs::File,
    io::{self, Write},
    time::Instant,
};
const STAGES: usize = 13;
const COUNTERS: usize = 14;
const NAMES: [&str; STAGES] = [
    "simulation",
    "animate",
    "vertex_upload",
    "other_uploads",
    "acquire",
    "portal_encode",
    "main_encode",
    "submit",
    "capture",
    "present",
    "runtime_tick_nested",
    "world_update_nested",
    "collider_rebuild_nested",
];
const COUNTS: [&str; COUNTERS] = [
    "ticks",
    "changed_instances",
    "changed_vertices",
    "vertex_bytes",
    "vertex_writes",
    "world_updates",
    "collider_rebuilds",
    "main_scene_views",
    "remote_scene_views",
    "main_scene_draws",
    "remote_scene_draws",
    "main_submitted_vertices",
    "remote_submitted_vertices",
    "translucent_triangle_draws",
];
#[derive(Default)]
struct Bucket {
    frames: u64,
    frame_sum: f64,
    frame_max: f64,
    stage_sum: [f64; STAGES],
    stage_max: [f64; STAGES],
    count_sum: [u64; COUNTERS],
    count_max: [u64; COUNTERS],
    histogram: [u64; 8],
}
impl Bucket {
    fn record(&mut self, frame: f64, stages: &[f64; STAGES], counts: &[u64; COUNTERS]) {
        self.frames += 1;
        self.frame_sum += frame;
        self.frame_max = self.frame_max.max(frame);
        let bin = [8., 16.7, 33.4, 50., 100., 250., 1000.]
            .iter()
            .position(|v| frame <= *v)
            .unwrap_or(7);
        self.histogram[bin] += 1;
        for i in 0..STAGES {
            self.stage_sum[i] += stages[i];
            self.stage_max[i] = self.stage_max[i].max(stages[i]);
        }
        for i in 0..COUNTERS {
            self.count_sum[i] += counts[i];
            self.count_max[i] = self.count_max[i].max(counts[i]);
        }
    }
    fn write(&self, out: &mut impl Write, state: &str) -> io::Result<()> {
        if self.frames == 0 {
            return Ok(());
        }
        write!(
            out,
            "{state},{},{:.3},{:.3}",
            self.frames,
            self.frame_sum / self.frames as f64,
            self.frame_max
        )?;
        for i in 0..STAGES {
            write!(
                out,
                ",{:.3},{:.3}",
                self.stage_sum[i] / self.frames as f64,
                self.stage_max[i]
            )?;
        }
        for i in 0..COUNTERS {
            write!(
                out,
                ",{:.2},{}",
                self.count_sum[i] as f64 / self.frames as f64,
                self.count_max[i]
            )?;
        }
        for n in self.histogram {
            write!(out, ",{n}")?;
        }
        writeln!(out)
    }
}
struct Profile {
    file: File,
    epoch: Instant,
    mark: Instant,
    stages: [f64; STAGES],
    counts: [u64; COUNTERS],
    buckets: [Bucket; 2],
}
thread_local! { static PROFILE: RefCell<Option<Profile>> = const { RefCell::new(None) }; }
pub fn init() {
    let Some(path) = std::env::var_os("SOURCE_FRAME_PROFILE") else {
        return;
    };
    let result = (|| -> io::Result<Profile> {
        let mut file = File::create(path)?;
        writeln!(file, "# CPU wall ms only; nested timings overlap simulation; frame_work excludes event-loop idle; includes present; build={}", option_env!("SOURCE_BUILD_GIT").unwrap_or("standalone"))?;
        write!(file, "state,frames,frame_work_avg_ms,frame_work_max_ms")?;
        for name in NAMES {
            write!(file, ",{name}_avg_ms,{name}_max_ms")?;
        }
        for name in COUNTS {
            write!(file, ",{name}_avg,{name}_max")?;
        }
        writeln!(file, ",le8,le16_7,le33_4,le50,le100,le250,le1000,gt1000")?;
        let now = Instant::now();
        Ok(Profile {
            file,
            epoch: now,
            mark: now,
            stages: [0.; STAGES],
            counts: [0; COUNTERS],
            buckets: Default::default(),
        })
    })();
    match result {
        Ok(p) => PROFILE.with(|v| *v.borrow_mut() = Some(p)),
        Err(e) => eprintln!("SOURCE_FRAME_PROFILE disabled: {e}"),
    }
}
pub fn begin() {
    PROFILE.with(|v| {
        if let Some(p) = v.borrow_mut().as_mut() {
            p.stages.fill(0.);
            p.counts.fill(0);
            p.mark = Instant::now();
        }
    });
}
pub fn mark(stage: usize) {
    PROFILE.with(|v| {
        if let Some(p) = v.borrow_mut().as_mut() {
            let now = Instant::now();
            p.stages[stage] += (now - p.mark).as_secs_f64() * 1000.;
            p.mark = now;
        }
    });
}
pub fn count(index: usize, n: u64) {
    PROFILE.with(|v| {
        if let Some(p) = v.borrow_mut().as_mut() {
            p.counts[index] += n;
        }
    });
}
/// Count API submissions, including zero-vertex pellet draws, not GPU execution.
/// Called only after the shared portal visibility gate admits a remote pass.
pub fn scene_view(view: usize, vertices: u64, translucent: u64, pellets: u64, opaque_draws: u64) {
    PROFILE.with(|v| {
        if let Some(p) = v.borrow_mut().as_mut() {
            let remote = usize::from(view != 0);
            p.counts[7 + remote] += 1;
            p.counts[9 + remote] += opaque_draws + 2 + translucent;
            // Count selected opaque ranges, then sorted translucent triangles.
            p.counts[11 + remote] += vertices + translucent * 3 + pellets * 2;
            p.counts[13] += translucent;
        }
    });
}
pub struct Scope {
    stage: usize,
    start: Option<Instant>,
}
pub fn scope(stage: usize) -> Scope {
    Scope {
        stage,
        start: PROFILE.with(|v| v.borrow().as_ref().map(|_| Instant::now())),
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            PROFILE.with(|v| {
                if let Some(p) = v.borrow_mut().as_mut() {
                    p.stages[self.stage] += start.elapsed().as_secs_f64() * 1000.;
                }
            });
        }
    }
}
pub fn finish() {
    PROFILE.with(|v| {
        let mut slot = v.borrow_mut();
        let Some(p) = slot.as_mut() else {
            return;
        };
        let frame_ms = p.stages[..10].iter().sum();
        p.buckets[usize::from(p.counts[1] > 0)].record(frame_ms, &p.stages, &p.counts);
        if p.epoch.elapsed().as_secs_f64() >= 1. {
            let result = p.buckets[0]
                .write(&mut p.file, "static")
                .and_then(|_| p.buckets[1].write(&mut p.file, "active"))
                .and_then(|_| p.file.flush());
            if let Err(e) = result {
                eprintln!("SOURCE_FRAME_PROFILE write failed: {e}");
                *slot = None;
                return;
            }
            p.buckets = Default::default();
            p.epoch = Instant::now();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_marks_nested_scopes_flush_both_buckets_to_disk() {
        let path =
            std::env::temp_dir().join(format!("source-profile-test-{}.csv", std::process::id()));
        let now = Instant::now();
        PROFILE.with(|v| {
            *v.borrow_mut() = Some(Profile {
                file: File::create(&path).unwrap(),
                epoch: now,
                mark: now,
                stages: [0.; STAGES],
                counts: [0; COUNTERS],
                buckets: Default::default(),
            })
        });
        begin();
        {
            let _runtime = scope(10);
            count(0, 2);
        }
        mark(0);
        finish();
        begin();
        count(1, 3);
        count(3, 1234);
        {
            let _rebuild = scope(12);
            count(6, 1);
        }
        mark(1);
        PROFILE.with(|v| {
            v.borrow_mut().as_mut().unwrap().epoch =
                Instant::now() - std::time::Duration::from_secs(2)
        });
        finish();
        let csv = std::fs::read_to_string(&path).unwrap();
        let rows: Vec<_> = csv.lines().collect();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("static,1,"));
        assert!(rows[1].starts_with("active,1,"));
        assert!(rows[1].contains(",1234.00,1234,"));
        PROFILE.with(|v| {
            let p = v.borrow();
            let p = p.as_ref().unwrap();
            assert_eq!(p.buckets[0].frames, 0);
            assert_eq!(p.buckets[1].frames, 0);
        });
        PROFILE.with(|v| *v.borrow_mut() = None);
        begin();
        mark(0);
        count(0, 1);
        drop(scope(10));
        finish();
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn render_workload_tracks_submitted_views_and_repeated_vertices() {
        let path =
            std::env::temp_dir().join(format!("source-workload-test-{}.csv", std::process::id()));
        let now = Instant::now();
        PROFILE.with(|v| {
            *v.borrow_mut() = Some(Profile {
                file: File::create(&path).unwrap(),
                epoch: now,
                mark: now,
                stages: [0.; STAGES],
                counts: [0; COUNTERS],
                buckets: Default::default(),
            })
        });
        begin();
        scene_view(0, 300, 4, 6, 1);
        scene_view(2, 600, 8, 0, 1);
        PROFILE.with(|v| {
            let p = v.borrow();
            let counts = &p.as_ref().unwrap().counts;
            assert_eq!(&counts[7..], &[1, 1, 7, 11, 324, 624, 12]);
        });
        PROFILE.with(|v| *v.borrow_mut() = None);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn summaries_separate_static_and_active_and_keep_worst_case() {
        let mut b = Bucket::default();
        b.record(10.0, &[2.0; STAGES], &[1; COUNTERS]);
        b.record(30.0, &[4.0; STAGES], &[3; COUNTERS]);
        assert_eq!(b.frames, 2);
        assert_eq!(b.frame_max, 30.0);
        assert_eq!(b.stage_max[0], 4.0);
        assert_eq!(b.count_max[0], 3);
        let mut output = Vec::new();
        b.write(&mut output, "active").unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("active,2,20.000,30.000"), "{text}");
        assert_eq!(Bucket::default().frames, 0);
    }
}
