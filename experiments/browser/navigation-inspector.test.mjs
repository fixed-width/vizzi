import { test, before, after } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { browser, paint } from "./support.mjs";
import { INSTRUMENTER_CONSTANTS as stryker } from "@stryker-mutator/api/core";
let chromium;
before(async()=>{chromium=await browser();});
after(async()=>{await chromium.close();});
async function fixture(initial="a") {
  const page=await chromium.newPage();
  page.setDefaultTimeout(5000);
  await page.addInitScript(({namespace,field,active,hitCount,hitLimit})=>{
    globalThis[namespace]={[field]:active,[hitCount]:0,[hitLimit]:1000000};
  },{namespace:stryker.NAMESPACE,field:stryker.ACTIVE_MUTANT,active:process.env[stryker.ACTIVE_MUTANT_ENV_VARIABLE],hitCount:stryker.HIT_COUNT,hitLimit:stryker.HIT_LIMIT});
  await page.route("http://fixture/**",async route=>{
    const path=new URL(route.request().url()).pathname;
    if(path==="/")return route.fulfill({contentType:"text/html",body:'<link rel="stylesheet" href="/app.css"><div style="display:flex"><div id="outline" style="height:240px;width:200px;flex:none"></div><div id="source" style="height:220px;width:420px"><div id="source-space"></div></div></div><button id="outside">Outside</button>'});
    return route.fulfill({contentType:path.endsWith(".css")?"text/css":"text/javascript",body:await readFile(resolve("crates/vizzi/web"+path),"utf8")});
  });
  await page.goto("http://fixture/");
  await page.evaluate(async initial=>{
    const {createInspector}=await import("/inspector.js");
    const definitions={a:{lines:500,declarations:50},b:{lines:20,declarations:2},c:{lines:500,declarations:50},long:{lines:1,declarations:0},otherlong:{lines:2,declarations:0},many:{lines:5000,declarations:500},wide:{lines:500,declarations:0}};
    const encoder=new TextEncoder(),decoder=new TextDecoder("utf-8",{fatal:true});
    const original=encoder.encode("HEAD"+"x".repeat(99992)+"TAIL"),different=encoder.encode("z".repeat(99996)+"NEWT");
    window.longSource=text=>{definitions.long.lines=2;window.longBytes=encoder.encode(text);};
    window.longWithContext=()=>{definitions.long.lines=2;};
    window.calls=[];window.errors=[];window.activations=[];window.adjustAnchor=false;
    window.meta=(file,version=file,snapshot="s",entity=null)=>{
      const summary={...definitions[file],active:entity===null?null:{id:file+"-e"+entity,index:entity,start_line:entity*8+1,end_line:entity*8+1}};
      return {snapshot,selection:{file,entity:summary.active?.id ?? null},source:{id:file,version,summary}};
    };
    window.widget=createInspector({source:document.getElementById("source"),space:document.getElementById("source-space"),outline:document.getElementById("outline"),select:row=>window.activations.push(row.id),onError:e=>window.errors.push(e.message),transport:window.transport=async uri=>{
      const q=new URL(uri,location.href).searchParams;window.calls.push(Object.fromEntries(q));
      if(q.get("previous_snapshot")===window.expiredPrevious)throw new Error("Expired previous snapshot");
      const file=q.get("file"),def=definitions[file],entry=index=>({index,analysis:0,id:file+"-e"+index,name:"item_"+index,kind:"function",name_truncated:false,kind_truncated:false,start_line:index*8+1,end_line:index*8+1});
      if(window.pauseTarget && file==="many" && q.get("cursor_index")==="499"){window.targetRequested=true;await window.targetGate;}
      const start=Number(q.get("line_start") ?? 1),lines=[];
      const count=Math.min(def.lines-start+1,Number(q.get("line_count") ?? 96)),allowance=Math.floor(32768/Math.max(1,count));
      for(let number=start;number<=Math.min(def.lines,start+Number(q.get("line_count") ?? 96)-1);number++) {
        const long=["long","otherlong"].includes(file) && number===1;
        const bytes=long?(file==="otherlong" || q.get("version")==="different-bytes"?different:window.longBytes ?? original):encoder.encode(`line ${file} ${number}`+(file==="wide"?"w".repeat(1000):""));
        const line_bytes=bytes.length,floor=offset=>{while(offset>0 && (bytes[offset]&192)===128)offset--;return offset;};
        const column=floor(Number(q.get("column_line"))===number?Math.min(line_bytes,Number(q.get("column") ?? 0)):0);
        const end_column=floor(Math.min(line_bytes,column+allowance));
        const text=decoder.decode(bytes.slice(column,end_column));
        lines.push({number,column,end_column,line_bytes,text,left:column>0,right:end_column<line_bytes});
      }
      let outline_start=Math.min(Number(q.get("outline_start") ?? 0),Math.max(0,def.declarations-1));
      if(window.adjustAnchor && q.has("previous_snapshot"))outline_start=Math.max(0,outline_start-1);
      const outline=Array.from({length:Math.min(Number(q.get("outline_count") ?? 200),def.declarations-outline_start)},(_,i)=>entry(outline_start+i));
      return {schema:1,snapshot:q.get("snapshot"),file,version:q.get("version"),line_total:def.lines,line_start:start,lines,
        outline_total:def.declarations,outline_start,outline,cursor:def.declarations?entry(Math.min(Number(q.get("cursor_index") ?? 0),def.declarations-1)):null};
    }});
    window.show=async(view,restore,reveal)=>{window.current=view;window.widget.apply(await window.widget.prepare(view,{restore,reveal}));};
    if(initial)await window.show(window.meta(initial));
  },initial);
  await paint(page);return page;
}
test("inspector restores source, outline and cursor with bounded windows; normal selection still reveals its declaration",async()=>{
  const page=await fixture();try{
    await page.locator("#source").evaluate(n=>{n.scrollTop=880;});
    await page.locator("#outline button").first().focus();await page.keyboard.press("End");await paint(page);
    const saved=await page.evaluate(()=>window.widget.position());
    assert.equal(saved.cursorIndex,49);
    await page.evaluate(()=>window.show(window.meta("c")));
    assert.equal(await page.locator("#outline").evaluate(n=>n.scrollTop),0);
    await page.evaluate(()=>window.show(window.meta("b")));
    assert.equal(await page.locator("#source").evaluate(n=>n.scrollTop),0);
    await page.evaluate(saved=>window.show(window.meta("a"),saved),saved);
    assert.deepEqual(await page.evaluate(()=>window.widget.position()),saved);
    assert.ok((await page.locator("#source").textContent()).includes("line a 41"));
    assert.ok(await page.locator(".code-line").count()<=96);assert.ok(await page.locator("#outline button").count()<=200);
    await page.evaluate(()=>window.show(window.meta("a","a","s",30)));
    assert.equal(await page.locator("#source").evaluate(n=>n.scrollTop),(241-4)*22);
    assert.equal(await page.evaluate(()=>window.widget.position().cursorIndex),30);
    assert.equal(await page.locator("#outline").evaluate(n=>{const b=n.querySelector('[aria-current="true"]').getBoundingClientRect(),r=n.getBoundingClientRect();return b.top>=r.top&&b.bottom<=r.bottom;}),true);
    await page.locator("#source").evaluate(n=>{n.scrollTop=8000;});
    await page.locator("#outline button[tabindex='0']").focus();await page.keyboard.press("End");await paint(page);
    const manual=await page.evaluate(()=>window.widget.position());
    await page.evaluate(()=>window.show(window.meta("b")));
    await page.evaluate(p=>window.show(window.meta("a","a","s",30),p),manual);
    assert.deepEqual(await page.evaluate(()=>window.widget.position()),manual);
    await page.evaluate(()=>{window.expiredPrevious="s";return window.show(window.meta("a","a","s2",30));});
    assert.equal(await page.locator("#source").evaluate(n=>n.scrollTop),8000);
    assert.equal(await page.evaluate(()=>window.widget.position().cursorIndex),manual.cursorIndex);
    assert.equal(await page.evaluate(()=>window.widget.position().outlineTop),manual.outlineTop);
    assert.deepEqual(await page.evaluate(()=>window.activations),[]);
    assert.deepEqual(await page.evaluate(()=>window.errors),[]);
  }finally{await page.close();}
});
test("long-line fragments and horizontal reading positions survive returns and unchanged refreshes, but not changed source",async()=>{
  const page=await fixture();try{
    await page.evaluate(()=>window.show(window.meta("long","shared")));
    await page.getByRole("button",{name:"Last text on line 1",exact:true}).click();
    await page.waitForFunction(()=>document.getElementById("source").textContent.includes("TAIL"));
    await page.locator("#source").evaluate(n=>{n.scrollLeft=150;});
    const saved=await page.evaluate(()=>window.widget.position());assert.ok(saved.column>=67232 && saved.column<=67235);assert.equal(saved.columnLine,1);
    await page.evaluate(()=>window.show(window.meta("b","shared")));
    assert.equal(await page.evaluate(()=>window.widget.position().columnLine),undefined);
    assert.ok((await page.locator("#source").textContent()).includes("line b 1"));
    await page.evaluate(saved=>window.show(window.meta("long","shared"),saved),saved);
    assert.equal(await page.evaluate(()=>window.widget.position().column),saved.column);
    assert.equal(await page.locator("#source").evaluate(n=>n.scrollLeft),150);
    assert.ok((await page.locator("#source").textContent()).includes("TAIL"));
    await page.evaluate(()=>window.show(window.meta("long","shared","s2")));
    assert.equal(await page.evaluate(()=>window.widget.position().column),saved.column);
    assert.equal(await page.evaluate(()=>window.widget.position().columnLine),saved.columnLine);
    assert.equal(await page.locator("#source").evaluate(n=>n.scrollLeft),150);
    assert.ok((await page.locator("#source").textContent()).includes("TAIL"));
    await page.evaluate(()=>window.show(window.meta("long","changed","s3")));
    assert.equal(await page.evaluate(()=>window.widget.position().columnLine),undefined);
    assert.equal(await page.evaluate(()=>window.widget.position().column),0);
    assert.equal(await page.locator("#source").evaluate(n=>n.scrollLeft),0);
    assert.ok(!(await page.locator("#source").textContent()).includes("TAIL"));
    assert.deepEqual(await page.evaluate(()=>window.errors),[]);
  }finally{await page.close();}
});
test("unchanged refresh retains an expanded fragment when the regular multi-line window has a smaller byte allowance",async()=>{
  const page=await fixture();try {
    await page.evaluate(()=>{window.longWithContext();return window.show(window.meta("long","shared"));});
    await page.getByRole("button",{name:"Last text on line 1",exact:true}).click();
    await page.waitForFunction(()=>document.querySelector("#source").textContent.includes("TAIL"));
    assert.ok((await page.locator("#source").textContent()).includes("line long 2"));
    const saved=await page.evaluate(()=>window.widget.position());assert.ok(saved.sourceLeft>0);
    await page.evaluate(()=>window.show(window.meta("long","shared","refreshed")));await paint(page);
    assert.equal(await page.locator("#source").evaluate(n=>n.scrollLeft),saved.sourceLeft);
    assert.ok((await page.locator("#source").textContent()).includes("TAIL"));
    assert.ok((await page.locator("#source").textContent()).includes("line long 2"));
    await page.evaluate(()=>window.show(window.meta("long","changed","new-bytes")));await paint(page);
    assert.equal(await page.locator("#source").evaluate(n=>n.scrollLeft),0);assert.equal((await page.locator("#source").textContent()).includes("TAIL"),false);
    assert.deepEqual(await page.evaluate(()=>window.errors),[]);
  } finally {await page.close();}
});
for(const tail of ["TAIL!","TAIL"])test(`UTF-8 end fragments include ${tail} and keep their actual byte range across unchanged refresh`,async()=>{
  const page=await fixture();try {
    await page.evaluate(tail=>{window.longSource("😺".repeat(25000)+tail);return window.show(window.meta("long","shared"));},tail);
    await page.getByRole("button",{name:"Last text on line 1",exact:true}).click();
    await page.waitForFunction(()=>document.querySelector("#source").textContent.includes("TAIL"));
    assert.ok((await page.locator("#source").textContent()).includes(tail));
    const before=await page.locator("#source .source-text").first().textContent();
    await page.evaluate(()=>window.show(window.meta("long","shared","refreshed")));await paint(page);
    assert.equal(await page.locator("#source .source-text").first().textContent(),before);assert.deepEqual(await page.evaluate(()=>window.errors),[]);
  } finally {await page.close();}
});

