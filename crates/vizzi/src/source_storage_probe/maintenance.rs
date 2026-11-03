//! Optional pinned-prefix read sampling during real maintenance.
use crate::source_spool::SourceSpool;
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use vizzi_core::model::{CapturedSource, SourceReadError};

pub(super) const READER_BYTES: usize = 65536;
pub(super) struct Probe<'a> {
    pub spool: Option<&'a SourceSpool>,
    pub reader: Option<Arc<CapturedSource>>,
}
impl Probe<'_> {
    pub fn measure(&self) -> Result<(Option<f64>, Option<Value>), super::Error> {
        let Some(spool) = self.spool else {
            return Ok((None, None));
        };
        let Some(reader) = &self.reader else {
            let started = Instant::now();
            spool.reclaim()?;
            return Ok((Some(started.elapsed().as_secs_f64() * 1000.0), None));
        };
        let stop = AtomicBool::new(false);
        let clock = Instant::now();
        std::thread::scope(|scope| {
            let stop = &stop;
            let (ready, started) = mpsc::channel();
            let worker = scope.spawn(move || -> Result<Vec<(f64, f64)>, SourceReadError> {
                let sample = || -> Result<(f64, f64), SourceReadError> {
                    let start = clock.elapsed().as_secs_f64() * 1000.0;
                    // A fresh session makes every sample perform verified backing IO.
                    if reader
                        .reader()
                        .read(0..READER_BYTES, READER_BYTES)?
                        .iter()
                        .any(|byte| *byte != b'p')
                    {
                        return Err(SourceReadError::Integrity);
                    }
                    Ok((start, clock.elapsed().as_secs_f64() * 1000.0))
                };
                let first = sample();
                let _ = ready.send(());
                let mut samples = vec![first?];
                while !stop.load(Ordering::Acquire) && samples.len() < 4096 {
                    std::thread::sleep(Duration::from_millis(2));
                    if stop.load(Ordering::Acquire) {
                        break;
                    }
                    samples.push(sample()?);
                }
                Ok(samples)
            });
            started.recv()?;
            let begin = clock.elapsed().as_secs_f64() * 1000.0;
            let result = spool.reclaim();
            let end = clock.elapsed().as_secs_f64() * 1000.0;
            stop.store(true, Ordering::Release);
            let samples = worker.join().map_err(|_| "maintenance reader panicked")??;
            result?;
            let completed = samples
                .iter()
                .filter(|(a, b)| *a >= begin && *b <= end)
                .count();
            let before_midpoint = samples
                .iter()
                .filter(|(a, b)| *a >= begin && *b <= (begin + end) / 2.0)
                .count();
            let overlap = samples
                .iter()
                .filter(|(a, b)| *a < end && *b > begin)
                .count();
            Ok((
                Some(end - begin),
                Some(json!({"start_ms":begin,"end_ms":end,
                "completed_during":completed,"completed_before_midpoint":before_midpoint,
                "overlapping":overlap,"capped":samples.len()==4096,
                "samples":samples.iter().map(|(a,b)|json!({"start_ms":a,"end_ms":b,"elapsed_ms":b-a})).collect::<Vec<_>>()})),
            ))
        })
    }
}
