import { spawnSync } from "node:child_process";

if (process.argv.includes("--install-browser")) {
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
} else if (process.argv.includes("--mutations")) {
  const result = spawnSync(process.execPath, ["node_modules/@stryker-mutator/core/bin/stryker.js", "run", "experiments/browser/stryker.config.json"], { stdio: "inherit" });
  process.exit(result.status ?? 1);
} else if (process.argv.includes("--measure")) {
  await import("./measure.mjs");
} else {
  for (const file of ["crates/vizzi/web/controller.test.js", "experiments/browser/statistics.test.mjs", "experiments/browser/browser.test.mjs"]) {
    const result = spawnSync(process.execPath, [file], { stdio: "inherit" });
    if (result.status !== 0) process.exit(result.status ?? 1);
  }
}
