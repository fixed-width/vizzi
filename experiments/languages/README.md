# Verify and measure mixed-language analysis

Use this procedure to check the [adapter and shared worker contracts](../../docs/prototype/LANGUAGES.md).
The [results](RESULTS.md) explain the initial native measurements and their limits.
The [source-policy follow-up](SOURCE_POLICY_RESULTS.md) records selection, snapshot
identity, schema-2 browser behavior, final mutation evidence, and native regression timings.
The [C/C++ report](C_FAMILY_RESULTS.md) records raw adapters and scoped reference checks.
Use pinned source archives without build artifacts; no analyzed project is built.

Run the regular checks and the actual browser:

```sh
cargo fmt --all --check
cargo test --workspace --all-features --offline
cargo clippy --workspace --all-targets --all-features --offline -- -D warnings
npm test
```

The language integration tests register instrumented Rust/Python adapters. A barrier
requires 1, 2, and 4 workers to analyze concurrently, and counters verify that all
languages share that limit. Warm loads must create no parsers. Other fixtures cover
context changes, reduced source scope, extension conflicts, invalid adapter output,
cache metadata mismatch, and incremental rename/removal equivalence to a full capture.
The browser test selects Python declarations and preserves selection across an edit.

Run complete changed-module mutation testing, including the integration tests:

```sh
cargo mutants -p vizzi-core -p vizzi \
  --config experiments/incremental/mutants.toml \
  --file 'crates/vizzi-core/src/{analysis,python,rust,syntax}.rs' \
  --file 'crates/vizzi/src/{engine,source,invalidation}.rs' \
  --test-package vizzi-core --test-package vizzi \
  --jobs 4 --timeout 15 --build-timeout 180
```

For the complete source-policy scope, replace the file filters with
`crates/vizzi-core/src/{policy,model,query,analysis}.rs` and
`crates/vizzi/src/{source,engine,invalidation,main,server}.rs`. Run
`npm test -- --mutations` for all controller/update/source-description modules.

Build the release host, then use the [native measurement driver](../prototype/README.md).
Run measurements without competing tests, mutations, or other benchmark processes:

```sh
cargo build --release -p vizzi --offline
python3 experiments/prototype/measure.py \
  --source /tmp/vizzi-perf-corpus/pytorch-1f4d2d5e \
  --revision 1f4d2d5e820dd870626dca4f9603e3a49879dbde \
  --workers 4 --output /tmp/vizzi-languages-pytorch.json
python3 experiments/prototype/measure.py \
  --source /tmp/vizzi-perf-corpus/lpc-rs \
  --revision 8d3b1f54b36eae33f689bae3e42c716647000e4d \
  --workers 4 --output /tmp/vizzi-languages-rust.json
```

The driver edits only temporary copies, chooses a language-appropriate comment prefix,
and compares every observed edit with a separate fresh complete capture. It refuses
to overwrite reports. Preserve the binary hash and source revision with each result;
the checked-in report additionally records selected-source inventory hashes.

## Inspect C-family candidates

Run the read-only suffix inventory on the same pinned archives to reproduce the
[policy evidence](../../docs/explanation/c-family-analysis.md). This counts candidates;
it does not identify their language, parse them, or measure performance:

```sh
python3 experiments/languages/inventory-c-family.py \
  --corpus linux /tmp/vizzi-perf-corpus/linux-cbbddb6b cbbddb6b08cc9900a87cccaa9b22bbe06aa4f50a \
  --corpus pytorch /tmp/vizzi-perf-corpus/pytorch-1f4d2d5e 1f4d2d5e820dd870626dca4f9603e3a49879dbde \
  --corpus lpc-rs /tmp/vizzi-perf-corpus/lpc-rs 8d3b1f54b36eae33f689bae3e42c716647000e4d \
  --output /tmp/vizzi-c-family-inventory.json
```

Revision strings are caller-supplied archive provenance. The optional
`--implementation-revision` records the Vizzi revision. The report fingerprints the
inventory script and each corpus's sorted path/size/content manifest, skips symlinks
and root `.git`/`target`/`node_modules`, and refuses to overwrite an existing report.
It reads candidate contents but runs no builds, compiler commands, or project scripts.

## Check raw C/C++ adapters

Use the regular correctness checks above. `c_family` core/native fixtures cover supported
declarations, spelling/spans/scopes, conditional and overload ambiguity, selection,
cache isolation, explicit headers, CUDA/LPC exclusions, and cross-language renames.
The worker-budget fixture now mixes all four languages at 1, 2, and 4 workers; it also
requires warm loads to create no parsers.

For complete affected-module mutation testing:

```sh
cargo mutants -p vizzi-core -p vizzi \
  --config experiments/incremental/mutants.toml \
  --file 'crates/vizzi-core/src/{c_family,syntax,analysis,rust,python,policy}.rs' \
  --file 'crates/vizzi/src/{engine,source}.rs' \
  --test-package vizzi-core --test-package vizzi \
  --jobs 4 --timeout 15 --build-timeout 180
```

For scoped native checks, build the optimized host and run without competing tests
or mutation jobs. These sample subtrees; they do not qualify full-repository scale:

```sh
cargo build --release -p vizzi --offline
python3 experiments/prototype/measure.py \
  --source /tmp/vizzi-perf-corpus/linux-cbbddb6b/kernel/locking \
  --revision cbbddb6b08cc9900a87cccaa9b22bbe06aa4f50a \
  --c-family-root . --syntax-language .=c --select-file mutex.c \
  --output /tmp/vizzi-c-family-linux-locking.json
python3 experiments/prototype/measure.py \
  --source /tmp/vizzi-perf-corpus/pytorch-1f4d2d5e/c10 \
  --revision 1f4d2d5e820dd870626dca4f9603e3a49879dbde \
  --c-family-root . --syntax-language .=cpp --select-file core/Device.cpp \
  --output /tmp/vizzi-c-family-pytorch-c10.json
```

The explicit root assignments select raw header grammars for these samples. They do
not import or establish compiler build configurations. Repeat the default lpc-rs run
without C-family options and confirm the captured inventory contains only `.rs`/`.py`.
The driver edits temporary copies and compares each incremental result with a complete
capture using the same policy arguments. Inspect reported grammar recovery separately
from source-only files and unsupported encoding.
