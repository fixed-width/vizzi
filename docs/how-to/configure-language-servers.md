# Configure semantic navigation

Use a language server when you want type-aware definition and reference lookup rather
than written-name candidates. Vizzi supports rust-analyzer for Rust, Pyright for Python,
and clangd for C/C++. Several can be configured in one mixed-language workspace.
Basic syntax browsing remains available without them.

## Python with Pyright

Install Pyright outside Vizzi using its [installation instructions](https://github.com/microsoft/pyright#installation).
For a Node-based installation:

```sh
npm install --global pyright
```

The executable used by Vizzi is `pyright-langserver`. Pyright needs its own runtime;
Node.js is not a requirement for basic Vizzi browsing.

```sh
./target/release/vizzi \
  --root /path/to/python-project \
  --workers 1 \
  --pyright pyright-langserver
```

Add `--python-path /path/to/project/.venv/bin/python` to select a project interpreter.
Vizzi freezes `pyrightconfig.json` or `[tool.pyright]` settings from `pyproject.toml`,
including inherited JSON configuration. The server uses captured project files;
imports, typed receivers and aliases can resolve before a name chooser is offered.

Open a function and follow a call through an object field. A unique admitted semantic
target opens directly. **Find references** can now include non-call uses, while **Find
callers** filters to written call sites. Dynamic/unknown types can still have multiple
or unavailable destinations: server results are static analysis, not a runtime trace.

Locally available `.py` and `.pyi` dependency definitions can open as independent captures.
A stub can describe a declaration rather than its runtime implementation. Vizzi does not
install missing Python packages. Keep virtual environments out of the source reading
scope with `--exclude-source .venv`; selecting the interpreter still lets the server
use that environment's installed packages.

## Rust with rust-analyzer

Use `--rust-analyzer rust-analyzer`. The existing option and workflow remain supported.
The [Rust guide](configure-rust.md) covers Cargo inputs, standard-library source and
local dependencies.

## C/C++ with clangd

Install [clangd](https://clangd.llvm.org/installation) and prepare the project's
[compilation commands](https://clangd.llvm.org/design/compile-commands) outside Vizzi.
Vizzi does not run a build/configuration command to generate them.

For a C++ project, for example:

```sh
./target/release/vizzi \
  --root /path/to/cpp-project \
  --workers 1 \
  --c-family-root . \
  --syntax-language include=cpp \
  --clangd clangd \
  --compile-commands /path/to/build/compile_commands.json
```

Select the correct raw syntax for ambiguous headers using `--syntax-language`; use
`.=cpp` for an entirely C++ source scope, or more specific C/C++ assignments in a mixed
project. A returned definition still needs an admitted declaration to open.

Without `--compile-commands`, the adapter looks for `compile_commands.json` at the root,
then `compile_flags.txt`. Missing context is reported rather than running tooling.
Compile arguments, include paths and source paths are translated into the captured
mirror. Plugin/response-file flags are not supported by this initial adapter. Clangd
project config is disabled; its captured compilation data supplies the parse context.

Ordinary written calls, method declarations and definitions are available to navigation.
Implicit constructor/operator calls, generated code and unsupported syntax can remain
unavailable. A language server can return a header declaration when an implementation
is not known. Local external source is captured only when reached by the provider.

## Mixed repositories and updates

Pass the applicable server options together. Routing uses the captured source language,
not the terminal function name. Each provider starts lazily and stays warm; C and C++
share their clangd session. Indexing remains independent of semantic requests.

Saved source changes synchronize the private mirror. Configuration changes are checked
on lookup and restart the relevant session. Dependency directories and external build
configuration are not watched wholesale; refresh/reopen to recheck them. Missing servers
or unsupported analysis retain explicitly weaker candidates rather than pretending to
be type-aware. See [language coverage](../reference/languages.md) and
[troubleshooting](troubleshoot.md).
