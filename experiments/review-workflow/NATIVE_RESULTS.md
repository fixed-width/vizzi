# Source-only native acquisition validation

The native `ReviewInput` seam now captures directory inputs, pinned Git trees and
observed working files into the same portable comparison model. The
[reference](../../docs/prototype/REVIEW_ACQUISITION.md) describes the delivered support,
source-only coverage, platform limits and admission work still required.

## Behavioural evidence

The first supplied-input test failed against the unimplemented capture function.
Git tests were written before the native Git interface existed. The final behavioural
suite exercises independently specified outcomes through the list/read/capture
interfaces, plus bounded protocol/OS validation at the acquisition seam:

- staged/unstaged/untracked edits, standard ignore rules, disk reversion of staged
  changes and restored files after staged deletion;
- pinned refs after movement, peeled annotated tags and shared directory/Git/memory
  workspace identities;
- assume-unchanged and unmerged index states without substituting staged bytes;
- raw object reads despite archive attributes/filters and disabled fsmonitor commands;
- literal symlink values, executable metadata, empty files, binary and unusual paths,
  no-follow ancestors and special-file refusal;
- sparse/partial support refusal, manual skip-worktree absence, missing objects and
  scope/exclusion coverage without false deletion/equality;
- independent byte/path boundary expectations, multi-chunk reads, resource refusal,
  cancellation, changing files, atomic saves and replaced roots;
- nested pointer changes with explicit unassessed child contents, without traversing
  child sources;
- protocol validation, descriptor close-on-exec and child termination/reaping.

No analysed-project scripts/builds or network fetches are invoked. Safe Rust uses the
already resolved rustix 1.1.6 release as a direct Unix filesystem dependency; unsafe
code remains denied. Existing source-policy defaults and live analysis scheduling
remain unchanged.

All 351 all-features workspace tests, doctest checks, formatting and Clippy pass.

## Frozen task inputs

Both prepared variants were captured directly through the native Git adapters and
checked against the independently frozen inventory and hunk oracle. Each base has
1,032 paths and each working observation 1,033, including the added Markdown file.
Both compare exactly two changed paths. Variant A exposes three Rust hunks, including
the unstaged guard change; variant B exposes two. The added document has an absent
base and one complete text hunk. Repeating capture for unchanged inputs preserves
comparison identity. Preparation status/bytes still verify after capture.

The [recorded native checks](results/native-fixtures.json) retain identities, path/byte
counts, notices and selected hunks. They are functional source-acquisition evidence,
not timing/RSS measurements. The originals were not edited or executed. Reproduce via
the [fixture how-to](README.md#check-native-git-capture).

## Verification scope

The [native mutation summary](results/native-mutation.json) records complete-module
mutation testing, final source/test hashes and baseline checks. Earlier runs exposed
missing exact-boundary and multi-chunk assertions; those were added. Duplicated open
flags and metadata checks were consolidated into tested OS/validation operations.

The narrowly documented chunk-size exclusion preserves a nonzero buffer below the
64-KiB scratch bound; only throughput/checkpoint frequency changes. Initial exclusion
matching was corrected against the generated list. Raw outcomes and any equivalent
survivor/timeouts remain separate from mutations caught by assertions.

The final low-priority, single-job run tested 211 cases: 171 caught by test failures,
37 unviable, two timeouts and one investigated equivalent chunk-size survivor. The
corrected exclusion selects 210 cases; it was validated with `--list` after the run,
without changing the raw result count. The two timeouts introduce respectively a
non-progress body loop and a framed-body over-read. Neither counts as assertion coverage.
Final formatting only changed `disk.rs`; the report retains tested/final hashes and
the formatting patch, verified against rustfmt output. There are no uninvestigated
behaviour-changing survivors in this run; generated patterns remain incomplete evidence.

At the user's request, subsequent mutation runs use one mutant and one compiler/
jobserver task at lower priority. Serial test execution is also the durable default
for future runs in [contributor guidance](../../AGENTS.md). This is resource-sharing
guidance, not a product design requirement.

This slice does not run language analysers, integrate the normal parser/cache workers,
admit global pair/source owners, publish host comparisons or demonstrate linked review
views. Linux is the verified platform. The next work is admission and integration with
the existing analysis scheduler before host/browser exposure; no comprehension or
large-repository performance claim follows from these checks.
