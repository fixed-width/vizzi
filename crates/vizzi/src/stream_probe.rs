//! Optional large-file capture probe with bounded independent verification.
use crate::source_spool::SourceSpool;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{fs::File, path::PathBuf, sync::Arc, time::Instant};
use vizzi_core::model::{CapturedSource, SourceEncoding};
type Error = Box<dyn std::error::Error>;
#[derive(Deserialize)]
struct Config {
    root: PathBuf,
    owner: String,
    mode: Mode,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Whole,
    Stream,
}
fn memory() -> Value {
    let text = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let number = |name: &str| {
        text.lines()
            .find(|line| line.starts_with(name))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|s| s.parse::<usize>().ok())
    };
    json!({"rss_kib":number("VmRSS:"),"hwm_kib":number("VmHWM:")})
}
/// Captures a marked large-file fixture twice, verifying old/new identity without full validation loads.
pub fn run(configuration: &[u8], emit: &mut impl FnMut(Value)) -> Result<Value, Error> {
    let config: Config = serde_json::from_slice(configuration)?;
    if config.owner.is_empty()
        || std::fs::read_to_string(config.root.join(".vizzi-stream-owned"))? != config.owner
    {
        return Err("stream probe requires an owned fixture".into());
    }
    let path = config.root.join("large.rs");
    let len = usize::try_from(std::fs::metadata(&path)?.len())?;
    let spool = SourceSpool::new(
        config.root.parent().ok_or("fixture parent missing")?,
        len.max(1),
    )?;
    let mut previous: Option<Arc<CapturedSource>> = None;
    let mut phases = Vec::new();
    for sample in 0..2 {
        emit(json!({"event":"phase_start","sample":sample}));
        let started = Instant::now();
        let source = match config.mode {
            Mode::Whole => spool.persist(CapturedSource::capture(
                std::fs::read(&path)?,
                previous.as_ref(),
            ))?,
            Mode::Stream => spool
                .capture(&mut File::open(&path)?, previous.as_ref(), &|| false)?
                .ok_or("capture cancelled")?,
        };
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
        phases.push(json!({"sample":sample,"capture_ms":elapsed,"memory_after_capture":memory(),"spool":spool.stats()}));
        emit(json!({"event":"phase_done","sample":sample}));
        if source.len() != len
            || source.encoding() != SourceEncoding::Utf8
            || source.line_count() != 1
        {
            return Err("large fixture metadata differs".into());
        }
        if let Some(previous) = &previous
            && !Arc::ptr_eq(previous, &source)
        {
            return Err("unchanged fixture lost source sharing".into());
        }
        let mut session = source.reader();
        let mut hash = blake3::Hasher::new();
        hash.update(b"ContentId");
        hash.update(&(len as u64).to_le_bytes());
        for start in (0..len).step_by(65536) {
            let bytes = session.read(start..(start + 65536).min(len), 65536)?;
            if bytes.iter().any(|byte| *byte != b' ') {
                return Err("fixture bytes changed".into());
            }
            hash.update(&bytes);
        }
        if hash.finalize().to_hex().as_str() != source.content().as_str() {
            return Err("bounded fixture identity differs".into());
        }
        previous = Some(source);
    }
    Ok(
        json!({"bytes":len,"phases":phases,"bounded_bytes_match":true,"unchanged_shared":true,"final_spool":spool.stats()}),
    )
}