test("cached fragments never cross files, versions or requested byte columns, or truncate a larger fresh fragment",async()=>{
  const page=await fixture();try {
    await page.evaluate(()=>{window.longWithContext();return window.show(window.meta("long","shared"));});
    await page.getByRole("button",{name:"Last text on line 1",exact:true}).click();await page.waitForFunction(()=>document.querySelector("#source").textContent.includes("TAIL"));
    const last=await page.evaluate(()=>window.widget.position());
    await page.evaluate(p=>window.show(window.meta("otherlong","shared"),p),last);
    assert.equal((await page.locator("#source").textContent()).includes("TAIL"),false);assert.ok((await page.locator("#source").textContent()).includes("zzzz"));
    await page.evaluate(()=>window.show(window.meta("long","shared")));await page.getByRole("button",{name:"Last text on line 1",exact:true}).click();await page.waitForFunction(()=>document.querySelector("#source").textContent.includes("TAIL"));
    await page.evaluate(p=>window.show(window.meta("long","different-bytes"),p),last);assert.equal((await page.locator("#source").textContent()).includes("TAIL"),false);assert.ok((await page.locator("#source").textContent()).includes("zzzz"));
    await page.evaluate(()=>window.show(window.meta("long","shared")));await page.getByRole("button",{name:"Last text on line 1",exact:true}).click();await page.waitForFunction(()=>document.querySelector("#source").textContent.includes("TAIL"));
    await page.evaluate(p=>window.show(window.meta("long","shared"),{...p,column:0,sourceLeft:0}),last);assert.ok((await page.locator("#source").textContent()).includes("HEAD"));assert.equal((await page.locator("#source").textContent()).includes("TAIL"),false);
    const length=await page.evaluate(async()=>{
      const next=window.meta("long","shared"),body=await window.transport("/api/window?snapshot=s&file=long&version=shared&line_start=1&line_count=1&column_line=1&column=0");
      window.widget.apply({...next,inspection:body,inspectionPosition:{sourceTop:0,sourceLeft:0,outlineTop:0,columnLine:1,column:0}});
      return document.querySelector("#source .source-text").textContent.length;
    });assert.equal(length,32769);
    await page.evaluate(async()=>{const next=window.meta("long","shared"),body=await window.transport("/api/window?snapshot=s&file=long&version=shared&line_start=2&line_count=1");window.widget.apply({...next,inspection:body,inspectionPosition:{sourceTop:0,sourceLeft:0,outlineTop:0,columnLine:1,column:0}});});
    await page.evaluate(async()=>{const next=window.meta("otherlong","shared"),body=await window.transport("/api/window?snapshot=s&file=otherlong&version=shared&line_start=2&line_count=1");window.widget.apply({...next,inspection:body,inspectionPosition:{sourceTop:0,sourceLeft:0,outlineTop:0,column:0}});});
    assert.equal((await page.locator("#source").textContent()).includes("HEAD"),false);assert.equal((await page.locator("#source").textContent()).includes("TAIL"),false);
    assert.deepEqual(await page.evaluate(()=>window.errors),[]);
  } finally {await page.close();}
});

