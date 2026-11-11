# Follow a C/C++ call trail

In this lesson you will trace a C++ function into C helpers, keep that chain visible,
and open a second branch beside it. Allow about ten minutes after
[building Vizzi](../how-to/install.md). The included example has a
[C++ processor](first-call-trail-cpp/processor.cpp) and [C helpers](first-call-trail-cpp/signal.c).
You do not need to build the example or install clangd.

## 1. Open the example

From your Vizzi checkout, copy the included files into a fresh temporary directory:

```sh
vizzi_demo=$(mktemp -d)
cp -R docs/tutorials/first-call-trail-cpp/. "$vizzi_demo/"
./target/release/vizzi \
  --root "$vizzi_demo" \
  --c-family-root . \
  --workers 1
```

Open the printed address in your browser. Wait for **Source ready** in the Call workspace.
The `--c-family-root .` option includes C/C++ source in this example's reading scope.
Vizzi reads the files; it does not compile or run them.

## 2. Open the C++ caller

Choose **Add bubble**, enter `process_sample`, select **Find functions**, and choose the
matching function in `processor.cpp`. Its source bubble shows two calls:
`prepare_sample` limits the incoming sample, then `apply_gain` halves it.

## 3. Follow the chain into C

Click the highlighted `prepare_sample` and choose **Open definition**. Its bubble opens
beside `process_sample`, showing the function from `signal.c`.

In that new bubble, open its call to `clamp_sample`. A third bubble opens. Read its two
checks: values above `1.0` become `1.0`, values below `-1.0` become `-1.0`, and other
values pass through. The caller and its C helper stay visible as you reach this step.

This example has one recorded definition for each call, so each opens directly.
These links are written-name candidates; the lesson runs without a language server.

## 4. Open the C++ branch

Choose **Fit workspace** to bring the current chain into view. Return to the
`process_sample` bubble and open its `apply_gain` call. The fourth bubble
shows the multiplication in `processor.cpp`. Choose **Fit workspace** to bring all four
bubbles into view.

Follow an input of `1.5` through the source: `clamp_sample` limits it to `1.0`,
`prepare_sample` returns it, and `apply_gain` multiplies it by `0.5`. Keep both checks
and the multiplication visible as you read back through their caller.

Hover over a helper bubble to highlight its originating call line and arrow. Hover
over the corresponding call line to highlight the opened helper instead.
Ctrl-click follows a call directly; Cmd-click provides the same shortcut on macOS.

## 5. Arrange the task

Drag bubble headers to move them. Choose **↕ Fit height** to fit a bubble to its loaded
code, and drag the background to pan. **Fit workspace** brings the chain and branch
back into view.

Open **Workspace → Groups**. Name the group `Signal path`, select all four bubbles,
and choose **Save group**. Collapse and expand the group; the bubbles keep their positions.

## 6. Leave and return

Choose **Workspace → Rename task**, enter `Read sample processing`, and save the name.
Choose **Workspace → New task** to open an empty task. Open **Workspace → Task shelf**
and select `Read sample processing`. Your four bubbles and their group return.

The shelf saves the arrangement in this browser at this server address. It does not
save a permanent source snapshot. Stop the server with Ctrl-C when finished.

Next, [run Vizzi on your project](../how-to/run-workspace.md) or
[configure clangd](../how-to/configure-language-servers.md#cc-with-clangd) for type-aware
navigation with compilation context. The [language reference](../reference/languages.md#cc)
describes C/C++ source scopes, headers and current analysis limits.
