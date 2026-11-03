//! Linux process-tree memory observations with optional verified cgroup enforcement.
use serde::Serialize;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Component, Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
const CAP: u64 = 12 * 1024 * 1024 * 1024;
const INTERVAL: Duration = Duration::from_millis(250);
fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[derive(Clone, Debug, Serialize)]
struct Process {
    pid: u32,
    parent: u32,
    start_ticks: u64,
    name: String,
}
fn process(text: &str) -> Result<Process> {
    let open = text.find('(').ok_or("missing process name")?;
    let close = text
        .rfind(')')
        .filter(|end| *end > open)
        .ok_or("missing process name end")?;
    let fields: Vec<_> = text[close + 1..].split_whitespace().collect();
    Ok(Process {
        pid: text[..open].trim().parse()?,
        parent: fields.get(1).ok_or("missing parent PID")?.parse()?,
        start_ticks: fields.get(19).ok_or("missing process start")?.parse()?,
        name: text[open + 1..close].into(),
    })
}
fn tree(root: u32, processes: &BTreeMap<u32, Process>) -> BTreeSet<u32> {
    let mut included = BTreeSet::from([root]);
    loop {
        let before = included.len();
        for item in processes.values() {
            if included.contains(&item.parent) {
                included.insert(item.pid);
            }
        }
        if included.len() == before {
            return included;
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
struct Usage {
    rss_kib: u64,
    pss_kib: u64,
}
fn usage(text: &str) -> Result<Usage> {
    let mut values = BTreeMap::new();
    for line in text.lines() {
        if let Some((key, value)) = line.split_once(':')
            && ["Rss", "Pss"].contains(&key)
        {
            let fields: Vec<_> = value.split_whitespace().collect();
            if fields.len() != 2 || fields[1] != "kB" || values.contains_key(key) {
                return Err("invalid smaps memory field".into());
            }
            values.insert(key, fields[0].parse()?);
        }
    }
    Ok(Usage {
        rss_kib: *values.get("Rss").ok_or("missing RSS")?,
        pss_kib: *values.get("Pss").ok_or("missing PSS")?,
    })
}
fn total(values: impl Iterator<Item = Usage>) -> Usage {
    values.fold(Usage::default(), |sum, value| Usage {
        rss_kib: sum.rss_kib + value.rss_kib,
        pss_kib: sum.pss_kib + value.pss_kib,
    })
}
fn gone(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
    ) || error.raw_os_error() == Some(3)
}
fn scope(text: &str) -> Result<PathBuf> {
    let path = text
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .ok_or("missing unified cgroup")?;
    let path = Path::new(path);
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err("invalid cgroup path".into());
    }
    Ok(path.to_path_buf())
}
fn counters(text: &str) -> Result<BTreeMap<String, u64>> {
    text.lines()
        .map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() != 2 {
                return Err("invalid memory counter".into());
            }
            Ok((fields[0].to_owned(), fields[1].parse()?))
        })
        .collect()
}
#[derive(Debug, Serialize)]
struct GroupSample {
    current_bytes: u64,
    peak_bytes: u64,
    swap_bytes: u64,
    events: BTreeMap<String, u64>,
}
struct Group {
    directory: PathBuf,
    scope: PathBuf,
}
impl Group {
    fn read(&self) -> Result<GroupSample> {
        let read = |name| fs::read_to_string(self.directory.join(name));
        if read("memory.max")?.trim() != CAP.to_string() || read("memory.swap.max")?.trim() != "0" {
            return Err("capped mode requires memory.max=12884901888 and memory.swap.max=0".into());
        }
        let events = counters(&read("memory.events")?)?;
        if !events.contains_key("oom") || !events.contains_key("oom_kill") {
            return Err("missing OOM counters".into());
        }
        Ok(GroupSample {
            current_bytes: read("memory.current")?.trim().parse()?,
            peak_bytes: read("memory.peak")?.trim().parse()?,
            swap_bytes: read("memory.swap.current")?.trim().parse()?,
            events,
        })
    }
    fn contains(&self, text: &str) -> Result<bool> {
        Ok(scope(text)?.starts_with(&self.scope))
    }
}
fn oom_changed(before: &GroupSample, after: &GroupSample) -> bool {
    ["oom", "oom_kill"]
        .into_iter()
        .any(|key| before.events.get(key) != after.events.get(key))
}
#[derive(Serialize)]
struct Entry {
    process: Process,
    usage: Usage,
}
#[derive(Serialize)]
struct Sample {
    elapsed_ms: f64,
    sampling_ms: f64,
    usage: Usage,
    processes: Vec<Entry>,
    disappeared: Vec<u32>,
    errors: Vec<String>,
    group: Option<GroupSample>,
}
fn evidence_valid(
    samples: &[Sample],
    errors: &[String],
    before: Option<&GroupSample>,
    after: Option<&GroupSample>,
) -> bool {
    !samples.is_empty()
        && errors.is_empty()
        && samples.iter().all(|sample| sample.errors.is_empty())
        && match (before, after) {
            (None, None) => true,
            (Some(before), Some(after)) => !oom_changed(before, after),
            _ => false,
        }
}
fn sample(proc: &Path, root: u32, group: Option<&Group>, started: Instant) -> Result<Sample> {
    let scan_started = Instant::now();
    let mut processes = BTreeMap::new();
    for entry in fs::read_dir(proc)? {
        let entry = entry?;
        if let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>()
            && let Ok(text) = fs::read_to_string(entry.path().join("stat"))
            && let Ok(item) = process(&text)
        {
            processes.insert(pid, item);
        }
    }
    if !processes.contains_key(&root) {
        return Err("observer process missing".into());
    }
    let mut readings = Vec::new();
    let mut disappeared = Vec::new();
    let mut errors = Vec::new();
    for pid in tree(root, &processes) {
        let Some(item) = processes.get(&pid) else {
            continue;
        };
        let directory = proc.join(pid.to_string());
        let reading = (|| -> Result<Usage> {
            let bytes = usage(&fs::read_to_string(directory.join("smaps_rollup"))?)?;
            let current = process(&fs::read_to_string(directory.join("stat"))?)?;
            if current.start_ticks != item.start_ticks {
                return Err("PID reused during sampling".into());
            }
            if let Some(group) = group
                && !group.contains(&fs::read_to_string(directory.join("cgroup"))?)?
            {
                return Err("process escaped verified cgroup".into());
            }
            Ok(bytes)
        })();
        match reading {
            Ok(bytes) => readings.push(Entry {
                process: item.clone(),
                usage: bytes,
            }),
            Err(error) if error.downcast_ref::<io::Error>().is_some_and(gone) => {
                disappeared.push(pid)
            }
            Err(error) => errors.push(format!("PID {pid}: {error}")),
        }
    }
    let group = group.map(Group::read).transpose()?;
    Ok(Sample {
        elapsed_ms: milliseconds(started.elapsed()),
        sampling_ms: milliseconds(scan_started.elapsed()),
        usage: total(readings.iter().map(|reading| reading.usage)),
        processes: readings,
        disappeared,
        errors,
        group,
    })
}
/// Runs a command while sampling this observer and its descendants every 250 ms.
///
/// Capped mode requires a pre-existing cgroup with a 12 GiB memory maximum and zero
/// swap allowance. The observer only reads cgroup controls. Observe-only mode never
/// claims enforcement. Returns false for child failure or incomplete/invalid evidence.
/// Output is created exclusively and records transient exits and sampling failures.
pub fn run(output: &Path, observe_only: bool, command: &[String]) -> Result<bool> {
    let (program, arguments) = command.split_first().ok_or("command required")?;
    let group = if observe_only {
        None
    } else {
        let current = scope(&fs::read_to_string("/proc/self/cgroup")?)?;
        Some(Group {
            directory: Path::new("/sys/fs/cgroup").join(current.strip_prefix("/")?),
            scope: current,
        })
    };
    let before = group.as_ref().map(Group::read).transpose()?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    let mut child = Command::new(program).args(arguments).spawn()?;
    let started = Instant::now();
    let mut samples = Vec::new();
    let mut errors = Vec::new();
    let status = loop {
        match sample(
            Path::new("/proc"),
            std::process::id(),
            group.as_ref(),
            started,
        ) {
            Ok(reading) => samples.push(reading),
            Err(error) => errors.push(error.to_string()),
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        std::thread::sleep(INTERVAL);
    };
    let after = group.as_ref().map(Group::read).transpose();
    let after = match after {
        Ok(value) => value,
        Err(error) => {
            errors.push(error.to_string());
            None
        }
    };
    let success =
        status.success() && evidence_valid(&samples, &errors, before.as_ref(), after.as_ref());
    let report = json!({
        "schema":1, "mode":if observe_only {"observe_only"} else {"cgroup"},
        "cap_verified":!observe_only && success,
        "requested_cap_bytes":CAP, "command":command, "child_exit_code":status.code(),
        "success":success, "interval_ms":INTERVAL.as_millis(), "wall_ms":milliseconds(started.elapsed()),
        "peak_pss_kib":samples.iter().map(|s|s.usage.pss_kib).max().unwrap_or(0),
        "peak_rss_kib":samples.iter().map(|s|s.usage.rss_kib).max().unwrap_or(0),
        "group_path":group.as_ref().map(|g|&g.scope), "group_before":before, "group_after":after,
        "samples":samples, "errors":errors,
        "limits":["PSS/RSS are simultaneous sampled sums, not sums of process high-water marks.",
        "Sampling is non-atomic; transient/missed processes and short peaks can be absent.",
        "RSS counts shared mappings repeatedly; PSS apportions mapped shared pages.",
        "Orphaned processes can leave the observed ancestry; a verified cgroup accounts for inherited membership.",
        "Process PSS excludes unmapped filesystem cache/kernel charges; cgroup values include controller-accounted memory.",
        "The observer and driver contribute to samples; no editor workload or physical laptop is emulated."]
    });
    serde_json::to_writer_pretty(&mut output, &report)?;
    output.write_all(b"\n")?;
    Ok(success)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incomplete_samples_or_oom_events_cannot_qualify_a_run() {
        assert_eq!(milliseconds(Duration::from_micros(1500)), 1.5);
        assert_eq!(milliseconds(Duration::ZERO), 0.0);
        let temporary = tempfile::tempdir().unwrap();
        let group = group_fixture(temporary.path());
        let before = group.read().unwrap();
        let after = group.read().unwrap();
        let mut samples = vec![Sample {
            elapsed_ms: 0.0,
            sampling_ms: 0.0,
            usage: Usage::default(),
            processes: vec![],
            disappeared: vec![],
            errors: vec![],
            group: None,
        }];
        assert!(evidence_valid(&samples, &[], None, None));
        assert!(evidence_valid(&samples, &[], Some(&before), Some(&after)));
        assert!(!evidence_valid(&[], &[], None, None));
        assert!(!evidence_valid(
            &samples,
            &["unreadable procfs".into()],
            None,
            None
        ));
        assert!(!evidence_valid(&samples, &[], Some(&before), None));
        assert!(!evidence_valid(&samples, &[], None, Some(&after)));
        samples[0].errors.push("unreadable member".into());
        assert!(!evidence_valid(&samples, &[], None, None));
        samples[0].errors.clear();
        fs::write(temporary.path().join("memory.events"), "oom 0\noom_kill 1").unwrap();
        assert!(!evidence_valid(
            &samples,
            &[],
            Some(&before),
            Some(&group.read().unwrap())
        ));
    }
    #[test]
    fn exited_processes_are_distinct_from_permission_or_io_failures() {
        for code in [2, 3, 20] {
            assert!(gone(&io::Error::from_raw_os_error(code)));
        }
        for code in [5, 13] {
            assert!(!gone(&io::Error::from_raw_os_error(code)));
        }
    }
    fn stat(pid: u32, parent: u32, start: u64) -> String {
        format!(
            "{pid} (name with ) and () S {parent} {} {start} 0",
            vec!["0"; 17].join(" ")
        )
    }
    #[test]
    fn stat_names_do_not_shift_parent_and_start_fields() {
        let item = process(&stat(17, 9, 1234)).unwrap();
        assert_eq!((item.pid, item.parent, item.start_ticks), (17, 9, 1234));
        assert_eq!(item.name, "name with ) and (");
        for text in ["", "1 (bad", "1 ()", "x (a) S", "1 )bad(", "1 (x) S bad"] {
            assert!(process(text).is_err());
        }
    }
    #[test]
    fn process_tree_is_transitive_and_excludes_siblings_and_unrelated_cycles() {
        let values = [(1, 0), (2, 1), (3, 2), (4, 1), (9, 10), (10, 9)]
            .map(|(pid, parent)| (pid, process(&stat(pid, parent, 1)).unwrap()))
            .into_iter()
            .collect();
        assert_eq!(tree(2, &values), BTreeSet::from([2, 3]));
        assert_eq!(tree(1, &values), BTreeSet::from([1, 2, 3, 4]));
    }
    #[test]
    fn pss_and_rss_are_distinct_with_strict_units_and_required_fields() {
        assert_eq!(
            usage("header\nRss: 12 kB\nPss: 7 kB\nShared_Clean: 5 kB").unwrap(),
            Usage {
                rss_kib: 12,
                pss_kib: 7
            }
        );
        for text in [
            "Rss: 12 kB",
            "Pss: 7 kB",
            "Rss: 12 MB\nPss: 7 kB",
            "Rss: -1 kB\nPss: 0 kB",
            "Rss: 12 kB extra\nPss: 7 kB",
            "Rss: 1 kB\nRss: 2 kB\nPss: 0 kB",
        ] {
            assert!(usage(text).is_err(), "{text}");
        }
        assert_eq!(
            total(
                [
                    Usage {
                        rss_kib: 12,
                        pss_kib: 7
                    },
                    Usage {
                        rss_kib: 20,
                        pss_kib: 10
                    }
                ]
                .into_iter()
            ),
            Usage {
                rss_kib: 32,
                pss_kib: 17
            }
        );
        assert_eq!(total(std::iter::empty()), Usage::default());
    }
    fn group_fixture(directory: &Path) -> Group {
        for (name, value) in [
            ("memory.max", CAP.to_string()),
            ("memory.swap.max", "0".into()),
            ("memory.current", "123".into()),
            ("memory.peak", "456".into()),
            ("memory.swap.current", "0".into()),
            (
                "memory.events",
                "low 0\nhigh 2\nmax 1\noom 0\noom_kill 0".into(),
            ),
        ] {
            fs::write(directory.join(name), value).unwrap();
        }
        Group {
            directory: directory.into(),
            scope: PathBuf::from("/fixture.scope"),
        }
    }
    #[test]
    fn capped_evidence_requires_exact_limits_and_oom_counters() {
        let temporary = tempfile::tempdir().unwrap();
        let group = group_fixture(temporary.path());
        let before = group.read().unwrap();
        assert_eq!(
            (before.current_bytes, before.peak_bytes, before.swap_bytes),
            (123, 456, 0)
        );
        assert!(!oom_changed(&before, &group.read().unwrap()));
        fs::write(temporary.path().join("memory.events"), "oom 1\noom_kill 0").unwrap();
        assert!(oom_changed(&before, &group.read().unwrap()));
        fs::write(temporary.path().join("memory.events"), "oom 0\noom_kill 1").unwrap();
        assert!(oom_changed(&before, &group.read().unwrap()));
        for value in ["high 1", "oom 0", "oom_kill 0"] {
            fs::write(temporary.path().join("memory.events"), value).unwrap();
            assert!(group.read().is_err());
        }
        group_fixture(temporary.path());
        for value in ["max", "0", "12884901887", "12884901889", "garbage"] {
            fs::write(temporary.path().join("memory.max"), value).unwrap();
            assert!(group.read().is_err());
        }
        group_fixture(temporary.path());
        for value in ["max", "1"] {
            fs::write(temporary.path().join("memory.swap.max"), value).unwrap();
            assert!(group.read().is_err());
        }
        assert!(counters("bad").is_err());
        assert!(counters("oom invalid").is_err());
    }
    #[test]
    fn unified_scope_validation_preserves_component_boundaries() {
        let temporary = tempfile::tempdir().unwrap();
        let group = group_fixture(temporary.path());
        assert!(group.contains("0::/fixture.scope").unwrap());
        assert!(group.contains("0::/fixture.scope/child").unwrap());
        assert!(!group.contains("0::/fixture.scope-other").unwrap());
        for value in ["1:memory:/x", "0::relative", "0::/../escape", ""] {
            assert!(scope(value).is_err());
        }
    }
    #[test]
    fn sampling_sums_only_descendants_and_reports_unreadable_or_escaped_members() {
        let temporary = tempfile::tempdir().unwrap();
        let proc = temporary.path().join("proc");
        fs::create_dir(&proc).unwrap();
        for (pid, parent, rss, pss) in [
            (100, 0, 100, 80),
            (50, 100, 20, 10),
            (30, 50, 15, 5),
            (200, 0, 1000, 1000),
        ] {
            let directory = proc.join(pid.to_string());
            fs::create_dir(&directory).unwrap();
            fs::write(directory.join("stat"), stat(pid, parent, 10)).unwrap();
            fs::write(
                directory.join("smaps_rollup"),
                format!("Rss: {rss} kB\nPss: {pss} kB"),
            )
            .unwrap();
            fs::write(directory.join("cgroup"), "0::/fixture.scope/child").unwrap();
        }
        let group = group_fixture(temporary.path());
        let first = sample(&proc, 100, Some(&group), Instant::now()).unwrap();
        assert_eq!(
            first.usage,
            Usage {
                rss_kib: 135,
                pss_kib: 95
            }
        );
        assert_eq!(
            first
                .processes
                .iter()
                .map(|entry| entry.process.pid)
                .collect::<Vec<_>>(),
            [30, 50, 100]
        );
        assert!(first.errors.is_empty() && first.disappeared.is_empty());
        assert_eq!(first.group.unwrap().peak_bytes, 456);
        fs::remove_file(proc.join("30/smaps_rollup")).unwrap();
        let exited = sample(&proc, 100, None, Instant::now()).unwrap();
        assert_eq!(
            exited.usage,
            Usage {
                rss_kib: 120,
                pss_kib: 90
            }
        );
        assert_eq!(exited.disappeared, [30]);
        assert!(exited.errors.is_empty());
        fs::write(proc.join("50/smaps_rollup"), "Rss: 1 kB").unwrap();
        assert_eq!(
            sample(&proc, 100, None, Instant::now())
                .unwrap()
                .errors
                .len(),
            1
        );
        fs::write(proc.join("50/smaps_rollup"), "Rss: 20 kB\nPss: 10 kB").unwrap();
        fs::write(proc.join("50/cgroup"), "0::/fixture.scope-escape").unwrap();
        assert_eq!(
            sample(&proc, 100, Some(&group), Instant::now())
                .unwrap()
                .errors
                .len(),
            1
        );
        assert!(sample(&proc, 999, None, Instant::now()).is_err());
    }
}