test("the last fragment retains the entire line when it fits the single-row byte budget",async()=>{
  const page=await fixture();try {
    for(const bytes of [32767,32768]) {
      await page.evaluate(n=>{window.longSource("HEAD"+"x".repeat(n-8)+"TAIL");return window.show(window.meta("long",`version-${n}`));},bytes);
      await page.getByRole("button",{name:"Last text on line 1",exact:true}).click();await page.waitForFunction(()=>document.querySelector("#source").textContent.includes("TAIL"));
      const text=await page.locator("#source .source-text").first().textContent();assert.ok(text.startsWith("HEAD"));assert.ok(text.endsWith("TAIL"));assert.equal(text.length,bytes);
    }
  } finally {await page.close();}
});

test("First, Later and Earlier navigate complete fragments without skipping backward or forcing horizontal end",async()=>{
  const page=await fixture();try {
    await page.evaluate(()=>{window.longWithContext();return window.show(window.meta("long","shared"));});
    await page.getByRole("button",{name:"Last text on line 1",exact:true}).click();await page.waitForFunction(()=>document.querySelector("#source").textContent.includes("TAIL"));
    await page.getByRole("button",{name:"First text on line 1",exact:true}).click();await page.waitForFunction(()=>document.querySelector("#source").textContent.includes("HEAD"));
    assert.equal(await page.evaluate(()=>window.widget.position().column),0);assert.equal(await page.locator("#source").evaluate(n=>n.scrollLeft),0);
    for(const column of [32768,65536]) {await page.getByRole("button",{name:"Later text on line 1",exact:true}).click();await page.waitForFunction(n=>window.widget.position().column===n,column);assert.equal(await page.locator("#source").evaluate(n=>n.scrollLeft),0);}
    for(const column of [32768,0]) {await page.getByRole("button",{name:"Earlier text on line 1",exact:true}).click();await page.waitForFunction(n=>window.widget.position().column===n,column);assert.equal(await page.locator("#source").evaluate(n=>n.scrollLeft),0);}
    assert.ok((await page.locator("#source").textContent()).includes("HEAD"));assert.deepEqual(await page.evaluate(()=>window.errors),[]);
  } finally {await page.close();}
});

