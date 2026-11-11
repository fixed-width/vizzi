# Contribute to Vizzi

For contributors changing the implementation or documentation. If you want to try Vizzi,
start with the [user documentation](docs/README.md).

Read [AGENTS.md](AGENTS.md) for contributor standards, [DESIGN.md](DESIGN.md) for current
scope and architecture, and [CONTEXT.md](CONTEXT.md) for domain terms. Editing, refactoring
and debugging inside the product are outside scope; the visual workspace supplements an editor.

## Build and verify

[Build the server](docs/how-to/install.md) first. Normal verification is:

```sh
cargo fmt --all -- --check
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 cargo test --workspace
CARGO_BUILD_JOBS=1 cargo clippy --workspace --all-targets -- -D warnings
```

The browser checks are separate from running the product. Their dependencies and browser
installation are described in the [browser procedure](experiments/browser/README.md).
Node.js/Playwright are development tools, not runtime prerequisites for users.

Prefer focused tests and deterministic fixtures before broader checks. Use the mutation
testing policy in AGENTS.md: one low-priority worker/compiler task with serial tests.
Performance claims require separate optimized-build measurements and their coverage limits.

## Keep user documentation current

The [documentation home](docs/README.md) is organised around tutorials, task guides,
reference and explanation. Keep a tutorial on one predictable path; record exact controls
and commands in reference, and explain uncertainty separately from task instructions.

Update the relevant user pages and [Unreleased changelog](CHANGELOG.md) with notable
user-visible changes. Verify runnable examples and interface labels when they change.
Keep the root README focused on the product and a first run. It is not a progress log,
release plan or experiment index.

The tutorial check exercises both languages, task restoration and saved-note persistence:

```sh
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 \
  node experiments/browser/documentation.test.mjs
```

It explicitly runs the toy examples and needs the browser-test dependencies, Python 3
and Rust. Vizzi itself does not build or execute the inspected project. Add `--with-rust-analyzer` to
check the optional provider when its prerequisites are installed. `--update-images`
refreshes the README screenshot; without it, the check writes only temporary artifacts.

The [contributor documentation](docs/development/README.md) indexes feature contracts,
historical investigations and validation procedures. Those records remain separate from
what users need to install and operate the current prototype.
