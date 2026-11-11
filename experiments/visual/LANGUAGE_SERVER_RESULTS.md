# Python and C-family semantic navigation

Contributor validation record, 2026-11-11. The [behaviour contract](../../docs/visual/LANGUAGE_SERVERS_PRODUCT.md)
and [integration reference](../../docs/visual/LANGUAGE_SERVERS_TECH.md) describe the implementation.
User setup belongs in the [configuration guide](../../docs/how-to/configure-language-servers.md).

## Behaviour checked

Real Pyright and clangd fixtures resolve typed receivers despite unrelated methods with
the same name. Python checks imported aliases, function-value references, a selected local
virtual environment and navigation through two local dependency files. C and C++ checks
cover ordinary calls, header declarations and out-of-class definitions. Live files are
deliberately replaced after capture to check that native lookups use captured source.

Incremental Python checks change configuration and insert a line before a target method.
This initially exposed a stale imported declaration position. Sending filesystem change
notifications as well as versioned text synchronization fixes the regression. Diagnostic
version acknowledgements gate changed-document requests; the deterministic protocol peer
also sends a stale acknowledgement before the current one.
Changing `extraPaths` must select a definition from the new import root. Removing a
captured module must remove it from the server's import graph.

The browser check configures both servers in one mixed Python/C++ repository. It checks
direct typed navigation, a semantic connection, no name chooser, semantic reference counts
and Python source/reference refresh after a saved edit. It disables desktop editor handoff.
The existing Rust real-server suite remains a separate regression check.

## Reproduce

The checked Linux tools were Rust/Cargo 1.99.1, rust-analyzer 1.99.1, Pyright 1.1.410,
and clangd 21.0.0. Server/toolchain installation is external to Vizzi. Use the repository's
locked browser development dependencies and installed Playwright Chromium.

```sh
cargo fmt --all --check
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 cargo test --offline --workspace
CARGO_BUILD_JOBS=1 nice -n 10 cargo clippy --offline --workspace --all-targets -- -D warnings
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 \
  cargo test --offline -p vizzi --test language_servers -- --include-ignored
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 \
  cargo test --offline -p vizzi --test rust_definitions -- --include-ignored
CARGO_BUILD_JOBS=1 nice -n 10 cargo build --offline --locked --release -p vizzi
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 npm test -- --no-build
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 npm test -- --language-servers --no-build
```

`VIZZI_CLANGD` selects an executable for the real-server tests and browser check;
`VIZZI_PYRIGHT` selects the browser test executable. Native Python fixtures currently use
`pyright-langserver` on PATH. The interpreter fixture creates a temporary environment
without pip and writes its own module; it does not download a package.

The final workspace run passed 665 tests with 13 opt-in cases ignored. The opted-in
language-server suite passed seven cases, and the existing Rust server suite passed six.
Formatting, Clippy, the full default browser suite and the mixed-language browser check
passed. Compressed logs accompany the mutation outcomes.

The execution sandbox blocks Chromium startup and Node interpreter subprocess socket
pairs. The Swiftly clangd launcher also fails there, while the real clangd executable
works. Browser and interpreter qualification therefore ran outside that sandbox. These
failures are environment restrictions, not caught mutations or product test failures.

## Mutation evidence

Runs use `CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 cargo mutants`,
`-j 1 --jobserver-tasks 1`, and a passing unmodified baseline. Complete C-family adapter,
router and project-input scopes supplement focused checks of the shared project adapter.
The initial input/router survivors prompted stronger language, path and exact-boundary
tests. C declarator tests now include pointer returns and calling-convention wrappers.

| Run | Caught by failing tests | Missed | Did not compile |
| --- | ---: | ---: | ---: |
| Complete language router | 2 | 3 | 4 |
| Router follow-up | 4 | 0 | 0 |
| Complete C-family adapter | 38 | 1 | 13 |
| C declarator follow-up | 5 | 0 | 7 |
| Complete project-input adapter | 47 | 36 | 3 |
| Input survivor follow-up | 25 | 2 | 0 |
| Input follow-up with remapped source coordinates | 14 | 0 | 0 |
| Relative object-path follow-up | 1 | 0 | 0 |
| Focused project session/synchronization/resolve guards | 13 | 0 | 1 |

The focused project run covers configuration-session reuse, caller version/identifier
checks, document opening, source synchronization and removal. It does not cover the
entire project adapter or establish complete dependency/version invalidation coverage.
The surviving `Component::CurDir` deletion is equivalent for the absolute base paths
supplied by the host: path components already normalize internal `.` segments. It is
retained as a missed case in the evidence, not counted as caught. The other input
survivor exposed a missing check for relative object-file paths in compiler arguments;
the follow-up adds that behavioural assertion and catches the mutation. No completed
mutation run timed out.

Detailed [compressed outcomes](results/language-servers/) are recorded beside this report.
Counts distinguish failing tests
from mutations that cannot compile. Follow-up runs overlap earlier scopes and must not
be added as independent coverage. The early router invocation rejected incorrectly placed
test arguments before testing mutations; it contributes no mutation evidence. A C
declarator invocation rejected the unsupported `cargo mutants --offline` option and was
rerun with `CARGO_NET_OFFLINE=true`; that rejected invocation tested no cases either.

## Limits

These are deterministic fixture and browser correctness checks, not a large-repository
performance benchmark or cross-platform qualification. Server versions, project settings,
dependency availability and unsupported syntax affect coverage. Semantic targets can be
stubs or headers; they do not establish runtime dispatch. External dependencies and build
configuration are not watched recursively. Source/configuration/catalog bounds remain
explicit, and written-name candidates remain a labelled fallback.