test("ordinary source window loads do not force horizontal reading to the end of the line",async()=>{
  const page=await fixture("wide");try {
    await page.locator("#source").evaluate(n=>{n.scrollLeft=150;n.scrollTop=4400;});await page.waitForFunction(()=>[...document.querySelectorAll("#source .source-text")].some(n=>n.textContent.startsWith("line wide 201")));
    assert.equal(await page.locator("#source").evaluate(n=>n.scrollLeft<n.scrollWidth-n.clientWidth),true);assert.deepEqual(await page.evaluate(()=>window.errors),[]);
  } finally {await page.close();}
});

test("pending keyboard activation waits for the remote cursor, stays focused and does not repeat on later windows",async()=>{
  const page=await fixture("many");try {
    await page.evaluate(()=>{window.pauseTarget=true;window.targetGate=new Promise(r=>{window.releaseTarget=r;});});
    await page.locator("#outline button").first().focus();await page.keyboard.press("End");await page.waitForFunction(()=>window.targetRequested);await page.keyboard.press("Enter");assert.deepEqual(await page.evaluate(()=>window.activations),[]);
    await page.evaluate(()=>{window.pauseTarget=false;window.releaseTarget();});await page.waitForFunction(()=>window.activations.length===1);
    assert.deepEqual(await page.evaluate(()=>window.activations),["many-e499"]);assert.equal(await page.evaluate(()=>document.activeElement.dataset.declaration),"many-e499");
    await page.locator("#source").evaluate(n=>{n.scrollTop=2200;});
    try{await page.waitForFunction(()=>[...document.querySelectorAll("#source .source-text")].some(n=>n.textContent==="line many 101"));}catch(error){console.log("pending-cursor diagnostic",await page.evaluate(()=>({position:window.widget.position(),calls:window.calls.slice(-4),errors:window.errors,rows:[...document.querySelectorAll("#source .source-text")].slice(0,3).map(n=>n.textContent)})));throw error;}
    assert.equal(await page.evaluate(()=>window.activations.length),1);
    await page.locator("#outline").evaluate(n=>{n.scrollTop=0;});await paint(page);await page.waitForFunction(()=>document.querySelector('#outline button[data-index="0"]'));await page.keyboard.press("Enter");await paint(page);
    assert.deepEqual(await page.evaluate(()=>window.activations),["many-e499","many-e499"]);assert.deepEqual(await page.evaluate(()=>window.errors),[]);
  } finally {await page.close();}
});

