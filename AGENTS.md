# Contributor guidance

Build coordinated codebase views for learning unfamiliar code and reviewing changes.

- Keep this file focused on durable contributor instructions. Store evolving design,
  scope, implementation plans, and progress in their own documents.
- Read [DESIGN.md](DESIGN.md) for current project design and scope. Develop from
  explicit architectural requirements and small, validated implementations.
- Use Rust with the 2024 edition, declared in workspace package metadata and inherited
  by implementation crates. Choose the supported compiler version separately.

## Dependencies

- Prefer the latest stable dependency releases, including newer major versions when
  feasible. Verify current published versions when adding or updating dependencies.
- Document the reason and compatibility constraint when an older version is necessary.
  Use prerelease versions only when a concrete requirement calls for them.

## Documentation

- Keep the README focused on the product and a first run. Index user guidance in
  [docs/README.md](docs/README.md); keep contributor contracts, history and experiment
  records behind [the contributor index](docs/development/README.md), not in the README.
- Update [CHANGELOG.md](CHANGELOG.md) in the same change as notable user-visible behaviour.
  Add concise entries under `Unreleased`, using applicable change categories. Keep plans
  and detailed validation in their existing documents; omit routine internal-only changes.
- Apply [Diátaxis](https://diataxis.fr/) principles: tutorials for learning by doing,
  how-to guides for completing tasks, reference for factual lookup, and explanations
  for understanding concepts and trade-offs.
- Give each document a clear audience and purpose. Separate and cross-link distinct
  documentation types. Let structure follow useful content rather than creating empty
  categories in advance.

## Rust safety

- Use safe Rust. Introduce unsafe code only when the requirement cannot be met
  through a viable safe interface, such as unavoidable FFI. Prefer existing safe
  wrappers. Performance gains or convenience alone do not justify unsafe code.
- Deny `unsafe_code` by default through inherited
  workspace lints. Keep any necessary exception narrowly scoped to the smallest
  module or adapter and expose a safe interface; do not relax the lint globally.
- Explain why each exception is necessary and why safe alternatives are insufficient.
  Document the safety invariants at each unsafe operation with a `SAFETY` comment,
  and add targeted tests for the safe interface and its error cases.

## Development and verification

- Strongly prefer TDD for new behavior: write a focused test, observe the expected
  failure, implement the smallest coherent change, then refactor with tests passing.
  For bug fixes, reproduce the bug in a failing regression test first.
- Test behavior through module interfaces. Avoid tests that merely repeat the
  implementation or assert private call sequences.
- Use small deterministic fixtures for correctness and real repositories for broader
  integration and performance checks. Keep the normal development feedback loop fast.
- Use cargo-mutants to check the strength of behavioral tests. Establish a passing
  unmodified baseline before interpreting mutation results.
- Keep mutation runs considerate of other work on the machine: use one mutant job
  and one compiler/jobserver task (`-j 1 --jobserver-tasks 1`, `CARGO_BUILD_JOBS=1`),
  with serial test execution (`RUST_TEST_THREADS=1`).
  On Unix, run at lower priority with `nice -n 10`. Do not raise concurrency unless
  the user explicitly requests it.
- Run focused mutation testing during development and broader relevant runs at
  integration milestones. Diff-scoped runs alone do not establish full coverage.
- Investigate surviving mutations. Add meaningful tests for behavior-changing survivors;
  document narrow exclusions for equivalent mutations. Report timeouts, build failures,
  exclusions, and incomplete runs separately from mutations caught by failing tests.
- Treat coverage and mutation results as evidence rather than proof of correctness.
  Report which checks ran and their material limitations.
- Keep performance verification separate from correctness tests. Benchmark optimized
  builds using representative workloads.

Baseline checks are formatting, workspace tests (including doctests), and Clippy.
Validate affected native and browser implementations on their respective targets.

Mutation testing references: [cargo-mutants](https://mutants.rs/) and
[testing changes in a diff](https://mutants.rs/pr-diff.html).
