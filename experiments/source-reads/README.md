# Compare resident and immutable-backed source

Use the existing pinned qualification archives. The source-only storage workflow needs
no fact-cache preparation; the admission workflow prepares current-context facts before
timing because reader/model changes conservatively update analyser fingerprints.

```sh
cargo build --release --offline -p vizzi --all-features \
  --example source_storage --example admit --bin vizzi
python3 experiments/source-reads/measure.py \
  --cache /tmp/vizzi-source-read-cache \
  --output results/local/source-reads.json
```

The driver compares resident and 16 GiB session-spool storage for four whole-repository
prefix generations, then exercises structural/paste/unseen/batch admission with spooling.
Storage children use a 12 GiB virtual address-space limit; admission children use 8 GiB.
These are not RSS/physical-memory limits or baseline-laptop qualification. All copies
are owned/marked and restored; original archives and analysed project tooling are untouched.

Use the source-storage and admission procedures for phase definitions and independent
checks. Resident/backed storage uses current metadata/integrity accounting in both modes.
The spool capacity is cumulative per session, not live disk usage or a memory limit.
Single sequential conditions and unique UUIDs are observations, not randomized speedups
or tail estimates. Run without competing builds, tests, mutation jobs or benchmarks.
Full memory peaks include integrity checks, verification and probe construction.
