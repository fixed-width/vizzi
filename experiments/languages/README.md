# Verify and measure mixed-language analysis

Use this procedure to check the [adapter and shared worker contracts](../../docs/prototype/LANGUAGES.md).
The [results](RESULTS.md) explain the initial native measurements and their limits.
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
