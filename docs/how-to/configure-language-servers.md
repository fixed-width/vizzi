# Configure semantic navigation

Use a language server when you want type-aware definition and reference lookup rather
than written-name candidates. Vizzi supports rust-analyzer for Rust, Pyright for Python,
Ruby LSP for Ruby, clangd for C/C++, JDT LS for Java, and the native TypeScript
server for JavaScript and TypeScript. Several can be configured in one
mixed-language workspace.
Basic syntax browsing remains available without them.

To save these settings in TOML, see [save launch settings](configure-vizzi.md).
Vizzi-managed servers use captured-source mirrors. Java, JavaScript, and TypeScript
work through VS Code's shared provider.

## JavaScript and TypeScript with a managed server

Install the current stable TypeScript release and select its `tsc` executable.
TypeScript 7 adds a native language server, which Vizzi starts with `--lsp --stdio`.
The workflow was verified with TypeScript 7.0.2.
[TypeScript 7 release announcement](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/)

```sh
npm install -g typescript
vizzi --root /path/to/project --typescript-server tsc
```

With the server configured, open a function and follow a call or choose **Find
references**. Vizzi uses captured `package.json`, `tsconfig.json`, `jsconfig.json`
and variant files such as `tsconfig.base.json`. Keep `extends` and project references
relative to captured configs. Path-valued settings such as `include` and `paths`
must resolve within the mirror. Workspaces without a root config get a default
project that includes both JavaScript and TypeScript.

The managed provider disables automatic type acquisition and compiler plugins.
It reopens its captured project after a relevant source or config change.
Definitions in installed packages can open dependency bubbles from local
`node_modules`. TypeScript may supply a declaration file. Whether it includes
untyped JavaScript package implementations follows the project's `allowJs` and
`maxNodeModuleJsDepth` settings. The default project includes JavaScript at depth
one. Other outside targets remain outside. If the server or project settings
are unavailable, the call workspace retains written-name evidence.

## JavaScript and TypeScript with VS Code

