//! Opt-in, bounded CSV diagnostics. No simulation behavior changes.
use std::io::{self, Write};
pub(crate) struct Recorder<W: Write> {
    output: W,
    remaining: usize,
}
impl<W: Write> Recorder<W> {
    fn row(&mut self, row: &str) -> io::Result<bool> {
        writeln!(self.output, "{row}")?;
        self.output.flush()?;
        self.remaining -= 1;
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
                eprintln!("controller trace complete (1800 tick cap)");
                *recorder = None;
            }
            Err(e) => {
                eprintln!("controller trace disabled after write error: {e}");
                *recorder = None;
            }
        }
    }
}
pub(crate) fn open() -> Result<Option<Recorder<std::fs::File>>, String> {
    let Some(path) = std::env::var_os("SOURCE_CONTROLLER_TRACE").filter(|p| !p.is_empty()) else {
        return Ok(None);
    };
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| format!("controller trace {:?}: {e} (use a new filename)", path))?;
    writeln!(output, "tick,frame,dt,frame_dt,map,exe,build_git,feet,velocity,grounded,crouched,eye_height,physics_eye,render_eye_at_tick,alpha,input,jump,crouch_key,captured,noclip,yaw,pitch,support_before,support_after,raw_before,raw_after,resolved_before,resolved_after,runtime_displacement")
        .map_err(|e| e.to_string())?;
    eprintln!("controller trace {:?}: first 1800 interactive ticks (15 simulation seconds), flushed each tick", path);
    Ok(Some(Recorder {
        output,
        remaining: 1800,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
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
