# Ruby support validation

Contributor evidence for [the Ruby contract](../../docs/prototype/RUBY.md).
User setup belongs in the [language-server guide](../../docs/how-to/configure-language-servers.md#ruby-with-ruby-lsp).

## Checked behaviour

Tree-sitter Ruby 0.23.1 supplies build-free declarations, colouring, reflow and written
calls. Tests cover singleton/instance identity, punctuation and Unicode names, endless
methods, assignment ordering, parameter/block/pattern/rescue locals, name/local limits,
literal contents, reopened module cues and incremental/fresh-analysis equivalence.

Ruby LSP 0.26.11 ran with Ruby 3.4.1 on Linux, installed in a temporary gem directory.
The native integration selects a known receiver over an unrelated same-name method,
checks references and moved source positions, and rejects execution of trap Gemfiles
and project add-ons. Additional cases cover Ruby standard-library source, installed
logger gem source and signature-only C methods. Distinct installed library versions
retain distinct captures; the adapter does not reproduce complete Bundler resolution.

The protocol peer checks indexing-progress gating, ordered document-symbol requests,
environment overrides/removals and operation without pushed diagnostic acknowledgements.
The browser check uses Python, C++ and Ruby together, including module labels and saved
edits followed by reference refresh. The default browser suite checks shared UI behaviour.

The edit regression exposed two Ruby LSP details: its incremental sync requires ranged
edits, and its empty-directory index glob can register a root file twice with equivalent
URIs. Full-capture close/open synchronization, a private empty context directory and
canonical location deduplication prevent stale or falsely ambiguous results. Coarse
Ruby method references are checked against unique definition locations before admission.

## Mutation scopes

All runs use one low-priority mutant job, one jobserver/compiler task and serial tests:
`CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 cargo mutants -j 1 --jobserver-tasks 1`.
Every interpreted run has a passing unmodified baseline. Follow-up scopes overlap and
their counts must not be added as independent coverage.

| Run | Caught by failing tests | Missed | Did not compile | Timed out |
| --- | ---: | ---: | ---: | ---: |
| Complete Ruby syntax/call adapters | 68 | 13 | 4 | 0 |
| Lexical survivor follow-up | 18 | 3 | 2 | 0 |
| Complete initial Ruby startup module | 21 | 1 | 1 | 0 |
| Inherited-environment follow-up | 1 | 0 | 0 | 0 |
| Initial unique-reference verifier | 13 | 1 | 0 | 0 |
| Changed shared protocol logic | 6 | 0 | 2 | 4 |
| Changed project-provider logic | 19 | 2 | 2 | 0 |
| Lifecycle protocol follow-up | 2 | 0 | 0 | 0 |

The remaining lexical cases are narrow grammar-equivalent mutations: changing the
`for` pattern guard to OR produces the same binding decision for grammar-admitted
identifier children (other pattern forms are handled by the later pattern check);
the two missing-callee omission-counter mutations cannot execute because each bundled
Ruby call production supplies a method or call operator, including recovered missing
tokens. They remain missed in the report and are not counted as caught.

The initial reference-verifier survivor altered a redundant RBS preference predicate.
That predicate was removed: reference targets are admitted Ruby declarations, so RBS
signatures are always excluded from their uniqueness check. Subsequent checks use the
simpler verifier. Protocol timeouts exceeded the mutation runner's 20-second test budget
while exercising bounded readiness/configuration waits; they are distinct from test
failures. A project-adapter invocation misplaced `--include-ignored`; its baseline was
rejected and it tested no mutations. The corrected invocation uses the harness separator.

The project-provider survivors prompted a deterministic server peer that rejects closing
an unopened document and rejects full-text `didChange` messages for this profile. Real
Ruby LSP tolerated those protocol violations by rereading the mirrored file; the stricter
peer checks the lifecycle contract directly through `Servers::resolve` and a changed
capture. Its follow-up is recorded separately in the table.

The lifecycle follow-up caught both survivors. The final workspace baseline passed 687
tests with 15 opt-in cases ignored. The installed-server run passed 15 cases across
Python, C/C++, Ruby and Rust; the qualification declaration probe also passed. Formatting,
Clippy, the default browser suite and the mixed-language Ruby live-update check passed.
Compressed logs and mutation outcomes are in [the evidence directory](results/ruby/).

## Reproduce

```sh
cargo fmt --all --check
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 cargo test --locked --offline --workspace
CARGO_BUILD_JOBS=1 nice -n 10 cargo clippy --locked --offline --workspace --all-targets -- -D warnings
GEM_HOME=/path/to/test-gems VIZZI_RUBY_LSP=/path/to/ruby-lsp \
  CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 \
  cargo test --locked --offline -p vizzi --test language_servers ruby_ -- --include-ignored
npm test -- --language-servers --with-ruby --no-build
npm test -- --no-build
```

The mixed-server browser check also needs Pyright and clangd; `VIZZI_CLANGD` can select
the actual clangd binary. Chromium ran outside the execution sandbox, which blocks its
startup. These checks do not establish large-repository performance, complete dynamic
Ruby dispatch, complete dependency invalidation or cross-platform runtime qualification.
