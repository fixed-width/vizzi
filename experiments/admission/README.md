# Measure source availability under analysis input admission

This how-to compares structural edits, a large paste, an unseen file and a three-file
batch during periodic recovery. See the [results](RESULTS.md) and
[input admission contract](../../docs/prototype/ADMISSION.md).

Use pinned archives from the [qualification profiles](../qualification/profiles.json).
The driver prepares compatible analysis caches before timing; the new deferred model
conservatively changes analyser-context fingerprints. Original archives remain unchanged
and no analysed project is built or executed.

```sh
cargo build --release --offline -p vizzi --all-features --example admit --bin vizzi
python3 experiments/admission/measure.py \
  --cache /tmp/vizzi-admission-cache \
  --output results/local/admission.json
```

Use `--profile linux` or `--profile pytorch` for one repository. Four shared parser workers
and 512 MiB opt-in syntax retention are used in both modes. Each marked disposable copy
gets separate processes for admission disabled and enabled: 32 MiB active input for Linux
and 12 MiB for PyTorch. Their per-file limits are 24/9 MiB and foreground headroom 8/3 MiB.
The limits admit the largest original file, while the batch exceeds total input capacity
and the paste exceeds per-file capacity. They are diagnostic choices, not defaults.

The structural step adds 1,000 declarations to an ordinary file. The paste appends the
primary header plus 2 MiB whitespace to that file. The unseen file receives a unique
header copy at the source root; the batch changes the three large selected files.
Every write uses a fresh UUID and follows an observation of active recovery. A recovery
can finish between the sample and write; this is observed activity, not instrumented
proof of all IO/parse ordering. Held-parser correctness tests establish true overlap.

Each step waits for matching published source, verifies source/classification/scope
against independent directory capture, waits for admissible inputs to complete without
another edit, and compares completed changed-file facts with independent fresh parsing.
Oversize inputs remain explicitly deferred. Original targets and the new file are then
restored/removed, and the final complete observation must match the initial baseline and
a full capture. Old pinned source is checked against an independent byte copy.

`write_to_published_source_ms` ends when fresh source is published. Deferred analysis
can still be outstanding. Settlement is checked after source verification; it is not a
separate causal analysis-latency measurement. Recorded latest engine measurements may
belong to later recovery; inspect document coverage and separate recovery metadata.

Run without competing builds, tests, mutation jobs or benchmarks. A Python wrapper sets
child-only `RLIMIT_AS` before exec; the default is 8,192 MiB, adjustable with
`--address-space-mib`. This limits virtual address space, not RSS or physical RAM. It
does not qualify the 16 GB laptop. Process peaks include probe input construction,
independent captures and fresh validation parses; the shared sampler's phase labels do
not isolate verification. No HTTP/browser/rendering or native-watcher timing is included.
Existing output/configuration/logs are not overwritten; completed-condition checkpoints
are retained even if a later condition fails.

The browser presentation checks and focused description mutations run separately:

```sh
npm test
npm test -- --admission-mutations
```
