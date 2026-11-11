# Follow a Python call trail

In this lesson you will follow two calls in a small Python program, keep their shared
caller visible, and return to the arrangement as a saved task. Allow about ten minutes
after [building Vizzi](../how-to/install.md). You do not need a Python environment or
language server to inspect the [example program](first-call-trail-python/greeting.py).

## 1. Open the example

From your Vizzi checkout, copy the example into a fresh temporary directory:

```sh
vizzi_demo=$(mktemp -d)
cp -R docs/tutorials/first-call-trail-python/. "$vizzi_demo/"
./target/release/vizzi \
  --root "$vizzi_demo" \
  --cache "$HOME/.cache/vizzi/python-tutorial" \
  --workers 1
```

Open the printed address in your browser. Wait for **Source ready** in the Call workspace.
Vizzi reads `greeting.py`; it does not run the program or install its dependencies.

## 2. Open the caller

Choose **Add bubble**, enter `greet`, select **Find functions**, and choose the matching
function from `greeting.py`. A source bubble opens. It shows a call to `prepare_name`
followed by a call to `message`.

## 3. Follow a call

Click the highlighted `prepare_name` in the source, then choose **Open definition**.
Its bubble opens beside `greet`. Read the call to `strip()` while keeping the caller
visible. An arrow connects the two source bubbles.

The example has one recorded definition named `prepare_name`, so it opens directly.
This lesson starts without a language server, so navigation uses written call names.
[Configure Pyright](../how-to/configure-language-servers.md#python-with-pyright) for type-aware
navigation on a real Python project.
The candidate arrow preserves that distinction; it does not establish runtime dispatch.
Without the optional provider, built-in string methods are not resolved into library source.

## 4. Open the second branch

Return to `greet` and open its call to `message`. Choose **Fit workspace** to bring the
caller and both helpers into view. Read the greeting returned by `message`, then look
back at how the name was prepared. Neither earlier bubble has been replaced.

Hover over a helper bubble to highlight its originating call line and arrow. Hover over
the corresponding line in `greet` to highlight the opened helper.

Ctrl-click follows a highlighted call without opening its menu. Cmd-click provides the
same shortcut on macOS. You can also Tab to the call and use Ctrl/Cmd+Enter.

## 5. Arrange the workspace

Drag the bubble headers to move them. Choose **↕ Fit height** to fit a bubble to its loaded
code. Drag the background to pan; **Fit workspace** brings the bubbles into view again.
The bottom-right overview can return you to an offscreen bubble.

Open **Workspace → Groups**. Name a group `Greeting`, select all three source bubbles,
and choose **Save group**. Collapse and expand the group. The bubbles keep their positions.

## 6. Leave and return

Choose **Workspace → Rename task**, enter `Read the greeting`, and save the name.
Choose **Workspace → New task**. The canvas is empty, ready for another task.
Open **Workspace → Task shelf** and select `Read the greeting`; your earlier context returns.

The shelf saves the arrangement in this browser at this server address. It does not
save a permanent copy of the Python program. Stop the server with Ctrl-C when finished.

You have followed two branches without losing their caller and returned to a saved task.
Next, [run Vizzi on your project](../how-to/run-workspace.md),
[find callers and references](../how-to/follow-a-call-trail.md), or read
[how Python navigation differs from Rust](../reference/languages.md).