In a trusted local folder with the Vizzi extension installed, choose **Vizzi: Open
Workspace**. This starts provider sharing. Use **Vizzi: Explore Call** on a saved
call to open the caller and follow the definition supplied by VS Code's JavaScript
and TypeScript service. **Find references** can include uses beyond written calls.
The editor's active project configuration determines its imports, types and aliases.
[Install and use the Vizzi extension](../../editors/vscode/INSTALL.md) ·
[VS Code TypeScript navigation](https://code.visualstudio.com/docs/typescript/typescript-editing#_code-navigation)

Save workspace buffers before looking up definitions or references. Vizzi verifies
locations against captured source and can open provider-reached source from the
workspace's installed packages.

## Java with a managed server

Install Eclipse JDT LS and a JDK supported by that release. Select the JDK with
`JAVA_HOME` or the launcher's normal runtime selection. Vizzi was verified on
Linux with JDT LS 1.63.0 and Java 25. Use the distribution's `bin/jdtls` launcher.
[Eclipse JDT LS downloads and launcher instructions](https://github.com/eclipse-jdtls/eclipse.jdt.ls#installation)

### Use a Maven or Gradle classpath

Open the build root with `vizzi --jdtls jdtls`. In the call workspace's
**Workspace** menu, choose **Import Maven classpath** or **Import Gradle classpath**.
This action runs build configuration using captured project descriptors and the
local plugin and dependency caches. Gradle import reuses the exact distribution
already cached for the project's Wrapper. For a build without a Wrapper, it
uses `GRADLE_HOME` or `PATH`. A configured `org.gradle.java.home` selects the
Java runtime for Gradle. Maven reports missing cached dependencies as an incomplete import.

Import again after changing a build file or installing new dependencies. Java
source edits use the imported session directly. Select a root with a single build
system. For builds that require extra scripts or plugins outside the captured
descriptors, use VS Code sharing.

### Explore a source folder directly

```sh
vizzi --root /path/to/project --jdtls jdtls \
  --java-source-root src/main/java \
  --java-source-root src/test/java
```

Each source root is relative to the selected workspace. Omit the flags for
sources whose package paths start at the workspace root. The equivalent TOML
keys under `[language-servers]` are `jdtls` and `java-source-root`, an array of paths.
The managed profile resolves definitions and references across these roots using
captured source and the JDK. Java files outside the roots remain browsable and
use written-name candidates. In source-root mode, JDT LS starts on the first
lookup with project import and automatic builds disabled. A saved change to
Java source under a configured root starts a fresh server on
the next lookup. JDT LS can check Gradle version metadata during startup.
Navigation also works with the network unavailable.

### Open Java library source

Install the JDK's `src.zip` to follow definitions into the standard library.
Maven and Gradle projects use the imported classpath for library resolution. For
an unmanaged folder, a local binary JAR and its matching `-sources.jar` can be
configured directly.

```sh
vizzi --jdtls jdtls --java-source-root src/main/java \
  --java-library lib/example.jar
```

Repeat `--java-library` for several JARs. Flag paths are relative to the workspace
root. The TOML key `java-library` is an array under `[language-servers]` with paths
relative to the config file. Use paths without glob characters. Vizzi opens
the matching source entry, including when the archive has a source-root prefix.
The dependency bubble can reopen from the same archive after a restart. With
the classpath available, Vizzi can resume navigation by replaying an unchanged
caller trail. Follow the call again if its source has changed or a capture in
the trail is no longer available.

## Java with VS Code

Open a trusted local folder with the Vizzi extension and a Java extension that
provides definitions and references. Let the Java extension finish loading the
project and save your workspace files. Run **Vizzi: Open Workspace** to share the
editor's language providers.

Right-click a saved Java call name and choose **Vizzi: Explore Call**, or follow a
call from a method bubble. Use **Find references** on a Java declaration to query
the shared provider. The editor's project configuration determines which overloads,
imports, and references it resolves. Vizzi verifies results against captured
workspace files. [Install and use the Vizzi extension](../../editors/vscode/INSTALL.md)

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

## Ruby with Ruby LSP

Install [Ruby LSP](https://shopify.github.io/ruby-lsp/editors.html) in the Ruby environment
you want to use, outside Vizzi:

```sh
gem install ruby-lsp
./target/release/vizzi \
  --root /path/to/ruby-project \
  --workers 1 \
  --ruby-lsp ruby-lsp
```

Use the installed RubyGems executable, or its full path for a specific Ruby version.
The option accepts an executable, not a `bundle exec` command or project binstub. Ruby
and the server are only needed for semantic navigation; syntax browsing works without them.

Vizzi gives the server a private copy of captured source and synchronises saved changes.
Known receiver definitions open directly. Ruby LSP's method reference results are checked
against definitions to discard unrelated same-name methods and declarations. Ambiguous
receiver references can be omitted; an empty result does not prove that a method is unused.

The adapter skips bundle composition, installation, project add-ons, formatters and linters.
It does not evaluate the project's Gemfile or gemspecs. Registry gem names from
`Gemfile.lock` help the server index gems already visible in the selected RubyGems
environment. `GEM_HOME` and `GEM_PATH` remain available for choosing that environment.
This does not reproduce complete Bundler resolution; path/Git gems and custom load paths
can remain unavailable. When multiple installed versions are indexed, distinct source
bodies retain a choice; gem capture labels include their source directory. Install missing dependencies and choose the Ruby environment
outside Vizzi. Exclude large vendor trees with `--exclude-source vendor/bundle` as needed.

Local Ruby source reached by the server uses independent dependency captures, including
available standard-library implementations. Ruby bodies take precedence over accompanying
RBS signatures; RBS-only targets, C implementations and generated DSL/alias declarations
without an admitted source body can remain unavailable. ERB and RBS analysis are outside
this initial support. Static lookup does not establish Ruby runtime dispatch.

The checked server is Ruby LSP 0.26.11 with Ruby 3.4.1 on Linux. A small server bootstrap
is specific to Ruby LSP; other Ruby language servers are not interchangeable with this option.

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
