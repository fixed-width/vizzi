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

Assigned C/C++ files now show raw definition/type-alias outlines. Unassigned `.h` files
remain source-only and explain their ambiguity; CUDA remains an unavailable dialect.
Source remains readable when UTF-8. The inspector shows assignment evidence, and
counts separate captured files from files with analysis. All raw outlines state that
configuration coverage is unknown. See the [extraction reference](../prototype/C_FAMILY_SYNTAX.md)
for supported forms, template/overload behavior, and limits.
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

To try retained syntax trees for repeated edits, add `--syntax-cache-mib 512`.
The default is disabled. This limits admitted cache weight and entries, rather than
whole-process memory. See the [retention contract](../prototype/SYNTAX_RETENTION.md)
for ownership, fallback, and resource accounting.

The browser fetches [pinned source/outline windows](../prototype/WINDOWS.md) automatically.
Generated long lines expose text-fragment controls; outline Home/End navigate the whole
logical list. The diagnostic `/api/view` and `/api/source` operations retain explicit
bulk access, while `projection=summary` and `/api/window` are available to other hosts.

To opt into a shared active analysis input budget, add `--analysis-input-mib 32`.
This permits at most 32 MiB of active input, with a 24 MiB per-file/recovery limit
and 8 MiB reserved from recovery for foreground work. It does not cap parser heap or
captured source storage. Refused files stay readable with explicit deferred analysis;
eligible jobs retry during recovery. `0` disables admission and is the default.
See [the admission reference](../prototype/ADMISSION.md) for cache, retry and resource semantics.

To prototype private source backing, add `--source-spool-mib 16384`. The anonymous
spool is created in the external cache directory and can append at most 16 GiB during
the session. Unchanged verified sources reuse backing; obsolete/cancelled versions still
consume session disk capacity until the last store/source descriptor is released.
Capacity failure keeps the previous published observation. `0` preserves resident
capture and is the default. See [source reads](../prototype/SOURCE_READS.md) for lifetime,
range/bulk limits, errors and the remaining transient-capture/parser/index limits.
