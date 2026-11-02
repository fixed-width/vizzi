# Run a source workspace

Use these instructions to inspect a local directory of Rust and/or Python source. Build Vizzi with
a current stable Rust toolchain; the lockfile records the tested dependencies. No build
commands are run in the directory being inspected.

From the Vizzi checkout:

```sh
cargo run --release -p vizzi -- \
  --root ../lpc-rs \
  --cache /tmp/vizzi-cache/lpc-rs
```

Open **http://127.0.0.1:7474** in a browser. All assets and source data are local. The
server binds to loopback. Choose another port with `--port 7475` if needed. Stop it with
Ctrl-C. The cache must be outside the inspected workspace, including through symlinks.

Select a file on the map to open its frozen source and declaration outline. Select a
declaration to reveal its lines. Drag to pan, scroll to zoom, or choose **Fit map**.
Filtering limits the overview while preserving an existing selection. The expandable
file list provides keyboard access to file selection; arrow keys pan a focused map.

Source is read only. Edit a file in your editor to trigger a refresh. The status shows
analysis in progress while the previous observation remains usable. Syntax recovery
appears beside the source. The outline describes raw syntax, without macro expansion
or compiler name resolution.

The host discovers `.rs` and `.py` files, skips symlinks and root `.git`, `target`, and
`node_modules` directories, and preserves nested source directories with those names.
It shares four parsing workers across both languages by default; choose 1–32 with
`--workers`. Other extensions are omitted; see the [language scope](../prototype/LANGUAGES.md).
The overview
returns at most 2,000 files per filtered query. Use a path filter to narrow larger scopes.

## Show C-family source

Enable candidate capture for a relative subtree with repeatable `--c-family-root`:

```sh
cargo run --release -p vizzi -- \
  --root ../pytorch --cache /tmp/vizzi-cache/pytorch \
  --c-family-root c10 \
  --syntax-language c10/util=cpp \
  --exclude-source c10/vendor
```

Use `.` for the whole workspace. `--syntax-language SCOPE=c|cpp` records an explicit
raw-language request; more specific scopes win and conflicting assignments at one
scope fail startup. `--exclude-source SCOPE` removes that path/subtree from all capture,
including default Rust/Python inputs. Paths are relative to `--root`; traversal and
absolute scopes are rejected. Options can be repeated and do not depend on ordering.

The current host has no C/C++ analyzers, so these candidates remain source-only.
Unassigned `.h` files explain their ambiguity; requested C/C++ and CUDA inputs explain
the unavailable analyzer/dialect. Source remains readable when UTF-8. The inspector
shows the assignment evidence, and overview counts separate captured files from files
with analysis. Raw Rust/Python outlines state that configuration coverage is unknown.
No compilation database, compiler, build, or project script is invoked.

Keep C-family capture disabled for lpc-rs: its LPC fixtures also use `.c`/`.h` suffixes.
Use explicit exclusions for known LPC subtrees in mixed repositories. See the
[source-policy reference](../prototype/SOURCE_POLICY.md) for candidate suffixes,
normalization, availability, and identity rules.

## Inspect without a browser

For a single capture and JSON status report:

```sh
cargo run --release -p vizzi -- \
  --root ../lpc-rs --cache /tmp/vizzi-cache/lpc-rs --once
```

For a live process accepting JSON requests on stdin:

```sh
cargo run --release -p vizzi -- \
  --root ../lpc-rs --cache /tmp/vizzi-cache/lpc-rs --stdio
```

Send one request per line:

```json
{"id":1,"uri":"/api/status"}
{"id":2,"uri":"/api/view?filter=compiler&limit=100"}
```

Each response contains the same HTTP handler status, content type, and text body used
by the browser. JSON bodies are encoded as strings inside the diagnostic envelope.
The streaming `/api/events` endpoint is available through HTTP or native subscriptions;
the finite diagnostic mode rejects it. See the [API reference](../prototype/API.md)
for pinned source and view requests.

## Handle a failed refresh

An unreadable source file leaves the previous observation available. Fix the filesystem
error and wait for reconciliation. If native watching is unavailable, the host reports
polling mode and checks the workspace every two seconds. Cache write failures appear
in the status measurements; valid source analysis continues without persistence.

Use the [browser validation procedure](../../experiments/browser/README.md) for automated
UI checks and renderer measurements. Stdio remains useful in environments without
socket access. Production renderer and baseline-laptop qualification remain separate
from the current bounded reference-view measurements.
