# Follow a Rust call trail

This Rust lesson builds a small reading workspace around a greeting function. You will follow
two calls, keep their shared caller visible, and save the arrangement as a task.
Allow about ten minutes after [building Vizzi](../how-to/install.md). The
[example source](first-call-trail-rust/src/lib.rs) is a small Cargo library.

## 1. Open the example

From your Vizzi checkout, copy the included example into a fresh temporary directory:

```sh
vizzi_demo=$(mktemp -d)
cp -R docs/tutorials/first-call-trail-rust/. "$vizzi_demo/"
./target/release/vizzi \
  --root "$vizzi_demo" \
  --workers 1
```

Open the printed address in your browser. Wait for **Source ready** in the Call workspace.
You do not need to build the example or install rust-analyzer.

## 2. Open the caller

Choose **Add bubble**. Enter `Greeter::greet`, select **Find functions**, and choose the
matching function. A source bubble opens showing `greet`, which calls `prepare_name`
and `message`. Its source lines keep their original numbers.

## 3. Follow the first call

Click the highlighted `prepare_name` in the source and choose **Open definition**.
The `prepare_name` bubble opens beside `greet`, with an arrow connecting them. The
caller stays visible. Read how the helper trims surrounding whitespace.

Clicking a call opens its menu; Ctrl-click, or Cmd-click on macOS, follows it directly.
For this example Vizzi can identify the source-local `self` methods without a language
server.

## 4. Open a branch without losing the first step

Return to the `greet` bubble and open its call to `message`. Choose **Fit workspace**
to bring the caller and both helpers into view. Read the returned greeting in `message`
while keeping `prepare_name` available for comparison.

Hover over a helper bubble. Its originating call line and arrow are highlighted. Hover
over that call line in `greet` to highlight the helper instead.

## 5. Arrange the task

Drag bubble headers to move them. Choose **↕ Fit height** to shrink a bubble to its loaded
code section, and drag the background to pan. **Fit workspace** brings all bubbles into
view. Selecting a box in the bottom-right overview returns to that bubble.

Open **Workspace → Groups**. Name the group `Greeting`, select the three source bubbles,
and choose **Save group**. Collapse and expand the group; its members return to their
previous positions.

## 6. Leave and return

Choose **Workspace → Rename task**, enter `Read the greeting`, and save the name.
Choose **Workspace → New task** to open an empty task. Then open **Workspace → Task shelf**
and select `Read the greeting`. Your bubbles and group return.

The shelf belongs to this browser and server address. It saves the reading arrangement;
it is not an export of the repository or a permanent source snapshot.

You have followed two branches while keeping their shared caller in view, and returned
to that context without reconstructing the trail. Stop the server with Ctrl-C when done.

Next, [run Vizzi on your project](../how-to/run-workspace.md). Use the
[call-trail guide](../how-to/follow-a-call-trail.md) for callers and references, or
[manage tasks](../how-to/manage-tasks.md) for more layout controls.
