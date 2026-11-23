# Cross-file historical calls architecture

Implementation contract for the [cross-file call behaviour](CROSS_FILE_CALLS_PRODUCT.md).
The existing [captured-call adapter](CAPTURED_CALLS_TECH.md) resolves a caller and
its candidate in one owned file. The registered syntax adapters already supply written
call names and declaration facts in a common representation. Keep Git access, source
ownership and candidate selection shared across the adapters.

## Changes

- Extend the captured-call interface with lookup in one selected file. The native owner
  supplies the immutable commit after validating the caller and its call witness. List
  paths one directory at a time from that Git tree. Read at most 256 KiB of directory
  records and return 24 entries per page. Accept a literal repository-relative file path
  even when a directory listing reaches a bound. The target must be a regular file
  admitted by the source policy.
- Analyze that file through the registered adapter and project candidates using the
  captured invocation's native name and the existing structural declaration rules. Apply
  the file scope before candidate paging. Availability follows captured facts and analysis
  coverage, including partial analysis and source-only files. A new written-call adapter
  inherits this path through the existing registration. Stronger resolution can use a
  shared captured-source interface once multiple language rules have concrete requirements.
- Revalidate the caller owner, source, declaration and call on open, and verify that the
  selected destination belongs to the candidates in that exact file and commit. Analyze
  at most one candidate file per lookup with the existing 2 MiB per-file limit. Retain at
  most eight additional files and 4 MiB per history session or comparison pair, within
  their aggregate byte allowances. Each history ordinal or committed comparison side
  binds its files to its immutable commit. Each owner has one replaceable prepared file
  within these bounds. The browser validates the prepared source and awaits native
  confirmation before adding or saving the bubble and link. If a competing preparation
  replaces the pending file, the picker reports the conflict and keeps the workspace
  intact. Quota refusals return a conflict immediately. Retries cover temporary
  contention for the owner. Existing confirmed files survive refusal and cancellation.
- Extend source window and call queries to validate exact admitted file observations
  under their owner. The browser uses that owner plus exact endpoint source pins for
  saved links. Reuse an existing destination only when both its observation and owner
  match. Preserve old same-file saved links and retain the existing browser navigation
  tickets and source-adoption checks for late replies. Additional files carry their
  own declaration and call facts. Project-wide module cues are outside this file-scoped capture.

## Validation

- Make the same two-commit fixture pass for every built-in written-call adapter. Keep
  the caller unchanged and change its callee in a second file. Confirm that each commit
  opens its own callee through the shared file selection and candidate flow. Cover Rust,
  Python, Ruby, C, C++, JavaScript, TypeScript and TSX before release (1–4).
- Exercise inline action and Calls in a mixed-language Chromium workspace. Include a
  committed comparison side, a saved working-tree side, divergent live source, a cycle,
  reload and layout assertions. Verify candidate labels and explicit selection for sole
  and multiple matches. Verify keyboard focus on Escape and Close. Hold confirmation
  while a competing request replaces the prepared file, then reload to check that the
  rejected callee and link were never saved (1–5).
- Check duplicate names, an unavailable written name, a source-only file, an excluded or
  missing path, wrong caller/call/target pins, ownership expiry and resource limits. Verify
  that a canceled or late result leaves retained captures readable at capacity. Keep the
  native and Chromium same-file regression suites (2–6).
- Run focused serial mutation checks after a passing baseline. Measure optimized cold
  and repeated opens on a Rust and a non-Rust reference repository. Record the commit,
  caller, callee, working-set size and coverage. Check that acquisition refusal on a
  larger repository leaves the original bubble readable.

The native matrix passed for all eight adapters. The mixed-language Chromium workflow
passed, as did the full native and default Chromium suites with their opt-in checks
excluded. The focused `cargo-mutants` run caught all ten selected retention and bound
mutations after a passing baseline. This covers the additional-file admission and
confirmation rules.

### Reference repository observations

The optimized native host used two workers and a fresh cache for each repository.
Each working set held the caller and its confirmed callee. Both selected files supplied
one name candidate with zero reported declaration or analysis omissions.

| Repository and commit | Caller → callee | First candidate lookup | First open preparation | Seven retained opens, median (range) |
| --- | --- | ---: | ---: | ---: |
| lpc-rs `4a7f30d992ff` | `src/compile_time_config.rs` → `src/runtime.rs` | 9.65 ms | 9.33 ms | 1.11 ms (0.87–1.29 ms) |
| PyTorch `1f4d2d5e820d` | `torch/_dynamo/test_dont_skip_tracing_functions.py` → `torch/compiler/__init__.py` | 6.74 ms | 11.47 ms | 0.98 ms (0.78–1.59 ms) |

These are local HTTP timings after indexing and caller selection. Open preparation follows
a required candidate lookup, which can warm the analysis cache. Source confirmation is a
separate request. PyTorch's configured
`aten/src/ATen/native/SobolEngineOpsUtils.cpp` was refused in 8.12 ms at 2,171,256 bytes,
above the 2 MiB per-file limit. The caller's pinned Calls query still returned its
original observation.
