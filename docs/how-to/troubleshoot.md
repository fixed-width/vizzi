# Troubleshoot a first run

Use this guide when the [installation](install.md) or first reading session differs
from what you expect.

## The build fails before Vizzi starts

Check `rustc --version`, `cargo --version` and `cc --version`. Old Rust installations
can lack the edition/dependency support needed by the lockfile; use a current stable
toolchain. A missing C compiler or linker can fail the Tree-sitter parser build.

Run `cargo build --locked --release -p vizzi` from the Vizzi checkout. The first build
needs access to its build dependencies; `--offline` only works after they are cached.
Node.js is not required. See the [build guide](install.md) for prerequisites.

## The server rejects a path or port

The cache must be outside `--root`, even when paths use symlinks. A persistent
`--investigation-file` must be outside both root and cache. Use separate state and cache
directories rather than putting saved locations inside disposable cache data.

Both launch paths have defaults: `--root` uses the current directory and `--cache`
uses a platform cache directory for that root. If that automatic cache falls inside
the source scope, choose an external directory with `--cache`. The
[storage reference](../reference/storage.md#cache-location) lists the platform paths.

If the port is occupied, choose another with `--port 7475` and open the printed address.
Changing the address/port also changes the browser storage used for tasks and preferences.
Return to the original address to recover its saved tasks.

## Source is ready but a file or function is missing

Check the root and [language coverage](../reference/languages.md). Rust/Python are
selected by default; C/C++ needs explicit scopes. Exclusions, unsupported extensions,
source-file symlinks, parser recovery and deferred analysis can remove or limit results.
Macro-generated functions may not have a captured declaration to open.

In the Source browser, clear path filters or expand the relevant directory. In the call
workspace, search by a function's qualified name. Reading a repository does not require
it to build, but that does not make every language construct available.

If virtual environments, generated code or vendored libraries add irrelevant Python
matches, exclude their directories with repeatable flags such as `--exclude-source .venv`.
This changes the reading scope, not the repository.

## Calls offer unrelated choices or cannot open library source

Name candidates are not type resolution. This is a fallback when semantic analysis is not configured or is unavailable. Check the containing declaration and path before
choosing. Missing targets are not proof that a call is absent.

[Configure the language provider](configure-language-servers.md) for the project.
For Rust, enable rust-analyzer and check that the root is a Cargo
project/workspace. Its toolchain, dependencies and any standard-library source must be
available locally. The provider does not execute build scripts/procedural macros, so
some generated or configuration-dependent targets remain unavailable.

## Results say the repository changed when the function did not

Caller/reference results depend on the captured repository. Another file can add a
caller, and configuration can change analysis. Check for other disk activity and choose
**Refresh** on the result stack. Unchanged periodic checks do not themselves invalidate it.

## A task is missing or old source is unavailable

Use the same browser profile, hostname and port as before. Visual tasks live in browser
storage, not in the source checkout or the saved-location file. Clearing site data can
remove them. Task-save errors mean that the intended write did not succeed.

An old layout can outlive its source capture. Reopen the current function with **Add
bubble** when an earlier capture has expired or the declaration cannot be recovered.
Pinning does not permanently archive source. See [storage and persistence](../reference/storage.md).

## Source updates are disconnected

Check the terminal running Vizzi. The browser retains its layout and retries a lost
connection. Restart the server if needed and reload at the same address. Address any
storage-save error before relying on reload to retain an unsaved arrangement.

## Editor handoff fails

Check `command -v code`, or configure an executable with `--editor-code /path/to/code`.
Vizzi must run on the desktop where the editor should open. Changed or unverifiable
working bytes can refuse the handoff; it cannot open a historical or unsaved editor
buffer. [The editor guide](investigate-with-vscode.md) covers the supported workflow.

## A review revision cannot be resolved

Use the Git repository root as `--root`. Revision inputs accept local Git expressions;
Repository path accepts a relative file/directory, not a revision. A shallow checkout
can lack requested parents or objects. Choose available commits or obtain the required
history outside Vizzi; capture does not fetch it for you.

## Report a reproducible problem

Include the Vizzi source revision if known, OS/browser, launch options with private paths
redacted, language, exact message, and steps to reproduce. A small example that reproduces
the behaviour is more useful than a private repository dump. State whether it involves
unsaved buffers, saved files, a pinned view or a review capture.
