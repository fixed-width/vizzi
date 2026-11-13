# Vizzi for VS Code

Install the bundled extension, open your project, and choose **Vizzi: Open Workspace**.
The extension starts Vizzi and opens its visual workspace in an editor tab, using VS
Code's existing definition and reference providers. The extension creates or refreshes
its connection and passes it directly to Vizzi on every launch; you never need to copy
a `vscode-connection` path into project settings.

## Install the bundled preview

Use the 0.4.1 VSIX for your operating system and processor:

| System | Processor | VSIX filename |
| --- | --- | --- |
| Linux | x86-64 | `vizzi-vscode-0.4.1-linux-x64.vsix` |
| Linux | ARM64 | `vizzi-vscode-0.4.1-linux-arm64.vsix` |
| macOS | Intel | `vizzi-vscode-0.4.1-darwin-x64.vsix` |
| macOS | Apple Silicon | `vizzi-vscode-0.4.1-darwin-arm64.vsix` |

In VS Code's Extensions view, open **⋯ → Install from VSIX…** and select the file.
If VS Code offers to reload extensions after updating, accept it. The VSIX includes
Vizzi's native executable and browser interface. Rust, Node.js and a separate native
installation are not required to use the bundled preview.

The earlier 0.1.0 extension only shares language providers. Update to the bundled
0.4.1 preview to use the launch command. The release workflow builds these VSIX files
alongside native archives; local source builds are described below.

## Open your project

1. **Open a project folder in VS Code**, trust the workspace, and enable your usual
   language extension. Save your files: Vizzi reads saved source from disk.
2. **Click Vizzi in the status bar**, or open the Command Palette with **Ctrl+Shift+P**
   (**Cmd+Shift+P** on macOS) and run **Vizzi: Open Workspace**. If several folders are
   open, select the one to inspect. Output opens immediately with startup information.
3. **Use the Vizzi editor tab that opens.** Wait for **Source ready**, choose **Add bubble**,
   enter a function name, select **Find functions**, and choose a result. Click a
   highlighted call and choose **Open definition** to open the called function beside
   it. Use a bubble's **⋯ → Find references** to find its references through VS Code.

Keep VS Code open while using the workspace. Opening the same project again reuses its
running Vizzi process and reveals its tab. You can move the tab into another editor
group to read it beside your source. Closing the tab leaves the server running;
**Vizzi: Stop Workspace** closes the tab, process and sharing. Closing the VS Code
window also stops the process it launched.

Use **Vizzi: Open Workspace in Browser** for an external browser. Saved tasks in that
browser use separate storage from the VS Code tab.

## Explore a call from your editor

Save the source file, right-click a call name and choose **Vizzi: Explore Call**.
Vizzi starts automatically if needed and opens its Call workspace tab. The enclosing
code and selected definition appear together in the current task. If several targets
are possible, choose one in Vizzi's normal destination list. Earlier bubbles and groups
stay in the task; using the action again reveals existing views.

You can also place the cursor on a call name and run **Vizzi: Explore Call** from the
Command Palette. The source must be a captured project file. Unsaved files, unsupported
syntax, excluded source and positions without a captured call report an error rather
than selecting a different same-name function. Rust calls can lead into locally available
dependency and standard-library source as described below.

## Optional project settings

For source scopes, exclusions or a different port, save `vizzi.toml` **inside your
project directory**. For example:

```toml
root = "."
port = 7474
exclude-source = [".venv", "vendor"]
```

For a C/C++ project, also add `c-family-root = ["."]`. Mixed C/C++ projects may need
[header syntax assignments](https://github.com/fixed-width/vizzi/blob/main/docs/how-to/configure-language-servers.md#cc-with-clangd).

**Open Workspace** reads this file at launch and supplies the selected project root
and editor connection. Leave `[language-servers]` out when using shared editor analysis;
VS Code supplies the server configuration. After changing settings, run **Stop Workspace**
and then **Open Workspace** again. The
[configuration reference](https://github.com/fixed-width/vizzi/blob/main/docs/reference/configuration.md)
lists all options. Independently launched CLI sessions still select settings explicitly
with `vizzi --config vizzi.toml`.

Your TOML file can live in the project. The extension keeps its generated connection
JSON, which contains a session credential, in VS Code's extension storage.

## If opening fails

Open **View → Output** and select **Vizzi** for startup details. Errors also appear as
VS Code notifications.

- **Open Workspace is missing:** check that the bundled 0.4.1 extension is installed
  and enabled in this window, and reload extensions if VS Code requests it.
- **No folder is open:** use **File → Open Folder…** and try again. This preview requires
  a trusted local folder; remote and virtual workspaces are unsupported.
- **The executable is missing:** reinstall the bundled VSIX matching your platform.
  Extension developers can override it through **Vizzi: Executable Path** in Settings.
- **Address already in use:** set `port = 7475` in your project's `vizzi.toml` and reopen
  the workspace. Keep a stable port when returning to saved browser tasks.
- **The browser does not open:** use the workspace address printed in Vizzi Output.
- **A function is missing:** check the selected project and source scopes. C/C++ needs
  `c-family-root`. See the
  [language reference](https://github.com/fixed-width/vizzi/blob/main/docs/reference/languages.md).
- **Definitions or references are unavailable:** save changed files, check VS Code's
  own navigation, and let Vizzi refresh its source. Shared analysis requires captured
  source to match saved editor files. Refresh stale reference results.

Shared analysis supports captured repository source and locally installed Rust dependency
and standard-library source returned by VS Code. Following calls such as `ok_or_else`
opens the dependency method beside its caller. No extra server or setting is needed.
For syntax the native parser cannot recognise, the editor can supply a checked function
range; call highlighting inside that function can remain unavailable.

Older captured versions that differ from disk, and external dependency source in other
languages, need [Vizzi-managed servers](https://github.com/fixed-width/vizzi/blob/main/docs/how-to/configure-language-servers.md).

## Launch Vizzi independently

**Vizzi: Share Language Providers (Advanced)** remains available for an existing native Vizzi
installation launched separately from the plugin. Its optional `root` and
`[language-servers]` settings appear in Vizzi Output. For that advanced workflow, copy
those settings into your project's `vizzi.toml`, then run `vizzi --config vizzi.toml`.
Open the address printed in that terminal. **Vizzi: Stop Sharing Language Providers**
closes sharing while keeping the native process and VS Code's language servers running.
The connection file path is stable for the same project and VS Code profile; sharing
again refreshes its contents, so an independently launched Vizzi process can reconnect
without editing its settings. Use **Open Workspace** to manage the whole launch automatically.

## Build a VSIX from source

For contributors, with a stable Rust toolchain, C compiler/linker, Python 3, Node.js 22
or later and npm available, run from the Vizzi checkout:

```sh
cargo build --locked --release -p vizzi
python3 editors/vscode/package.py
```

The packager prints the platform-specific VSIX path under `dist/`. Install that file
using **Install from VSIX…**. The build uses Microsoft's VSCE 4.1.0 and includes the
native executable and project licences. Use `--binary PATH --target PLATFORM` when
packaging a release executable built for another supported target.
