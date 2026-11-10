# Follow a call trail without replacing its caller

For a reader tracing code across functions. The visual workspace is a read-only prototype;
use your editor for changes.

Start the local host with source and a cache outside that source directory:

```sh
CARGO_BUILD_JOBS=1 cargo build --release --offline -p vizzi
target/release/vizzi --root /path/to/repository --cache /tmp/vizzi-call-trail --workers 1 --port 7474
```

Open `http://127.0.0.1:7474/workspace`. Select **Add fragment**, find a function by name or qualified name (for example `LufsMeter::process_sample`),
and choose its source location. Its captured declaration opens on the canvas.
The function starts at the code gutter, with its enclosing file/class/implementation
indentation removed from display. Internal nesting stays relative to it. This applies
across sections and both code-layout modes; copied source keeps its original indentation.

Click an underlined call name in the source, then choose **Open definition**. Use
Ctrl-click (Cmd-click on macOS) to follow it immediately. A supported source-local destination opens directly. If
several unresolved declarations share the terminal name, choose a candidate using its
containing declaration and file. The destination
opens beside the caller; an already-open destination is revealed instead. Follow another
step, or return to an earlier caller and open a branch. Existing panes retain their geometry
and reading positions. Solid **self call** arrows record Rust receiver bindings; dashed **candidate** arrows
record name-based choices. [Destination selection](../explanation/call-destinations.md) explains
the distinction.

With the keyboard, Tab to a call name and press Enter, Space or Down to open its menu;
Ctrl/Cmd+Enter follows directly. Escape closes the menu and returns focus to the name.
You can still select and copy source text. **Calls** lists written calls when their
identifiers are outside the current section or cannot be decorated.

Drag a header to arrange a fragment. Drag its lower-right grip to resize it; focus a header
or grip and use arrow keys for the same operations. Collapse an earlier step to retain its
place while freeing space. Closing a fragment removes its visual connections.

Click **×** at the right of a transient notice or error banner to dismiss it.

Scroll inside code to read it. **Next section** and **Previous section** page long declarations.
Long lines reflow at suggested code boundaries as you resize the pane. A continuation mark
in the gutter refers to the same original source line. Use **More → Code layout → Original
lines** to turn reflow off; the preference survives reload. Copying within fully captured
rows preserves original whitespace and line endings. Long indivisible tokens can still
scroll horizontally. See [source reflow](../explanation/source-reflow.md) for the design
and fallback limits.

Drag or scroll the background to pan, use **+**/**−** or Ctrl/⌘ + wheel to zoom, and use
**Fit working set** or a fragment in the overview to return to earlier context.

The layout and reading positions restore on reload in the same browser and origin. Live
bubbles follow files saved in your editor, retaining their arrangement. Reading positions
follow an unchanged nearby source line where it can be identified; otherwise the view
returns to the function start and says its reading position was reset.

Use a bubble's **Pin version** to keep its current source. **Follow live** resumes updates.
Pinning is separate from **Keep**. Older pinned versions disable current-source actions;
their loaded text stays visible. Further paging or reload can report capture expiry.
A removed, renamed, moved or ambiguous function retains its earlier view with a notice;
reopen it through **Add fragment** to choose its current identity. Historical arrows say
**previous call** or **removed call** until current evidence or a fresh follow validates
them. Unsaved editor buffers need a future plugin; this mode follows disk saves.

Use **Editor** to open a current fragment at its original declaration line in VS Code.
The host checks the captured location and working-file contents first. **Keep** independently
retains a location in the investigation; saving is optional for editor handoff. The host's
`--editor-code /path/to/code` option selects the trusted launcher. These actions are explicit;
opening a fragment never launches an editor automatically.

For the known Minitosis example, point the host at `tract-plugin-pack` and open
`/workspace?demo=minitosis`. Follow `process → render → Processor::process_sample` and
branch from `render` to `Processor::begin_block`. The candidates in `minitosis/src/engine.rs`
contain the latter two methods. This is a guided example, not an automatic answer to an
input/output tracing question.

The [prototype contract](../visual/PRODUCT.md) records limits and the
[working-set explanation](../explanation/visual-working-sets.md) describes the design rationale.

To enable project-aware Rust navigation, install the matching server with
`rustup component add rust-analyzer`, then add `--rust-analyzer rust-analyzer` to the host
command. The selected root must be a Cargo project/workspace. Dependency metadata is read
offline; if dependency sources are missing, fetch the locked metadata explicitly with
`cargo metadata --locked --manifest-path /path/to/repository/Cargo.toml`. Neither lookup
nor that metadata operation builds the repository.

With the provider enabled, `GsMeter::process` → `self.lufs_meter.process_sample` opens
`LufsMeter::process_sample` in `gs-meter/src/lufs.rs` directly. Opening a function starts a single relevant background preparation request.
A selected call waits for preparation if necessary and is then resolved again against the
current capture/configuration. Subsequent lookups reuse the private server. Basic browsing stays
available when the server or project analysis is unavailable.

Fragment search accepts `::` and `.` between captured namespace/type names and ignores
case. Exact names come first, then exact qualified suffixes, function-name prefixes/substrings, namespace prefixes/substrings and file-path
matches. Within a group, results sort by function name, qualified
name, path and source position. Ranking and function filtering happen before pagination,
so exact functions cannot be hidden behind a page of partial/type matches.
