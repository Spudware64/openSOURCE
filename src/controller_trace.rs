//! Opt-in, bounded CSV diagnostics. No simulation behavior changes.
use std::io::{self, Write};
fn tick_limit(seconds: Option<&str>) -> Result<usize, String> {
    seconds
        .unwrap_or("120")
        .parse::<usize>()
        .ok()
        .filter(|s| (1..=3600).contains(s))
        .map(|s| s * 120)
        .ok_or_else(|| "SOURCE_CONTROLLER_TRACE_SECONDS must be an integer from 1 to 3600".into())
}
pub(crate) struct Recorder<W: Write> {
    output: W,
    remaining: usize,
}
impl<W: Write> Recorder<W> {
    fn row(&mut self, row: &str) -> io::Result<bool> {
        writeln!(self.output, "{row}")?;
        self.remaining -= 1;
        if self.remaining % 120 == 0 {
            self.output.flush()?;
        }
        Ok(self.remaining == 0)
    }
}
pub(crate) fn csv(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}
pub(crate) fn record<W: Write>(recorder: &mut Option<Recorder<W>>, make: impl FnOnce() -> String) {
    if let Some(active) = recorder.as_mut() {
        match active.row(&make()) {
            Ok(false) => (),
            Ok(true) => {
                eprintln!("controller trace complete (configured simulation tick cap)");
                *recorder = None;
            }
            Err(e) => {
                eprintln!("controller trace disabled after write error: {e}");
                *recorder = None;
            }
        }
    }
}
pub(crate) fn open() -> Result<Option<Recorder<io::BufWriter<std::fs::File>>>, String> {
    let Some(path) = std::env::var_os("SOURCE_CONTROLLER_TRACE").filter(|p| !p.is_empty()) else {
        return Ok(None);
    };
    let remaining = tick_limit(
        std::env::var("SOURCE_CONTROLLER_TRACE_SECONDS")
            .ok()
            .as_deref(),
    )?;
    let output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| format!("controller trace {:?}: {e} (use a new filename)", path))?;
    let mut output = io::BufWriter::new(output);
    writeln!(output, "tick,frame,dt,frame_dt,map,exe,build_git,feet,velocity,grounded,crouched,eye_height,physics_eye,render_eye_at_tick,alpha,input,jump,crouch_key,captured,noclip,yaw,pitch,support_before,support_after,raw_before,raw_after,resolved_before,resolved_after,runtime_displacement,previous_feet,post_runtime_feet,controller_displacement,hull_height,probe_drop18_fraction,probe_drop18_normal,new_support_raw,new_support_resolved")
        .map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())?;
    eprintln!("controller trace {:?}: first {remaining} interactive ticks ({} simulation seconds), buffered; flush every 120 ticks and on close", path, remaining / 120);
    Ok(Some(Recorder { output, remaining }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configurable_duration_is_bounded() {
        assert_eq!(tick_limit(None).unwrap(), 14400);
        assert_eq!(tick_limit(Some("2")).unwrap(), 240);
        for invalid in ["0", "3601", "bad", "-1"] {
            assert!(tick_limit(Some(invalid)).is_err());
        }
    }
    #[test]
    fn write_errors_disable_trace_without_panicking() {
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("disk full"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut active = Some(Recorder {
            output: Broken,
            remaining: 120,
        });
        record(&mut active, || "row".into());
        assert!(active.is_none());
    }
    #[test]
    fn buffering_flushes_at_interval_and_cap() {
        #[derive(Default)]
        struct Sink {
            flushes: usize,
        }
        impl Write for Sink {
            fn write(&mut self, data: &[u8]) -> io::Result<usize> {
                Ok(data.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                self.flushes += 1;
                Ok(())
            }
        }
        let mut r = Recorder {
            output: Sink::default(),
            remaining: 240,
        };
        for _ in 0..119 {
            assert!(!r.row("row").unwrap());
        }
        assert_eq!(r.output.flushes, 0);
        assert!(!r.row("row").unwrap());
        assert_eq!(r.output.flushes, 1);
        for _ in 0..119 {
            assert!(!r.row("row").unwrap());
        }
        assert!(r.row("row").unwrap());
        assert_eq!(r.output.flushes, 2);
    }
    #[test]
    fn disabled_is_lazy_and_cap_is_exact() {
        let mut disabled: Option<Recorder<Vec<u8>>> = None;
        record(&mut disabled, || panic!("disabled trace evaluated"));
        let mut active = Some(Recorder {
            output: Vec::new(),
            remaining: 2,
        });
        for _ in 0..4 {
            record(&mut active, || "1,2,3".into());
        }
        assert!(active.is_none());
        let mut recorder = Recorder {
            output: Vec::new(),
            remaining: 1,
        };
        assert!(recorder.row("1,2,3").unwrap());
        assert_eq!(String::from_utf8(recorder.output).unwrap(), "1,2,3\n");
        assert_eq!(csv("a,\"b\""), "\"a,\"\"b\"\"\"");
    }
}
