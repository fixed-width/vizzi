import assert from "node:assert/strict";
import {spawnSync} from "node:child_process";
import {writeFile,rm} from "node:fs/promises";
import {join} from "node:path";
import {buildHost,browser,startHost,workspace,paint} from "./support.mjs";

if(!process.argv.includes("--no-build"))buildHost();
const input=await workspace({files:{"a.rs":"fn before() {}\nfn context() {}\n","reader.rs":"pub trait Reader { fn read(&self); }\n","removed.txt":"before\n"}});
const git=(...args)=>{const r=spawnSync("git",["-C",input.root,"-c","core.hooksPath=/dev/null","-c","user.name=Fixture","-c","user.email=fixture@example.invalid",...args],{encoding:"utf8",env:{...process.env,GIT_CONFIG_GLOBAL:"/dev/null",GIT_CONFIG_NOSYSTEM:"1"}});assert.equal(r.status,0,r.stderr);return r.stdout.trim();};
let chromium,host,context;
try {
  git("init","--quiet","--initial-branch=main","--template=");git("add",".");git("commit","--quiet","-m","base");
  await writeFile(join(input.root,"a.rs"),"fn after() {}\nfn context() {}\n");
  await writeFile(join(input.root,"notes.md"),"# Notes\n<img src=x onerror=window.injected=true>\n");
  await writeFile(join(input.root,"binary.bin"),Buffer.from([0,255]));await rm(join(input.root,"removed.txt"));
  host=await startHost(input,1);chromium=await browser({desktopBus:false});context=await chromium.newContext({viewport:{width:1440,height:1000}});const page=await context.newPage(),errors=[],windows=[];
  page.on("pageerror",error=>errors.push(error.message));
  page.on("response",response=>{const u=new URL(response.url());if(u.pathname.endsWith("/window") && response.status()===200)windows.push(response.json());});
  await page.goto(`${host.url}/review`);
  await page.getByRole("button",{name:"Compare with working tree",exact:true}).click();
  await page.waitForFunction(()=>document.querySelector("#review-workspace").dataset.token);
  const token=await page.locator("#review-workspace").getAttribute("data-token");
  const choose=async path=>{await page.locator("#review-file-list button").filter({hasText:path}).click();await page.waitForFunction(path=>document.querySelector("#review-file-title").textContent===path && document.querySelector("#review-navigation").getAttribute("aria-busy")==="false",path);await paint(page);};
  await choose("a.rs");
  assert.match(await page.locator("#base-source").textContent(),/before/);assert.match(await page.locator("#working-source").textContent(),/after/);
  assert.equal(await page.locator("#base-source .review-removed").count(),1);assert.equal(await page.locator("#working-source .review-added").count(),1);
  await page.locator("#working-outline button").filter({hasText:"after"}).click();
  await page.waitForFunction(()=>document.querySelector('#working-outline [aria-current="true"]'));
  assert.equal(await page.locator("#select-working").getAttribute("aria-pressed"),"true");
  await page.getByLabel("Include unchanged context").check();await page.getByLabel("Filter paths",{exact:true}).fill("reader.rs");
  await page.waitForFunction(()=>document.querySelector("#review-file-list").textContent.includes("reader.rs"));await choose("reader.rs");
  assert.match(await page.locator("#working-source").textContent(),/Reader/);
  await page.getByRole("button",{name:"Back",exact:true}).click();
  await page.waitForFunction(()=>document.querySelector("#review-file-title").textContent==="a.rs");
  assert.ok(await page.locator('#working-outline [aria-current="true"]').count());
  await page.goForward();await page.waitForFunction(()=>document.querySelector("#review-file-title").textContent==="reader.rs");
  await page.getByLabel("Filter paths",{exact:true}).fill("");await page.waitForFunction(()=>document.querySelector("#review-file-list").textContent.includes("removed.txt"));
  await choose("removed.txt");assert.match(await page.locator("#base-source").textContent(),/before/);assert.match(await page.locator("#working-evidence").textContent(),/Absent/);
  await choose("notes.md");assert.equal(await page.locator("#base-source .code-line").count(),0);assert.match(await page.locator("#working-source").textContent(),/<img/);assert.equal(await page.locator("#working-source img").count(),0);assert.equal(await page.evaluate(()=>window.injected),undefined);
  await choose("binary.bin");assert.match(await page.locator("#diff-note").textContent(),/unavailable/i);assert.match(await page.locator("#working-evidence").textContent(),/binary|unsupported/);
  await page.getByRole("button",{name:"Refresh working",exact:true}).click();await page.waitForFunction(token=>document.querySelector("#review-workspace").dataset.token!==token,token);
  const replacement=await page.locator("#review-workspace").getAttribute("data-token");
  await page.getByRole("button",{name:"Back",exact:true}).click();await page.waitForFunction(()=>document.querySelector("#review-notice").textContent.includes("expired"));assert.equal(await page.locator("#review-workspace").getAttribute("data-token"),replacement);
  await page.getByRole("button",{name:"Open current comparison",exact:true}).click();await page.waitForFunction(()=>document.querySelector("#review-notice").hidden);
  await page.setViewportSize({width:600,height:900});await paint(page);assert.ok(await page.locator("#select-base").isVisible());
  await page.getByRole("button",{name:"Close comparison",exact:true}).click();await page.waitForFunction(()=>document.querySelector("#review-workspace").dataset.token==="");
  await Promise.all(windows);for(const w of await Promise.all(windows)){assert.ok(w.window.lines.length<=96);assert.ok(w.window.outline.length<=200);assert.ok(["base","working"].includes(w.side));}
  assert.deepEqual(errors,[]);console.log("review browser: linked source, context, history, unavailable content, refresh expiry and narrow layout passed");
} finally {await context?.close();await chromium?.close();await host?.close();await input.close();}
