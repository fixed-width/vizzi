import {installAppHeader} from '/app-header.js';
import {reviewLabel} from "/view-status.js";
import {createMessage,textRow} from "/messages.js";
import {installAppearance} from "/appearance.js";
import {pageCount} from "/presentation.js";
import {installSplitter} from "/splitter.js";
import {createReviewController} from "/review-controller.js";
import {createReviewWindowReader} from "/review-windows.js";
import {createInspector} from "/inspector.js";
import {followChanges} from "/updates.js";

installAppHeader(document.querySelector('body > header'));
const element=id=>document.getElementById(id);
const node=(tag,text)=>{const value=document.createElement(tag);value.textContent=text;return value;};
const sides=["base","working"],inspectors={};
let view,updates,filterTimer;
installAppearance(element("theme"));
const dismissible=new Set(["comparison-error","review-notice"]);
const messages=Object.fromEntries([["review-status","review status"],["comparison-error","comparison error"],["comparison-labels","comparison details"],["review-notice","review notice"],["file-change","file status"],["diff-note","diff note"],["base-evidence","base availability"],["working-evidence","working availability"]].map(([id,label])=>[id,dismissible.has(id)?createMessage(element(id),{label:`Dismiss ${label}`,...(id==="review-notice"?{content:element(id).querySelector("span")}:{})}):textRow(element(id))]));
installSplitter(element("review-workspace"),{key:"review-files",label:"Review files and source widths",initial:.21,minBefore:170,minAfter:400});
installSplitter(document.querySelector(".review-pair"),{key:"review-pair",label:"Base and target source widths",initial:.5,minBefore:220,minAfter:220});
for(const side of sides)installSplitter(element(`${side}-outline`).closest(".source-layout"),{key:`review-${side}-outline`,label:`${side} declarations and source widths`,initial:.3,minBefore:90,minAfter:130});
function captureError(message,key){messages["comparison-error"].show(message,key);}
function comparisonMode(){const commits=element("comparison-mode").value==="commits";element("target-field").hidden=!commits;element("target-revision").required=commits;element("compare").textContent=commits?"Compare commits":"Compare with working tree";}
element("comparison-mode").addEventListener("change",comparisonMode);
function cancelFilter(){clearTimeout(filterTimer);filterTimer=undefined;}
const transport=async(uri,options)=>{
  const response=await fetch(uri,options);
  if(!response.ok){let message=await response.text();try{message=JSON.parse(message).message ?? message;}catch{}throw Object.assign(new Error(message.slice(0,512)),{status:response.status});}
  return response.status===204?null:response.json();
};
function notice(message) {messages["review-notice"].show(message);}
for(const side of sides) {
  inspectors[side]=createInspector({source:element(`${side}-source`),space:element(`${side}-space`),outline:element(`${side}-outline`),transport,makeReader:createReviewWindowReader,
    emptyText:"No retained text on this side. Inspect the availability description above.",
    select:entity=>controller.navigate({side,entity:{id:entity.id,index:entity.index,start_line:entity.start_line}}),
    onError:error=>notice(`${error.message}. The displayed capture is kept; retry inspection or open the current comparison.`),
    decorateLine:(row,number)=>{
      const changed=Boolean(view?.diff?.edits.some(edit=>number-1>=edit[side].lines.start && number-1<edit[side].lines.end));
      row.classList.toggle("review-removed",changed && side==="base");row.classList.toggle("review-added",changed && side==="working");
    },
  });
  element(`select-${side}`).addEventListener("click",()=>controller.navigate({side,entity:null}));
}
const capture=()=>Object.fromEntries(sides.map(side=>[side,inspectors[side].position()]));
const controller=createReviewController({transport,capture,onView:render,onNotice:notice,onError:error=>captureError(error.message),
  onStatus:status=>{
    captureError(status.error?.message,status.ticket);
    messages["review-status"].show(reviewLabel(status));
    element("review-workspace").setAttribute("aria-busy",String(status.pending));
    element("refresh-working").disabled=!status.active || status.pending || Boolean(status.active.target);element("refresh-working").hidden=Boolean(status.active?.target);element("close-comparison").disabled=!status.active;
    element("open-current").disabled=!status.active;
  },
  onHistory:state=>{element("review-back").disabled=!state.canBack;element("review-forward").disabled=!state.canForward;element("review-navigation").setAttribute("aria-busy",String(state.pending));},
  history:{push:state=>history.pushState(state,""),replace:state=>history.replaceState(state,""),go:delta=>history.go(delta)},
});
function rangeLabel(range) {const count=range.end-range.start;return `${count?range.start+1:range.start},${count}`;}
function header(hunk) {return `@@ -${rangeLabel(hunk.base.lines)} +${rangeLabel(hunk.working.lines)} @@`;}
function pager(previous,next,start,following,action) {
  element(previous).disabled=start===0;element(next).disabled=following==null;
  element(previous).onclick=()=>action(Math.max(0,start-200));element(next).onclick=()=>action(following);
}
function evidence(entry,body,side) {
  if(!entry)return `Absent from ${side}`;
  if(entry.state==="unobserved")return `Not observed: ${entry.unavailable}`;
  const parts=[entry.kind];
  if(entry.bytes!=null)parts.push(entry.bytes===0?"Empty file (0 bytes)":`${entry.bytes.toLocaleString()} bytes`);
  if(entry.executable!=null)parts.push(entry.executable?"executable":"not executable");
  if(entry.literal!=null)parts.push(`literal ${entry.kind==="symlink"?"target":"reference"}: ${entry.literal}${entry.literal_truncated?"… (truncated)":""}`);
  if(entry.unavailable)parts.push(`content unavailable: ${entry.unavailable}`);
  if(body?.error)parts.push(`source unavailable: ${body.error}`);
  if(body?.window && body.window.line_total===0 && entry.bytes>0)parts.push("binary or unsupported text encoding");
  return parts.join(" · ");
}
function render(next) {
  const fileFocus=document.activeElement.closest?.("#review-file-list button")?.dataset.file;
  const hunkFocus=document.activeElement.closest?.("#review-hunks button")?.dataset.hunk;
  view=next;
  const workspace=element("review-workspace");workspace.dataset.token=next?.metadata.token ?? "";workspace.dataset.comparison=next?.metadata.comparison ?? "";
  const files=element("review-file-list"),hunks=element("review-hunks");files.replaceChildren();hunks.replaceChildren();
  if(!next) {
    messages["comparison-labels"].show("Comparison closed. Choose a baseline to capture another review.");
    element("review-file-title").textContent="Choose a baseline";messages["file-change"].show("No comparison selected");
    element("review-counts").textContent="No comparison captured";
    messages["review-status"].show("Comparison closed");element("file-page-label").textContent="";element("hunk-page-label").textContent="";messages["diff-note"].show(null);element("edit-details").hidden=true;
    for(const id of ["files-previous","files-next","hunks-previous","hunks-next","edits-previous","edits-next"])element(id).disabled=true;
    for(const side of sides){inspectors[side].apply({source:null,selection:{}});messages[`${side}-evidence`].show("No source selected");element(`${side}-contexts`).textContent="";element(`${side}-pane`).dataset.version="";}
    return;
  }
  const {metadata,query,file,diff,windows,selection}=next;
  const committed=Boolean(metadata.target),right=committed?"Target":"Working";
  messages["comparison-labels"].show(`Base ${metadata.revision} → ${metadata.resolved} · ${committed?`Target ${metadata.target.revision} → ${metadata.target.resolved}`:`Working tree observed ${new Date(metadata.working_started_ms).toISOString()} to ${new Date(metadata.working_finished_ms).toISOString()}`} · repository path ${metadata.scope}`);
  element("select-working").textContent=right;element("working-pane").setAttribute("aria-label",`${right} source pane`);element("working-outline").previousElementSibling.textContent=`${right} declarations`;element("working-source").setAttribute("aria-label",`${right} captured source`);
  element("refresh-working").hidden=committed;element("refresh-working").disabled=committed;
  if(filterTimer===undefined)element("review-filter").value=query.filter;element("include-unchanged").checked=query.unchanged;
  const totals=next.files.totals;
  element("review-counts").textContent=`${totals.paths} paths · ${totals.added} added · ${totals.removed} removed · ${totals.modified} modified · ${totals.unchanged} unchanged · ${totals.unassessed} unassessed · ${totals.unavailable} unavailable`;
  element("file-page-label").textContent=pageCount(next.files.offset,next.files.files.length,next.files.total);
  for(const row of next.files.files) {
    const button=node("button",`${row.path}${row.path_truncated?"…":""}`);button.type="button";button.dataset.file=row.file;
    button.append(node("small",row.change));button.setAttribute("aria-current",String(file?.file===row.file));button.addEventListener("click",()=>controller.navigate({file:row}));files.append(button);
  }
  if(!next.files.files.length)files.append(node("p","No paths match. Clear the filter or include unchanged context."));
  pager("files-previous","files-next",next.files.offset,next.files.next,offset=>controller.navigate({offset}));
  element("review-file-title").textContent=file?.path ?? "No file selected";
  messages["file-change"].show(file?`${file.change}${file.path_truncated?" · path label truncated":""} · ${selection.side} side selected${selection.entity?" · lexical declaration selected":""}`:"Select a file to inspect source");
  element("hunk-page-label").textContent=diff?`${diff.hunks.length} of ${diff.hunk_total} hunks · ${diff.remaining_hunks} after this page`:"Textual changes unavailable";
  for(const [index,hunk] of (diff?.hunks ?? []).entries()) {
    const button=node("button",`${query.hunk_start+index+1}: ${header(hunk)}`);button.type="button";button.dataset.hunk=hunk.id;button.setAttribute("aria-current",String(selection.hunk===hunk.id));button.addEventListener("click",()=>controller.navigate({hunk:hunk.id}));hunks.append(button);
  }
  pager("hunks-previous","hunks-next",query.hunk_start,diff?.next_hunk,start=>controller.navigate({hunk_start:start}));
  messages["diff-note"].show(next.diffError?`Textual changes unavailable: ${next.diffError}`:diff?.hunk_total===0?"No textual changes. File type or executable status can still differ.":"Choose a hunk to reveal both ranges, or a declaration to inspect its side. Only loaded change ranges receive diff backgrounds. Line-ending edits can look equal in the preview; exact byte ranges are available below.");
  element("edit-details").hidden=!diff;
  element("edit-page-label").textContent=diff?`Change ranges ${diff.edits.length?query.edit_start+1:0}–${query.edit_start+(diff?.edits.length ?? 0)} / ${diff.edit_total}`:"";
  pager("edits-previous","edits-next",query.edit_start,diff?.next_edit,start=>controller.navigate({edit_start:start}));
  element("edit-ranges").textContent=(diff?.edits ?? []).map(edit=>`− lines ${rangeLabel(edit.base.lines)}, bytes [${edit.base.bytes.start}, ${edit.base.bytes.end})\n+ lines ${rangeLabel(edit.working.lines)}, bytes [${edit.working.bytes.start}, ${edit.working.bytes.end})`).join("\n\n");
  for(const side of sides) {
    const body=windows[side],window=body?.window,version=file?.[`${side}_version`];
    element(`select-${side}`).setAttribute("aria-pressed",String(selection.side===side));
    element(`${side}-pane`).dataset.version=version ?? "";
    messages[`${side}-evidence`].show(evidence(file?.[side],body,side),version);
    const contexts=body?.contexts ?? [];
    element(`${side}-contexts`).textContent=window?[body.classification.kind==="source_only"?"Source only; no declaration analysis assigned.":"Raw syntax; build configuration unknown.",...contexts.map(c=>`${c.language}: ${c.coverage} · ${c.declarations} declarations\n${c.description}${c.truncated?" (truncated)":""}\n${c.diagnostics.join("\n")}${c.hidden_diagnostics?`\n${c.hidden_diagnostics} more diagnostics`:""}`),body.hidden_contexts?`${body.hidden_contexts} more analysis contexts`:""].filter(Boolean).join("\n"):"No retained textual analysis for this side.";
    const active=selection.side===side && selection.entity?window?.cursor:null;
    inspectors[side].apply({review:{token:metadata.token,comparison:metadata.comparison,side},selection:{file:file?.file,entity:active?.id ?? null},source:window?{id:file.file,path:file.path,version,summary:{lines:window.line_total,declarations:window.outline_total,active}}:null,inspection:window,inspectionPosition:body?.position});
  }
  if(fileFocus)[...files.querySelectorAll("button")].find(button=>button.dataset.file===fileFocus)?.focus({preventScroll:true});
  if(hunkFocus)[...hunks.querySelectorAll("button")].find(button=>button.dataset.hunk===hunkFocus)?.focus({preventScroll:true});
}
element("comparison-form").addEventListener("submit",async event=>{event.preventDefault();cancelFilter();element("compare").disabled=true;try{const commits=element("comparison-mode").value==="commits";captureError(null);await controller.create({revision:element("baseline").value,scope:element("scope").value || ".",...(commits?{target_revision:element("target-revision").value}:{})});}finally{element("compare").disabled=false;}});
element("refresh-working").addEventListener("click",()=>controller.refresh());
element("close-comparison").addEventListener("click",()=>{cancelFilter();void controller.close();});
element("review-filter").addEventListener("input",()=>{cancelFilter();filterTimer=setTimeout(()=>{filterTimer=undefined;void controller.navigate({filter:element("review-filter").value,offset:0});},160);});
element("include-unchanged").addEventListener("change",()=>{cancelFilter();void controller.navigate({filter:element("review-filter").value,unchanged:element("include-unchanged").checked,offset:0});});
element("review-back").addEventListener("click",()=>{cancelFilter();controller.move(-1);});
element("review-forward").addEventListener("click",()=>{cancelFilter();controller.move(1);});
window.addEventListener("popstate",event=>{cancelFilter();void controller.traverse(event.state);});
element("open-current").addEventListener("click",()=>{cancelFilter();void controller.openCurrent();});
element("retry-review").addEventListener("click",()=>controller.navigate());
element("reconnect-review").addEventListener("click",()=>updates.restart());
updates=followChanges({url:"/api/comparisons/events",eventName:"comparison",session:()=>controller.updates()});
