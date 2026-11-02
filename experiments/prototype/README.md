# Measure the first application slice

This procedure measures the native application through its public diagnostic transport.
It complements the isolated [performance experiments](../performance/README.md), and
does not measure browser presentation. The script edits only a temporary source copy.
See [the first application report](RESULTS.md) for recorded results and limitations.
The driver uses `//` or `#` edit probes according to the selected outline language.
Default capture is Rust/Python; explicit source-policy options also select C/C++ inputs. See the
[mixed-language procedure and evidence](../languages/README.md) for PyTorch runs.

For C-family runs, repeat `--c-family-root`, `--syntax-language`, and `--exclude-source`
as needed; these are forwarded to cold, warm, live, and clean-capture processes alike.
`--select-file RELATIVE_PATH` chooses an assigned file to edit. The report records the
effective source arguments, capture policy, selection provenance, and bounded captured
inventory. It also records whether that inventory covers the complete scope. A selected
source-only file is rejected by this edit/analysis driver. See the
[C-family procedure](../languages/README.md#check-raw-cc-adapters) for exact scoped runs.

Build the optimized host and run with a pinned, build-artifact-free source archive:

```sh
cargo build --release -p vizzi
python3 experiments/prototype/measure.py \
  --source /tmp/vizzi-perf-corpus/lpc-rs \
  --revision 8d3b1f54b36eae33f689bae3e42c716647000e4d \
  --workers 4 \
  --output /tmp/vizzi-application-results.json
```

The Linux driver requires Python 3, `/usr/bin/time`, and `/proc` for live peak RSS.
It refuses to overwrite an existing result. It records three loads with independent
empty caches, three fresh-process persisted-cache loads, a settled in-memory no-op,
and three one-file edits through the live coordinator. It checks snapshot identity,
cache reuse, selected declaration continuity, new captured source, and pinned old source.
Cache writes are atomic replacements with buffered I/O; this does not measure durability
or crash recovery through power loss.

Run without competing benchmarks or mutation jobs. OS file caches are warm; an empty
analysis cache is described as cold, without implying cold storage. Three samples give
an initial range, not a tail distribution. `write_to_response_ms` ends after receiving
a coherent diagnostic view. It excludes browser status polling and rendering.

## Verify correctness

```sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
node crates/vizzi/web/controller.test.js
```

The JavaScript tests run in process using Node's built-in test module. They cover request
ordering, publication reloads, retries, and preservation of reconciled selection. Native
tests include an actual stdio host receiving an atomic edit. HTTP handlers run in process.
These checks do not execute browser layout, Canvas painting, or DOM interaction.

Run mutation testing for the core model and native source/cache/publication boundary:

```sh
cargo mutants -p vizzi-core -p vizzi --no-config \
  --file 'crates/vizzi-core/src/{model,publication,query,rust,python,syntax,analysis}.rs' \
  --file 'crates/vizzi/src/{cache,engine,application,source}.rs' \
  --test-package vizzi-core --test-package vizzi \
  --jobs 4 --timeout 15 --build-timeout 180
```

This scope covers complete modules rather than just changed lines. Scheduler, HTTP/stdio
transport, and browser-adapter mutations are outside this run. Keep timeouts and unviable
mutations separate from those caught by assertions. No prototype-specific exclusions
are configured.

The [browser follow-up](../browser/README.md) measures labelled overview interaction,
picking, virtualised source, render-ready update delay, and frame scheduling during
background analysis. The 16 GB laptop baseline and physical presentation remain separate
qualification work. `window.vizziMetrics` supplies bounded CPU draw/request samples;
those are instrumentation, not a substitute for presentation/GPU measurements.
