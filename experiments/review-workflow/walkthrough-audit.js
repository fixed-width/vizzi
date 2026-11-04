import assert from "node:assert/strict";

const integer=value=>Number.isSafeInteger(value) && value>=0;
const byteLength=text=>Buffer.byteLength(text,"utf8");
const sides=["base","working"];

export function auditResponse(uri,body) {
  const url=new URL(uri,"http://local");
  const match=url.pathname.match(/^\/api\/comparisons\/([^/]+)\/(files|window|diff)$/);
  if(!match)return null;
  const [,token,kind]=match;
  assert.equal(body?.schema,1,"projection schema");
  assert.equal(body.token,decodeURIComponent(token),"projection token");
  assert.ok(typeof body.comparison==="string" && body.comparison.length>0,"comparison identity");
  const result={kind,token:body.token,comparison:body.comparison};
  if(kind==="files") {
    assert.ok(Array.isArray(body.files) && body.files.length<=200,"file page bound");
    return {...result,rows:body.files.length};
  }
  assert.equal(kind==="window"?body.window?.file:body.file,url.searchParams.get("file"),"projection file");
  if(kind==="diff") {
    for(const side of sides)assert.equal(body[`${side}_version`],url.searchParams.get(`${side}_version`),"diff version");
    assert.ok(Array.isArray(body.hunks) && body.hunks.length<=200,"hunk page bound");
    assert.ok(Array.isArray(body.edits) && body.edits.length<=200,"edit page bound");
    return {...result,hunks:body.hunks.length,edits:body.edits.length};
  }
  assert.ok(sides.includes(body.side),"explicit window side");
  assert.equal(body.side,url.searchParams.get("side"),"window side pin");
  assert.equal(body.window.version,url.searchParams.get("version"),"window version pin");
  const window=body.window;
  assert.equal(window.schema,1,"document schema");
  assert.ok(Array.isArray(window.lines) && window.lines.length<=96,"source row bound");
  assert.ok(Array.isArray(window.outline) && window.outline.length<=200,"outline page bound");
  let bytes=0;
  for(const row of window.lines) {
    assert.ok(integer(row.number) && row.number>0,"source line number");
    assert.ok([row.column,row.end_column,row.line_bytes].every(integer) && row.column<=row.end_column && row.end_column<=row.line_bytes,"fragment byte coordinates");
    assert.equal(byteLength(row.text),row.end_column-row.column,"fragment UTF-8 bytes");
    assert.equal(row.left,row.column>0,"earlier-fragment flag");assert.equal(row.right,row.end_column<row.line_bytes,"later-fragment flag");
    bytes+=byteLength(row.text);
  }
  assert.ok(bytes<=32768,"source byte bound");
  return {...result,file:window.file,version:window.version,side:body.side,sourceBytes:bytes,rows:window.lines.length,outlineRows:window.outline.length};
}

export function auditCheckpoint(state,file,windows,sources) {
  assert.ok(state.token && state.comparison,"displayed comparison pins");
  assert.equal(state.token,file.token,"inventory token");
  assert.equal(state.comparison,file.comparison,"inventory comparison");
  assert.equal(state.path,file.path,"displayed source path");
  assert.ok(sides.includes(state.side),"selected side");
  assert.deepEqual(state.selectedSides,[state.side],"one selected side");
  assert.ok(integer(state.fileButtons) && state.fileButtons<=200,"file DOM bound");
  assert.ok(integer(state.hunkButtons) && state.hunkButtons<=200,"hunk DOM bound");
  let verifiedRows=0,maxRenderedRows=0,maxOutlineButtons=0;
  const decoder=new TextDecoder("utf-8",{fatal:true});
  for(const side of sides) {
    const pane=state.panes[side],version=file[`${side}_version`];
    assert.equal(pane.version,version ?? "","displayed side version");
    assert.ok(integer(pane.renderedRows) && pane.renderedRows<=96,"source DOM bound");
    assert.ok(integer(pane.outlineButtons) && pane.outlineButtons<=200,"outline DOM bound");
    assert.ok(pane.rows.length<=pane.renderedRows,"visible/rendered source count");
    maxRenderedRows=Math.max(maxRenderedRows,pane.renderedRows);maxOutlineButtons=Math.max(maxOutlineButtons,pane.outlineButtons);
    if(!version) {assert.equal(pane.renderedRows,0,"absent side has no source");continue;}
    const matching=windows.filter(body=>body.token===state.token && body.comparison===state.comparison && body.side===side && body.window?.file===file.file && body.window.version===version);
    assert.ok(matching.length>0,"source response anchors");
    if(!pane.rows.length)continue;
    const lines=decoder.decode(sources[side]).split("\n");
    for(const visible of pane.rows) {
      assert.ok(integer(visible.number) && visible.number>0 && visible.number<=lines.length,"visible source line");
      const raw=Buffer.from(lines[visible.number-1].replace(/\r$/,""),"utf8");
      const candidates=matching.flatMap(body=>body.window.lines.filter(row=>row.number===visible.number));
      assert.ok(candidates.some(row=>{
        if(row.line_bytes!==raw.length || row.end_column>raw.length)return false;
        const text=`${row.left?"…":""}${decoder.decode(raw.subarray(row.column,row.end_column))}${row.right?"…":""}`;
        return visible.text===text;
      }),"visible source differs from captured bytes or has no matching fragment");
      verifiedRows++;
    }
  }
  return {verifiedRows,maxRenderedRows,maxOutlineButtons};
}