test("revealActive fetches missing source and outline, avoids redundant reads and is safe before any source exists",async()=>{
  const page=await fixture(null);try {
    await page.evaluate(async()=>{await window.widget.revealActive();window.widget.apply({source:null,selection:{}});window.widget.apply({source:null,selection:{}});await window.widget.revealActive();});
    await page.evaluate(()=>window.show(window.meta("a","a","s",30)));
    await page.locator("#source").evaluate(n=>{n.scrollTop=0;});await page.locator("#outline").evaluate(n=>{n.scrollTop=0;});await paint(page);await page.waitForFunction(()=>[...document.querySelectorAll("#source .source-text")].some(n=>n.textContent==="line a 1"));
    const revealed=await page.evaluate(async()=>{await window.widget.revealActive();return {top:document.querySelector("#source").scrollTop,text:document.querySelector("#source").textContent,outline:document.querySelector("#outline").scrollTop};});
    assert.equal(revealed.top,237*22);assert.ok(revealed.text.includes("line a 241"));assert.ok(revealed.outline>0);
    const count=await page.evaluate(()=>window.calls.length);await page.evaluate(()=>window.widget.revealActive());assert.equal(await page.evaluate(()=>window.calls.length),count);
    await page.locator("#source").evaluate(n=>{n.scrollTop=4334;});await page.locator("#outline").evaluate(n=>{n.scrollTop=0;});await paint(page);await page.waitForFunction(()=>[...document.querySelectorAll("#source .source-text")].some(n=>n.textContent==="line a 198"));
    const cached=await page.evaluate(async()=>{const before=window.calls.length;await window.widget.revealActive();return{queries:window.calls.length-before,source:document.querySelector("#source").textContent,outline:document.querySelector("#outline").textContent};});
    assert.equal(cached.queries,0);assert.ok(cached.source.includes("line a 241"));assert.ok(cached.outline.includes("item_26"));
    await page.evaluate(()=>window.show(window.meta("b")));const before=await page.evaluate(()=>window.calls.length);await page.evaluate(()=>window.widget.revealActive());assert.equal(await page.evaluate(()=>window.calls.length),before);assert.deepEqual(await page.evaluate(()=>window.errors),[]);
  } finally {await page.close();}
});
test("restored positions clamp to the viewport; changed-version anchor adjustment and empty views remain coherent",async()=>{
  const page=await fixture();try{
    const prepared=await page.evaluate(()=>window.widget.prepare(window.meta("a"),{restore:{sourceTop:1e6,sourceLeft:0,outlineTop:1e6,cursorIndex:49,column:0}}));
    assert.equal(prepared.inspectionPosition.sourceTop,500*22-220);assert.equal(prepared.inspectionPosition.outlineTop,50*48-240);
    await page.evaluate(()=>window.show(window.meta("a"),{sourceTop:1e6,sourceLeft:0,outlineTop:1e6,cursorIndex:49,column:0}));
    const saved=await page.evaluate(()=>window.widget.position());
    assert.equal(saved.sourceTop,500*22-220);assert.equal(saved.outlineTop,50*48-240);
    assert.ok((await page.locator("#source").textContent()).includes("line a 491"));
    assert.equal(await page.locator('#outline button[data-index="45"]').count(),1);
    await page.locator("#outline button[tabindex='0']").focus();
    await page.evaluate(()=>{window.adjustAnchor=true;return window.show(window.meta("a","v2","s2"));});
    const query=await page.evaluate(()=>window.calls.at(-1));
    assert.equal(query.previous_snapshot,"s");assert.equal(query.cursor,"a-e49");assert.equal(query.anchor,"a-e45");assert.equal(query.anchor_offset,"8");
    assert.equal(await page.locator("#outline").evaluate(n=>n.scrollTop),saved.outlineTop-48);
    await page.locator("#source").focus();
    await page.evaluate(()=>window.show(window.meta("a","v3","s3")));
    assert.equal(await page.evaluate(()=>window.calls.at(-1).cursor),"a-e49");
    assert.equal(await page.evaluate(()=>document.activeElement.id),"source");
    await page.locator("#outline button[tabindex='0']").focus();
    await page.evaluate(()=>window.show({snapshot:"empty",source:null,selection:{file:null,entity:null}}));
    assert.equal(await page.locator(".code-line").count(),0);assert.equal(await page.locator("#outline button").count(),0);
    assert.equal(await page.locator("#outline").evaluate(n=>document.activeElement===n),true);
    await page.evaluate(()=>window.show(window.meta("a")));
    assert.equal(await page.locator(".source-empty").count(),0);
    assert.equal(await page.locator("#outline p").count(),0);
    assert.ok((await page.locator("#source").textContent()).includes("line a 1"));
    await page.evaluate(()=>window.show(window.meta("b","b","s",1)));
    assert.equal(await page.evaluate(()=>window.widget.position().cursorIndex),1);
    assert.deepEqual(await page.evaluate(()=>window.errors),[]);
  }finally{await page.close();}
});

