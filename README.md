# Vizzi

Vizzi helps you follow a call chain without losing the code you came from. Open functions
as connected source bubbles, keep earlier steps and branches in view, and return to the
same arrangement when you pick up a task again.

It runs locally beside your editor. Source exploration is read-only; editing, refactoring
and debugging stay in the editor.

![A call workspace with a caller and two branches](docs/images/call-workspace.png)

## Try it

Vizzi is an early preview built from source. The tested setup is Linux with a recent
Chromium-based browser. You need a current stable Rust toolchain and native C build tools;
Node.js is not needed to run Vizzi. See the [installation guide](docs/how-to/install.md).

From a Vizzi checkout:

```sh
cargo build --locked --release -p vizzi
./target/release/vizzi \
  --root /path/to/project \
  --cache "$HOME/.cache/vizzi/my-project" \
  --workers 1
```

Open the printed address, normally <http://127.0.0.1:7474/>. Select **Add bubble**, find a
function, and follow a highlighted call. The cache must be outside the project you inspect.
Stop the server with Ctrl-C.

For a guided first run, [follow a Python call trail](docs/tutorials/python-call-trail.md)
or [a Rust call trail](docs/tutorials/first-call-trail.md) using the included examples.
No project build, language server or editor integration is required for either lesson.

## What you can do

- Follow calls into neighbouring bubbles, inspect callers and references, and keep branches
  visible. Rust type-aware navigation is available with an optional rust-analyzer setup.
- Arrange related views into groups and save browser-local tasks. Follow saved file changes
  live or pin an earlier source view.
- Browse source, compare Git revisions or working changes, and keep source locations and
  notes. Hand off a current location to VS Code when you want to work on it.

Rust and Python are selected by default. Raw C/C++ browsing uses explicit source scopes.
Language support and call resolution have [important differences](docs/reference/languages.md);
a candidate match is not proof of a runtime call.

## Documentation

- [Documentation home](docs/README.md)
- [Run Vizzi on your project](docs/how-to/run-workspace.md)
- [Configure Rust navigation and local dependency source](docs/how-to/configure-rust.md)
- [Review changes](docs/how-to/review-local-changes.md)
- [Troubleshooting](docs/how-to/troubleshoot.md)
- [Changelog](CHANGELOG.md)

[Contributing](CONTRIBUTING.md) covers development and verification. Design contracts,
plans and experiment records are indexed in the [contributor documentation](docs/development/README.md).
