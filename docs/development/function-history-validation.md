# Function-history browser qualification

Contributor evidence for [issue #4](https://github.com/fixed-width/vizzi/issues/4),
recorded on 2026-11-16. This integration record supplements the focused
[history](../visual/HISTORY_TECH.md#observed-validation-2026-11-16),
[function-comparison](../visual/FUNCTION_COMPARISON_TECH.md#observed-validation-2026-11-16)
and [captured-call](../visual/CAPTURED_CALLS_TECH.md#observed-validation-2026-11-16)
checks. It records functional qualification; representative-repository performance
and independent reading/review usefulness remain separate work.

## Tested revisions and CI

[PR #10](https://github.com/fixed-width/vizzi/pull/10) ended at
`4d8bebc10807e8f3b8ecf3db96421c9e1ee6511c` and merged as
`e9d809900c588ccd1b9e3900a4dbbf0d8fab2c79`. Both commits have the same tree,
`43a6ed588eab22d07b61c2acc19ae23c224741bf`. The local run uses the merge commit;
the qualification change adds documentation only.

The complete **Browser and tutorials** job passed on both revisions:

- [Final PR commit](https://github.com/fixed-width/vizzi/actions/runs/43977595950/job/133685484426).
- [Merge commit](https://github.com/fixed-width/vizzi/actions/runs/43979296502/job/133691282085).

Both jobs used Ubuntu 24.04 x86-64, Playwright 1.64.1 and its Chromium/Chrome
Headless Shell 156.0.8078.4 (build 1248). CI setup selected Node 26.13.1 and npm
11.20.0; the package pins Node 26.13.0, selected by npm scripts, and the tutorial
command explicitly uses that packaged runtime. Python 3.15.2 and Ruby 4.0.9 were
installed by CI. Rust was stable 1.100.0.

The commands were the normal CI sequence:

```sh
npm ci
node_modules/.bin/playwright install --with-deps chromium
cargo build --locked --release -p vizzi
npm run test:editor
npm test -- --no-build
node_modules/node/bin/node experiments/browser/documentation.test.mjs --no-build
python3 -m unittest discover -s scripts/release -p 'test_*.py'
```

The release host serves embedded assets through the normal default runner.
The runner invokes all 87 entries and exits on the
first unsuccessful child; its packaged-host children can perform additional cached
offline release builds because the runner does not forward `--no-build` to them.

| Check | Final PR CI | Merge CI |
| --- | --- | --- |
| Complete default browser runner | All 87 entries passed; `node:test` reported 371 passed, no failures or skips | Same |
| Ordinary editor suite | 166 passed, 2 skipped, no failures | Same |
| Python, Rust, Ruby and C/C++ tutorials | All four passed | Same |
| Release helper tests | 13 passed | Same |

The 371 count sums the runner's `node:test` summaries. Several other entries use
standalone assertions and a completion message, including the five workflows below;
they are required for runner success but do not contribute to that count.

The [final PR native jobs](https://github.com/fixed-width/vizzi/actions/runs/43977595950)
and [merge native jobs](https://github.com/fixed-width/vizzi/actions/runs/43979296502)
also passed `bash scripts/ci/check-native.sh`: formatting, locked all-feature
workspace tests/doctests and Clippy over all targets/features with warnings denied.
Each Linux run reported 825 passing tests/doctests; each macOS ARM64 run reported
808. Each platform explicitly ignored 15 opt-in integration checks. These are
separate from the browser results and are not counted as passing ignored checks.

## Required regressions in the default runner

All five ran successfully in both complete CI runs and the local run, alongside the
older navigation, live-source, layout, task, module, delivery, review and refresh checks.

| Browser entry in `experiments/browser/` | Verified behaviour |
| --- | --- |
| `function-history.test.mjs` | Skipped unrelated edits, back/forward, busy concurrent restoration, paging/reload, stable bubbles, historical links/actions, independent live edits, failure retention and supersession |
| `function-comparison.test.mjs` | Changes beyond the first source page, original line numbers/copying, coordinated edits, independent frozen captures, retry/reload, historical adjacency, retained layout, minimum size and cancellation |
| `captured-calls.test.mjs` | Exact old helper bodies, cycles, inline/Calls ambiguity, provider isolation, unresolved external calls, later sections, live coexistence, reload, comparison sides, newer-editor cancellation and Return-to-live choice cancellation |
| `comparison-delivery.test.mjs` | A newer Explore Call cancels comparison preparation; late success/failure cannot pin its caller, add a pair, change focus or leave loading controls |
| `comparison-edit-cancellation.test.mjs` | Current edit failures remain visible; uncancellable failures after editor navigation, Return to live or closing either comparison side remain silent and preserve newer state |

The default runner also includes the history, comparison and captured-call model
checks. The tutorial check compiles/runs disposable example copies using `python3`,
`rustc`, `ruby`, `cc` and `c++`, then exercises calls, groups and restored tasks in
Chromium. Its optional `--with-rust-analyzer` branch is not part of browser CI.

## Local environment and reproduction

The local checkout initially had no `target/` directory. Dependencies and Playwright
browsers were already installed; no dependency update or browser download was needed.
The rebuilt `target/release/vizzi` SHA-256 was
`2d630cad1b97d5cd129084c0bb0ad5245f152f3ff0ee58aa439f118d2b5f0bb3`.

The host was Linux Mint 22.3 x86-64, kernel 6.8.0-149-lowlatency, with Rust/Cargo
1.100.0, packaged Node 26.13.0, npm 10.9.2 and Playwright 1.64.1. Chromium
156.0.8078.4 ran headless with ANGLE SwiftShader 5.0.0. Tutorial prerequisites were
Python 3.12.3, Ruby 3.4.1 and GCC/G++ 13.3.0. This is a separate Linux environment
from the supported Ubuntu CI runner.

Commands, from the repository root:

```sh
CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 nice -n 10 \
  cargo build --locked --release --offline -p vizzi
PATH="$PWD/node_modules/node/bin:$PATH" nice -n 10 npm test -- --capabilities
CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 PATH="$PWD/node_modules/node/bin:$PATH" \
  nice -n 10 npm test -- --no-build
PATH="$PWD/node_modules/node/bin:$PATH" nice -n 10 npm run test:editor
nice -n 10 node_modules/node/bin/node --expose-gc --test editors/vscode/*.test.cjs
nice -n 10 node_modules/node/bin/node experiments/browser/documentation.test.mjs --no-build
nice -n 10 python3 -m unittest discover -s scripts/release -p 'test_*.py'
cargo fmt --all --check
```

| Local check | Result |
| --- | --- |
| Fresh offline release build and browser capability probe | Passed |
| Complete default browser runner | All 87 entries passed; `node:test` reported 371 passed, zero failed, skipped or cancelled |
| Ordinary editor suite | 166 passed, 2 skipped, zero failed |
| Editor suite with explicit GC | 168 passed, zero failed or skipped |
| Python, Rust, Ruby and C/C++ tutorials | All four passed, including calls, groups and task restoration |
| Release helper tests | 13 passed |
| Formatting | Passed |

Local command output is retained in `/tmp/vizzi-issue-4-browser.log`,
`/tmp/vizzi-issue-4-editor-final.log`, `/tmp/vizzi-issue-4-editor-gc.log`,
`/tmp/vizzi-issue-4-tutorials.log` and `/tmp/vizzi-issue-4-release-helpers.log`.
These are temporary local logs; the linked CI jobs provide the shared run evidence.
Native workspace tests and Clippy above are CI evidence, rather than new local runs.

## Failures, skips and limits

- The [earlier CI attempt](https://github.com/fixed-width/vizzi/actions/runs/43975482881)
  at `ed2de0635e6ed7b64cdd4fce123754843f502c63` failed to compile the macOS native
  `origin_editor` fixture because `rustix::fs::mknodat` is unavailable there. This was
  a test-fixture portability problem, fixed by
  [the final PR commit](https://github.com/fixed-width/vizzi/commit/4d8bebc10807e8f3b8ecf3db96421c9e1ee6511c)
  using the existing `mkfifo` command pattern. The final PR and merge native jobs
  passed on both platforms. This failure is not a browser application regression.
- Initial local sandboxed probes failed at Chromium startup (`sandbox_host_linux.cc`,
  `shutdown: Operation not permitted`) and loopback `listen` (`EPERM`). These are
  environment restrictions, not passing or skipped checks. The enabled process
  environment could launch Chromium and bind loopback sockets.
- An early local editor attempt also preceded release-build completion: native cases
  could not spawn `target/release/vizzi` (`ENOENT`), and stdio handoff timed out. This
  prerequisite-ordering mistake is excluded from qualification results; the editor
  suites were rerun after the build completed.
- The ordinary editor command skips `completed transient selections are released`
  and `failed native acquisition releases its signal after cleanup` when `global.gc`
  is unavailable. Both passed in the separate local `--expose-gc` run (168 passed,
  zero skipped). CI's two skips remain reported as skips.

The complete runs found no application regression or remaining missing prerequisite.
The macOS fixture defect was already resolved by the final PR commit; no additional
follow-up defect was needed for these results.

This record adds no application change and no new mutation run. Earlier scoped
mutation results and their survivors, timeouts, invalid builds and exclusions remain
in the three linked feature contracts; complete mutation qualification is not claimed.
The complete browser run qualifies embedded release assets in Chromium, not Firefox,
WebKit, a headed/hardware renderer, a real installed VS Code extension or every
language-server setup. Opt-in semantic/dependency workflows and the 15 ignored native
checks remain outside this evidence. Fixture correctness does not establish
large-repository latency, the 16 GB baseline under pressure or independent usability.
