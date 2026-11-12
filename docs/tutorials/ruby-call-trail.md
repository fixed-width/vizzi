# Follow a Ruby call trail

In this lesson you will follow two calls in a small Ruby class, keep their shared
caller visible, and return to the arrangement as a saved task. Allow about ten minutes
after [building Vizzi](../how-to/install.md). You do not need a Ruby runtime, gems or
language server to inspect the [example program](first-call-trail-ruby/greeting.rb).

## 1. Open the example

From your Vizzi checkout, copy the included example into a fresh temporary directory:

```sh
vizzi_demo=$(mktemp -d)
cp -R docs/tutorials/first-call-trail-ruby/. "$vizzi_demo/"
./target/release/vizzi \
  --root "$vizzi_demo" \
  --workers 1
```

Open the printed address in your browser. Wait for **Source ready** in the Call workspace.
Ruby files are included by default. Vizzi reads `greeting.rb`; it does not run the program.

## 2. Open the caller

Choose **Add bubble**, enter `Greetings::Greeter#greet`, select **Find functions**, and
choose the matching method. A source bubble opens showing `greet`, which calls
`prepare_name` and then `message`.

The bubble's `Greetings` label identifies its declared Ruby module. Its source lines
keep their original numbers, even though you are reading one method at a time.

## 3. Follow the first call

Click the highlighted `prepare_name` and choose **Open definition**. Its bubble opens
beside `greet`, with an arrow connecting them. Read how `name.strip` removes surrounding
whitespace while keeping the caller visible.

The example has one recorded definition named `prepare_name`, so it opens directly.
This lesson runs without a language server: its links are written-name candidates,
not proof of runtime dispatch. The built-in `strip` method has no body in this example.

## 4. Open the second branch

Return to `greet` and open its `message` call. Choose **Fit workspace** to bring the
caller and both helpers into view. Read the interpolated greeting in `message`, then
look back at how the name was prepared. Ruby returns the last expression of each method.

All three bubbles share the `Greetings` module label and accent. Hover over a helper
bubble to highlight its originating call line and arrow. Hover over that line in
`greet` to highlight the opened helper instead.

Ctrl-click follows a highlighted call directly; Cmd-click provides the same shortcut
on macOS. You can also Tab to a call and use Ctrl/Cmd+Enter.

## 5. Arrange the workspace

Drag bubble headers to move them. Choose **↕ Fit height** to fit a bubble to its loaded
code. Drag the background to pan; **Fit workspace** brings the bubbles into view again.
The bottom-right overview can return you to an offscreen bubble.

Open **Workspace → Groups**. Name the group `Greeting`, select all three bubbles,
and choose **Save group**. Collapse and expand the group; its members keep their positions.

## 6. Leave and return

Choose **Workspace → Rename task**, enter `Read the greeting`, and save the name.
Choose **Workspace → New task** to open an empty task. Open **Workspace → Task shelf**
and select `Read the greeting`. Your three bubbles and their group return.

The shelf saves the arrangement in this browser at this server address. It does not
save a permanent copy of the Ruby source. Stop the server with Ctrl-C when finished.

Next, [run Vizzi on your project](../how-to/run-workspace.md),
[configure Ruby LSP](../how-to/configure-language-servers.md#ruby-with-ruby-lsp) for
semantic navigation and local dependency source, or
[find callers and references](../how-to/follow-a-call-trail.md).
