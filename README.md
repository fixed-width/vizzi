# Vizzi

Vizzi helps you visualize your code, follow a call chain without losing the code you came from, and assist reviews. 

It runs locally beside your editor. Source exploration is read-only; editing, refactoring
and debugging stay in the editor.

![A call workspace with a caller and two branches](docs/images/call-workspace.png)

## Try it

Vizzi is an early preview. [Download a release](https://github.com/fixed-width/vizzi/releases/latest)
for Linux or macOS on x86-64 or ARM64, or build from source below. Prebuilt executables
do not need Rust or a C compiler. Browser checks use Chromium on Linux; native checks
also run on macOS. Basic browsing does not need Node.js. Optional language servers have
their own runtime requirements. See the [installation guide](docs/how-to/install.md).

To build from a Vizzi checkout, install a current stable Rust toolchain and native C
build tools, then run:

```sh
cargo build --locked --release -p vizzi
./target/release/vizzi \
  --root /path/to/project \
  --workers 1
```

Open the printed address, normally <http://127.0.0.1:7474/>, to browse source. To follow
a call, choose **Call workspace** in the header, then select **Add bubble** and find a
function. With `vizzi` on your `PATH`, you can run it
without arguments from the project directory. `--root` selects another directory;
`--cache` overrides the automatically chosen [workspace cache](docs/reference/storage.md#cache-location).
Stop the server with Ctrl-C.

Using VS Code? [Install the bundled extension](editors/vscode/INSTALL.md), then right-click
a saved call name and choose **Vizzi: Explore Call** to open its caller and definition
together in an editor tab.

Using Neovim? [Load the companion plugin](editors/neovim/INSTALL.md) and run `:VizziOpen`
to open the browser workspace. Use a bubble's **Editor** button to jump back to its
saved source in Neovim.

For semantic navigation on a real project, [configure its language server](docs/how-to/configure-language-servers.md):
`--pyright pyright-langserver` for Python, `--rust-analyzer rust-analyzer` for Rust,
`--ruby-lsp ruby-lsp` for Ruby, `--clangd clangd` with C/C++ compilation context, or
`--jdtls jdtls` for Java.

For a guided first run, follow a [Python](docs/tutorials/python-call-trail.md),
[Rust](docs/tutorials/rust-call-trail.md), [Ruby](docs/tutorials/ruby-call-trail.md)
or [C/C++](docs/tutorials/cpp-call-trail.md)
call trail using the included examples. No project build, language server or editor
integration is required for these lessons.

## What you can do

- Follow calls into neighbouring bubbles, inspect callers and references, and keep branches
  visible. Configured language servers resolve typed receivers, imports and references
  for Python, Rust, Ruby, C/C++, and Java.
- Arrange related views into groups and save browser-local tasks. Follow saved file changes
  live or pin an earlier source view.
- Browse source, compare Git revisions or working changes, and keep source locations and
  notes. Hand off a current location to VS Code or Neovim when you want to work on it.

Rust, Python, Ruby, Java, JavaScript, and TypeScript are selected by default, including
JSX and TSX. Raw C/C++ browsing uses explicit source scopes.
Language support and call resolution have [important differences](docs/reference/languages.md);
a candidate match is not proof of a runtime call.

## Documentation

- [Documentation home](docs/README.md)
- [Run Vizzi on your project](docs/how-to/run-workspace.md)
- [Configure semantic navigation and local dependency source](docs/how-to/configure-language-servers.md)
- [Review changes](docs/how-to/review-local-changes.md)
- [Troubleshooting](docs/how-to/troubleshoot.md)
- [Changelog](CHANGELOG.md)

[Contributing](CONTRIBUTING.md) covers development and verification. Design contracts,
plans and experiment records are indexed in the [contributor documentation](docs/development/README.md).
