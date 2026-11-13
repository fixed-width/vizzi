# Install Vizzi

Use a prebuilt executable or build the local server from source. You do not need a
frontend build step, a model download or a GPU for code analysis. Node.js is only needed
if you choose a language server that uses it, such as the Node-based Pyright server.

## Install a prebuilt executable

Download an archive and `SHA256SUMS` from the
[latest release](https://github.com/fixed-width/vizzi/releases/latest). Choose the target
for your operating system and processor; `uname -m` reports the processor architecture.

| System | Processor | Archive target |
| --- | --- | --- |
| Linux | x86-64 (`x86_64`) | `x86_64-unknown-linux-musl` |
| Linux | ARM64 (`aarch64`) | `aarch64-unknown-linux-musl` |
| macOS | Intel (`x86_64`) | `x86_64-apple-darwin` |
| macOS | Apple Silicon (`arm64`) | `aarch64-apple-darwin` |

Compare the archive's SHA-256 with its entry in `SHA256SUMS`, using `sha256sum ARCHIVE`
on Linux or `shasum -a 256 ARCHIVE` on macOS. Extract the archive, then run the `vizzi`
executable inside it. For example, for the Linux x86-64 release:

```sh
tar -xzf vizzi-v0.1.1-x86_64-unknown-linux-musl.tar.gz
cd vizzi-v0.1.1-x86_64-unknown-linux-musl
./vizzi --help
```

Copy `vizzi` to a directory on your `PATH` if desired. No Rust or C compiler is needed
to run it. The browser assets are embedded. Linux executables use static musl linking;
macOS executables are ad-hoc signed and are not notarized. Git is needed for Git review
comparisons. Native checks run on Linux and macOS; browser checks use Chromium on Linux.

## Build from source

Install a current stable Rust toolchain using the [official Rust instructions](https://rust-lang.org/tools/install/).
You also need a C compiler and native linker: Vizzi's syntax parsers include C code.
Install your operating system's normal native development tools if those are missing.
Git is needed to obtain a checkout and to use Git review comparisons.

Check that the tools are available:

```sh
rustc --version
cargo --version
cc --version
git --version
```

The documentation was checked with Rust/Cargo 1.99.1. Vizzi uses Rust 2024; the project
does not yet declare a minimum supported compiler version.

Get access to the repository and clone it, or use a source checkout provided by the
maintainer:

```sh
git clone https://github.com/fixed-width/vizzi.git
cd vizzi
cargo build --locked --release -p vizzi
```

Cargo downloads the locked build dependencies on the first build. Once they are cached,
`--offline` can be added for an offline build. The executable is `target/release/vizzi`;
its browser assets are embedded, so there is no separate web server to start.

Check the executable:

```sh
./target/release/vizzi --help
```

You can keep running it from this checkout or copy the executable to a directory on
your `PATH`. Rebuild it after updating the checkout; it does not update itself.

## Make a first run

The [Python](../tutorials/python-call-trail.md), [Rust](../tutorials/rust-call-trail.md),
[Ruby](../tutorials/ruby-call-trail.md) and [C/C++](../tutorials/cpp-call-trail.md)
call-trail lessons use included examples.
To inspect existing code, [run Vizzi on your project](run-workspace.md).

Language servers are optional for basic browsing. [Configure semantic navigation](configure-language-servers.md)
for Python, Rust, Ruby or C/C++ project-aware definitions, references and local dependency source. VS Code is
optional, for editor handoff or [shared language analysis](../../editors/vscode/INSTALL.md).
The [troubleshooting guide](troubleshoot.md) covers
build tools, startup and missing results.
