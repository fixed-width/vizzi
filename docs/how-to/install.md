# Build and install Vizzi

Use this guide to build the local server. You do not need a frontend build step, a model download or a GPU for code analysis. Node.js is only
needed if you choose a language server that uses it, such as the Node-based Pyright server.

## Prepare the build tools

The checked setup is Linux with a recent Chromium-based browser. The commands below use
a POSIX shell. Other platforms and browsers have not yet been qualified.

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

## Build the server

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

[Follow a Python call trail](../tutorials/python-call-trail.md) or
[a Rust call trail](../tutorials/first-call-trail.md) for a self-contained lesson, or
[run Vizzi on your project](run-workspace.md).

Language servers are optional for basic browsing. [Configure semantic navigation](configure-language-servers.md)
for Python, Rust or C/C++ project-aware definitions, references and local dependency source. VS Code is
only needed for the editor handoff. The [troubleshooting guide](troubleshoot.md) covers
build tools, startup and missing results.
