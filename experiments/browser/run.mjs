import { spawnSync } from "node:child_process";
import { resolve } from "node:path";

if (process.argv.includes("--review-scale")) {
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
} else if (process.argv.includes("--outline-mutations")) {
  const result = spawnSync(process.execPath, ["node_modules/@stryker-mutator/core/bin/stryker.js", "run", "experiments/browser/outline-stryker.config.json"], { stdio: "inherit" });
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--mutations")) {
  const result = spawnSync(process.execPath, ["node_modules/@stryker-mutator/core/bin/stryker.js", "run", "experiments/browser/stryker.config.json"], { stdio: "inherit" });
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--measure")) {
  await import("./measure.mjs");
} else {
  for (const file of ["crates/vizzi/web/startup.test.js", "crates/vizzi/web/controller.test.js", "crates/vizzi/web/navigation.test.js", "crates/vizzi/web/updates.test.js", "crates/vizzi/web/source-description.test.js", "crates/vizzi/web/review-controller.test.js", "crates/vizzi/web/review-windows.test.js", "experiments/review-workflow/walkthrough-audit.test.mjs", "experiments/browser/statistics.test.mjs", "crates/vizzi/web/windows.test.js", "experiments/browser/outline.test.mjs", "experiments/browser/inspection.test.mjs", "experiments/browser/navigation-inspector.test.mjs", "experiments/browser/startup.test.mjs", "experiments/browser/browser.test.mjs", "experiments/browser/navigation.test.mjs", "experiments/browser/review.test.mjs", "experiments/browser/review-refresh.test.mjs"]) {
    const result = spawnSync(process.execPath, [file], { stdio: "inherit" });
    if (result.status !== 0) process.exit(result.status ?? 1);
  }
}
