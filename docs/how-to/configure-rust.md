# Configure Rust navigation and local dependencies

Use rust-analyzer for project-aware Rust definitions and references, including source
outside the repository. Basic browsing and the [Rust tutorial](../tutorials/first-call-trail.md)
work without it. [Python and C/C++ setup](configure-language-servers.md) uses the same
semantic navigation interface.

## Prepare the toolchain and project

With a Rustup-managed toolchain, install the server and standard-library source explicitly:

```sh
rustup component add rust-analyzer rust-src
rust-analyzer --version
```

The upstream [binary installation guide](https://rust-analyzer.github.io/book/rust_analyzer_binary.html)
and [standard-library setup](https://rust-analyzer.github.io/book/installation.html#rust-standard-library)
cover other installations. Match the server/source to the toolchain used by the project.

Choose a Cargo project or workspace as Vizzi's root. Its dependency metadata and sources
must already be available locally. If a locked project needs dependencies downloaded,
prepare them explicitly before starting Vizzi:

```sh
cargo fetch --locked --manifest-path /path/to/project/Cargo.toml
```

This preparation can use the network. Vizzi's runtime lookup remains offline: it does not
fetch dependencies, install toolchains or download standard-library source for you.

## Enable the provider

From the Vizzi checkout:

```sh
./target/release/vizzi \
  --root /path/to/project \
  --cache "$HOME/.cache/vizzi/my-project" \
  --workers 1 \
  --rust-analyzer rust-analyzer
```

Use an executable path instead of `rust-analyzer` if it is not on `PATH`. Open a function
and follow a call through a field or imported name. A unique admitted definition opens
directly; multiple semantic destinations still require a choice. The first lookup can
take longer while the server prepares its project model.

The provider reads Cargo metadata in a private captured-source mirror. Build scripts,
procedural macros and check-on-save builds are disabled. Features requiring those may
remain unresolved; this is not a full compiler build of the inspected project.

## Follow dependency source

Follow a call whose definition belongs to a locally available dependency. If the server
can resolve it and Vizzi can capture the declaration, it opens beside its caller with a
dependency/package label. Cached registry or Git sources, local path dependencies and
installed Rust standard-library source can be available this way.

Only reached files are captured. Dependency trees are not added to repository search
or scanned wholesale. Missing source or unsupported declaration syntax remains unavailable.
Caller/reference results include captured repository source and already admitted dependency
files; they are not a complete search of every dependency.

Dependency source has its own captured version. Disk/configuration changes can require
reopening it from a current caller. [Live and pinned views](follow-live-changes.md) and
the [language reference](../reference/languages.md) describe the limits.
