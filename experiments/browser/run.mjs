import { spawnSync } from "node:child_process";
import { resolve } from "node:path";

if (process.argv.includes("--dependencies")) {
 const result=spawnSync(process.execPath,["experiments/browser/dependency-workspace.test.mjs",...process.argv.slice(2).filter(a=>a!=="--dependencies")],{stdio:"inherit"});process.exit(result.status??1);
} else if (process.argv.includes("--semantic")) {
  const result=spawnSync(process.execPath,["experiments/browser/semantic-calls.test.mjs",...process.argv.slice(2).filter(arg=>arg!=="--semantic")],{stdio:"inherit"});process.exit(result.status ?? 1);
} else if (process.argv.includes("--visual")) {
  const result=spawnSync(process.execPath,["experiments/browser/visual-workspace.test.mjs",...process.argv.slice(2).filter(arg=>arg!=="--visual")],{stdio:"inherit"});process.exit(result.status ?? 1);
} else if (process.argv.includes("--discovery-measure")) {
  await import("../companion/measure-discovery.mjs");
} else if (process.argv.includes("--discovery")) {
  const result=spawnSync(process.execPath,["experiments/browser/discovery.test.mjs",...process.argv.slice(2).filter(arg=>arg!=="--discovery")],{stdio:"inherit"});process.exit(result.status ?? 1);
} else if (process.argv.includes("--companion-replay")) {
  await import("../companion/run.mjs");
} else if (process.argv.includes("--companion-measure")) {
  await import("../companion/measure.mjs");
} else if (process.argv.includes("--companion")) {
  const result=spawnSync(process.execPath,["experiments/browser/companion.test.mjs",...process.argv.slice(2).filter(arg=>arg!=="--companion")],{stdio:"inherit"});process.exit(result.status ?? 1);
} else if (process.argv.includes("--refinement")) {
  const result=spawnSync(process.execPath,["experiments/browser/refinement.test.mjs",...process.argv.slice(2).filter(arg=>arg!=="--refinement")],{stdio:"inherit"});process.exit(result.status ?? 1);
} else if (process.argv.includes("--orientation")) {
  const result=spawnSync(process.execPath,["experiments/browser/orientation.test.mjs",...process.argv.slice(2).filter(arg=>arg!=="--orientation")],{stdio:"inherit"});process.exit(result.status ?? 1);
} else if (process.argv.includes("--trial")) {
  const result=spawnSync(process.execPath,["experiments/browser/trial.test.mjs",...process.argv.slice(2).filter(arg=>arg!=="--trial")],{stdio:"inherit"});process.exit(result.status ?? 1);
} else if (process.argv.includes("--trial-mutations")) {
  const at=process.argv.indexOf("--output"),output=process.argv[at+1];
  if(at<0 || !output || output.startsWith("--"))throw new Error("--trial-mutations needs --output");
  const args=["experiments/browser/offline-mutations.mjs","experiments/browser/trial-stryker.config.json",output];
  const result=spawnSync(process.platform==="win32"?process.execPath:"nice",process.platform==="win32"?args:["-n","10",process.execPath,...args],{stdio:"inherit"});process.exit(result.status ?? 1);
} else if (process.argv.includes("--review-scale")) {
  await import("../review-workflow/measure.mjs");
} else if (process.argv.includes("--review-walkthrough")) {
  await import("../review-workflow/run.mjs");
} else if (process.argv.includes("--review")) {
  const result = spawnSync(process.execPath, ["experiments/browser/review.test.mjs",...process.argv.slice(2).filter(arg=>arg!=="--review")], { stdio: "inherit" });
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--startup")) {
  const result = spawnSync(process.execPath, ["experiments/browser/startup.test.mjs",...process.argv.slice(2).filter(arg=>arg!=="--startup")], { stdio: "inherit" });
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--review-refresh")) {
  const result = spawnSync(process.execPath, ["experiments/browser/review-refresh.test.mjs",...process.argv.slice(2).filter(arg=>arg!=="--review-refresh")], { stdio: "inherit" });
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--inspection")) {
  const result = spawnSync(process.execPath, ["experiments/browser/inspection.test.mjs"], { stdio: "inherit" });
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--navigation")) {
  const result = spawnSync(process.execPath, ["experiments/browser/navigation.test.mjs"], { stdio: "inherit" });
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--navigation-inspector")) {
  const result = spawnSync(process.execPath, ["experiments/browser/navigation-inspector.test.mjs"], { stdio: "inherit" });
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--navigation-inspector-mutations")) {
  const at=process.argv.indexOf("--config"),config=at<0?"experiments/browser/navigation-inspector-stryker.config.json":process.argv[at+1];
  if(!config || config.startsWith("--"))throw new Error("--config needs a Stryker configuration file");
  const args=["node_modules/@stryker-mutator/core/bin/stryker.js", "run", config];
  const result = spawnSync(process.platform==="win32"?process.execPath:"nice",process.platform==="win32"?args:["-n","10",process.execPath,...args], { stdio: "inherit" });
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--navigation-mutations")) {
  const result = spawnSync(process.execPath, ["node_modules/@stryker-mutator/core/bin/stryker.js", "run", "experiments/browser/navigation-stryker.config.json"], { stdio: "inherit" });
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--learning-walkthrough")) {
  await import("../learning-workflow/run.mjs");
} else if (process.argv.includes("--envelope-matrix")) {
  const args = process.argv.slice(2).filter(value => value !== "--envelope-matrix");
  const result = spawnSync("python3", ["-B", "experiments/memory-envelope/measure.py", ...args], { stdio: "inherit" });
  if (result.error) throw result.error;
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--envelope")) {
  const observe = process.argv.includes("--observe-only");
  const args = process.argv.slice(2).filter(value => !["--envelope", "--observe-only"].includes(value));
  const output = args[args.indexOf("--output") + 1];
  if (!args.includes("--output") || !output) throw new Error("--envelope needs --output");
  const result = spawnSync(resolve("target/release/vizzi-envelope"), [
    ...(observe ? ["--observe-only"] : []), output + ".memory.json", "--",
    process.execPath, resolve("experiments/browser/run.mjs"), "--measure", "--no-build", "--no-desktop-bus", ...args,
  ], { stdio: "inherit" });
  if (result.error) throw result.error;
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--install-browser")) {
  const result = spawnSync(process.execPath, ["node_modules/playwright/cli.js", "install", "chromium"], { stdio: "inherit" });
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--capabilities")) {
  const { browser } = await import("./support.mjs");
  const { readdir } = await import("node:fs/promises");
  const instance = await browser();
  try {
    const cdp = await instance.newBrowserCDPSession();
    const info = await cdp.send("SystemInfo.getInfo");
    console.log(JSON.stringify({ browser: instance.version(), deviceNodes: await readdir("/dev/dri").catch(() => []), display: process.env.DISPLAY ?? null, wayland: process.env.WAYLAND_DISPLAY ?? null, devices: info.gpu.devices, features: info.gpu.featureStatus }, null, 2));
  } finally { await instance.close(); }
} else if (process.argv.includes("--admission-mutations")) {
  const result = spawnSync(process.execPath, ["node_modules/@stryker-mutator/core/bin/stryker.js", "run", "experiments/browser/admission-stryker.config.json"], { stdio: "inherit" });
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--window-mutations")) {
  const result = spawnSync(process.execPath, ["node_modules/@stryker-mutator/core/bin/stryker.js", "run", "experiments/browser/window-stryker.config.json"], { stdio: "inherit" });
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--language-servers")) {
  const result=spawnSync(process.execPath,["experiments/browser/language-servers.test.mjs",...process.argv.slice(2).filter(arg=>arg!=="--language-servers")],{stdio:"inherit"});process.exit(result.status??1);
} else if (process.argv.includes("--outline-mutations")) {
  const result = spawnSync(process.execPath, ["node_modules/@stryker-mutator/core/bin/stryker.js", "run", "experiments/browser/outline-stryker.config.json"], { stdio: "inherit" });
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--mutations")) {
  const result = spawnSync(process.execPath, ["node_modules/@stryker-mutator/core/bin/stryker.js", "run", "experiments/browser/stryker.config.json"], { stdio: "inherit" });
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--measure")) {
  await import("./measure.mjs");
} else {
  for (const file of ["crates/vizzi/web/editor-call-links.test.js", "experiments/browser/app-header.test.mjs", "experiments/browser/header-navigation.test.mjs", "crates/vizzi/web/task-workspace.test.js", "crates/vizzi/web/task-shelf.test.js", "experiments/browser/working-sets.test.mjs", "crates/vizzi/web/dependency-model.test.js", "crates/vizzi/web/source-origin.test.js", "crates/vizzi/web/read-retry.test.js", "crates/vizzi/web/module-identity.test.js", "crates/vizzi/web/module-contours.test.js", "experiments/browser/module-contours-dom.test.mjs", "experiments/browser/module-outlines.test.mjs", "experiments/browser/module-identity.test.mjs","crates/vizzi/web/link-highlights.test.js", "experiments/browser/linked-hover.test.mjs","crates/vizzi/web/workspace-layout.test.js", "experiments/browser/workspace-controls.test.mjs","crates/vizzi/web/live-model.test.js", "crates/vizzi/web/live-updates.test.js", "experiments/browser/live-workspace.test.mjs", "crates/vizzi/web/source-margin.test.js", "experiments/browser/source-margin-dom.test.mjs", "experiments/browser/function-margin.test.mjs", "experiments/browser/direct-editor.test.mjs", "crates/vizzi/web/reflow.test.js", "experiments/browser/reflow-dom.test.mjs", "experiments/browser/reflow-workspace.test.mjs", "crates/vizzi/web/call-ranges.test.js", "experiments/browser/call-actions-dom.test.mjs", "experiments/browser/inline-calls.test.mjs", "crates/vizzi/web/visual-model.test.js", "crates/vizzi/web/definition-warmup.test.js", "experiments/browser/visual-workspace.test.mjs", "experiments/browser/fragment-search.test.mjs", "experiments/browser/visual-sections.test.mjs", "experiments/browser/self-calls.test.mjs", "experiments/browser/definition-selection.test.mjs", "experiments/browser/definition-warmup.test.mjs", "experiments/browser/structure.test.mjs", "experiments/companion/question-state.test.js", "experiments/browser/retired-question-ui.test.mjs", "experiments/browser/discovery.test.mjs", "experiments/browser/companion.test.mjs", "crates/vizzi/web/map-headings.test.js", "crates/vizzi/web/view-status.test.js", "experiments/browser/refinement.test.mjs", "crates/vizzi/web/map-scene.test.js", "experiments/browser/directory-controls.test.mjs", "experiments/browser/appearance.test.mjs", "experiments/browser/orientation.test.mjs", "crates/vizzi/web/presentation-dom.test.js", "crates/vizzi/web/presentation.test.js", "crates/vizzi/web/camera.test.js", "experiments/browser/trial.test.mjs", "crates/vizzi/web/startup.test.js", "crates/vizzi/web/controller.test.js", "crates/vizzi/web/navigation.test.js", "crates/vizzi/web/updates.test.js", "crates/vizzi/web/source-description.test.js", "crates/vizzi/web/review-controller.test.js", "crates/vizzi/web/review-windows.test.js", "experiments/review-workflow/walkthrough-audit.test.mjs", "experiments/browser/statistics.test.mjs", "crates/vizzi/web/windows.test.js", "experiments/browser/outline.test.mjs", "experiments/browser/inspection.test.mjs", "experiments/browser/navigation-inspector.test.mjs", "experiments/browser/startup.test.mjs", "experiments/browser/browser.test.mjs", "experiments/browser/navigation.test.mjs", "experiments/browser/review.test.mjs", "experiments/browser/review-refresh.test.mjs"]) {
    const result = spawnSync(process.execPath, [file], { stdio: "inherit" });
    if (result.status !== 0) process.exit(result.status ?? 1);
  }
}