test("preparation does not change live state and empty/restored views preserve keyboard focus without page scrolling",async()=>{
  const page=await fixture();try{
    const before=await page.evaluate(()=>window.widget.position());
    const prepared=await page.evaluate(()=>window.widget.prepare({snapshot:"empty",source:null,selection:{file:null,entity:null}}));
    assert.equal(prepared.source,null);assert.equal(prepared.inspection,null);
    assert.deepEqual(await page.evaluate(()=>window.widget.position()),before);
    await page.evaluate(()=>{document.body.style.paddingTop="1200px";document.body.style.height="3000px";document.querySelector("#outline button").focus({preventScroll:true});});
    const scroll=await page.evaluate(()=>scrollY);
    await page.evaluate(()=>window.show({snapshot:"empty",source:null,selection:{file:null,entity:null}}));
    assert.equal(await page.locator("#outline").evaluate(n=>document.activeElement===n),true);
    assert.equal(await page.evaluate(()=>scrollY),scroll);
    await page.evaluate(()=>window.show(window.meta("a")));
    await page.locator("#outside").evaluate(n=>n.focus({preventScroll:true}));
    await page.evaluate(()=>window.show({snapshot:"empty",source:null,selection:{file:null,entity:null}}));
    assert.equal(await page.evaluate(()=>document.activeElement.id),"outside");
    assert.deepEqual(await page.evaluate(()=>window.activations),[]);
  }finally{await page.close();}
});
test("changed-context returns reveal current code or the file start even when file and selection are already current",async()=>{
  const page=await fixture();try{
    await page.evaluate(()=>window.show(window.meta("a","a","s",30)));
    await page.locator("#source").evaluate(n=>{n.scrollTop=8000;});
    await page.locator("#outline button[tabindex='0']").focus();await page.keyboard.press("End");
    await page.evaluate(()=>window.show(window.meta("a","a","s",30),undefined,true));
    assert.equal(await page.locator("#source").evaluate(n=>n.scrollTop),(241-4)*22);
    assert.equal(await page.evaluate(()=>window.widget.position().cursorIndex),30);
    await page.locator("#source").evaluate(n=>{n.scrollTop=8000;});
    await page.evaluate(()=>window.show(window.meta("a"),undefined,true));
    assert.equal(await page.locator("#source").evaluate(n=>n.scrollTop),0);
    assert.equal(await page.evaluate(()=>window.widget.position().cursorIndex),0);
    assert.equal(await page.evaluate(()=>window.widget.position().outlineTop),0);
    await page.evaluate(()=>window.show(window.meta("a","new-version","s2",30),undefined,true));
    assert.equal(await page.evaluate(()=>window.calls.at(-1).previous_snapshot),undefined);
    assert.equal(await page.evaluate(()=>window.calls.at(-1).cursor),undefined);
    await page.evaluate(()=>window.show(window.meta("long")));
    await page.getByRole("button",{name:"Last text on line 1",exact:true}).click();
    await page.waitForFunction(()=>document.getElementById("source").textContent.includes("TAIL"));
    await page.locator("#source").evaluate(n=>{n.scrollLeft=150;});
    await page.evaluate(()=>window.show(window.meta("long"),undefined,true));
    assert.equal(await page.locator("#source").evaluate(n=>n.scrollLeft),0);
    assert.equal(await page.evaluate(()=>window.widget.position().columnLine),undefined);
    assert.ok(!(await page.locator("#source").textContent()).includes("TAIL"));
    assert.deepEqual(await page.evaluate(()=>window.errors),[]);
  }finally{await page.close();}
});
