# Use Vizzi with Neovim

Use Neovim 0.10 or newer on Linux or macOS. Keep a Vizzi checkout for the plugin
and [install or build the Vizzi executable](../../docs/how-to/install.md).

Add the plugin directory to your `init.lua` so Neovim loads its commands on startup.
If `vizzi` is on your `PATH`, you can omit the executable setting.

```lua
vim.opt.runtimepath:prepend('/path/to/vizzi/editors/neovim')
vim.g.vizzi_executable = '/path/to/vizzi/target/release/vizzi'
```

1. Restart Neovim and run `:VizziOpen` from the project directory. To select a
   directory explicitly, run `:VizziOpen /path/to/project`. Escape spaces in
   the path with a backslash.
2. In the browser, wait for **Source ready** and use **Add bubble** to open a
   function. Click **Editor** on the bubble to jump to its saved line in the
   Neovim split that launched Vizzi. Switch back to Neovim to see the selected
   line. If the target has unsaved changes, save it and retry once the browser
   has refreshed the source.
3. Run `:VizziStop` to stop the local server. Closing the launching split or
   quitting Neovim also stops it.

Run `:VizziOpen` again to reopen the browser at the same address. Each Neovim
instance owns one workspace. Opening a different project replaces it. If the
default browser does not open, copy the address from `:messages`.

## Configure native analysis

For project-aware definitions and references, create an explicit
[Vizzi TOML configuration](../../docs/how-to/configure-vizzi.md) with the
[managed language servers](../../docs/how-to/configure-language-servers.md) you
want. Add the configuration path to your `init.lua`.

```lua
vim.g.vizzi_config = '/path/to/vizzi.toml'
```

The directory selected by `:VizziOpen` takes precedence over the TOML root.
Without a TOML file, Vizzi selects an available port. With a TOML file, set
`port = 0` to allow concurrent Neovim sessions, or set a fixed port for one
session at a time. Saved browser layouts are scoped to the address, including
the port.
