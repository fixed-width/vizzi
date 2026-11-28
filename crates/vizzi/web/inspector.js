import {kindGroup} from "/presentation.js";
import { createWindowReader } from "/windows.js";
import { decorateSourceActions } from "/call-actions.js";
import {copySourceSelection,renderReflowText,renderStyledText} from "/reflow-text.js";
import {SourceLayout} from "/source-layout.js";

const SOURCE_ROW = 22, OUTLINE_ROW = 48, SOURCE_GUTTER = 60, SOURCE_PADDING = 22;
function coversRows(rows,first,last) {
  for(let number=first;number<=last;number++)if(!rows.has(number))return false;
  return true;
}

export function createInspector({ source, space, outline, transport, select, onError, makeReader=createWindowReader, decorateLine=()=>{}, loadActions, activateAction, reflow=false, emptyText="Select a file in the repository map." }) {
  const outlineSpace = outline.ownerDocument.createElement("div"); outline.replaceChildren(outlineSpace);
  outline.setAttribute("role","list"); outline.setAttribute("aria-label","Declarations");
  outline.style.maxHeight = `${180 * OUTLINE_ROW}px`; source.style.maxHeight = `${64 * SOURCE_ROW}px`;
  let view, outlineRows, sourceRows = new Map(), entries = new Map(), cursor = 0, scheduled = false, queuedActivation = false;
  let columnLine, column = 0;
  let actionsByLine = new Map(), actionRequest;
  const sourceNodes = new Map(), outlineNodes = new Map();
  const layout=reflow?new SourceLayout(source,schedule):null;
  let smartReflow=reflow, wrappedRows=[];
  if(reflow){
    source.classList.add("source-reflow");
    document.addEventListener("copy",event=>copySourceSelection(source,[...sourceNodes.values()].sort((a,b)=>a.line.number-b.line.number),event));
  }
  const emptyNote=document.createElement("p");emptyNote.className="source-empty";
  const emptyOutline=document.createElement("p");emptyOutline.className="outline-empty";
  const focused = () => outline.contains(document.activeElement) ? document.activeElement.closest("button[data-declaration]")?.dataset.declaration : undefined;
  // Only the current source window contributes continuation heights.
  function sourceReading(top=source.scrollTop) {
    let added=0;
    for(const {top:rowTop,plan} of wrappedRows){
      if(top<rowTop)break;
      const segment=Math.floor((top-rowTop)/SOURCE_ROW);
      if(segment<plan.length)return {sourceTop:top-added-segment*SOURCE_ROW,sourceByte:plan[segment].start};
      added+=(plan.length-1)*SOURCE_ROW;
    }
    return {sourceTop:top-added};
  }
  function displayTop(sourceTop,sourceByte=0) {
    let top=sourceTop;
    const number=Math.floor(sourceTop/SOURCE_ROW)+1;
    for(const row of wrappedRows){
      if(row.number>number)break;
      const segments=row.number===number?row.plan.findLastIndex(piece=>piece.start<=sourceByte):row.plan.length-1;
      top+=segments*SOURCE_ROW;
    }
    return top;
  }
  function restoreSource({sourceTop,sourceByte,sourceLine}) {
    const top=displayTop(sourceTop,sourceByte);
    source.scrollTop=sourceLine===undefined?top:Math.max(top,displayTop((sourceLine-1)*SOURCE_ROW)+SOURCE_ROW-source.clientHeight);
  }
  function ranges(sourceTop = sourceReading().sourceTop, outlineTop = outline.scrollTop) {
    return {line_start:Math.max(1,Math.floor(sourceTop/SOURCE_ROW)-16+1),line_count:96,outline_start:Math.max(0,Math.floor(outlineTop/OUTLINE_ROW)-8),outline_count:200,cursor_index:cursor,column_line:columnLine,column,reflow:reflow||undefined};
  }
  function revealOutline(index) {
    const top=index*OUTLINE_ROW;
    outline.scrollTop=Math.max(0,Math.min(top,Math.max(outline.scrollTop,top+OUTLINE_ROW-outline.clientHeight)));
  }
  function sizes() {
    space.style.height=`${Math.max(1,view?.source?.summary.lines ?? 0)*SOURCE_ROW}px`;
    outlineSpace.style.height=`${Math.max(1,view?.source?.summary.declarations ?? 0)*OUTLINE_ROW}px`;
  }
  function refreshActions(window) {
    actionRequest?.abort();
    actionRequest=undefined;
    actionsByLine.clear();
    if(!loadActions||!window?.lines.length)return;
    const request=actionRequest=new AbortController();
    void loadActions(view,window,request.signal).then(actions=>{
      if(request.signal.aborted)return;
      actionsByLine=Map.groupBy(actions,action=>action.position?.line);
      renderSource();
    }).catch(()=>{}).finally(()=>{if(actionRequest===request)actionRequest=undefined;});
  }
  function coversOutlineViewport(rows) {
    const first = Math.floor(outline.scrollTop / OUTLINE_ROW);
    const end = Math.min(view.source.summary.declarations, Math.ceil((outline.scrollTop + outline.clientHeight) / OUTLINE_ROW));
    return rows?.[0]?.index <= first && rows.at(-1).index >= end - 1;
  }
  function accept(window, query, intent = {}, retainedFragment, reading, preserveOutline = false) {
    const hadFocus=outline.contains(document.activeElement);
    if (intent.fragment) {
      for(const row of window.lines)sourceRows.set(row.number,row);
    } else {
      sourceRows=new Map(window.lines.map(row=>[row.number,row]));
      refreshActions(window);
    }
    if(retainedFragment) {
      const row=sourceRows.get(retainedFragment.number);
      if(row && row.column===retainedFragment.column && row.end_column<retainedFragment.end_column)sourceRows.set(row.number,retainedFragment);
    }
    if (!preserveOutline || coversOutlineViewport(window.outline) || !coversOutlineViewport(outlineRows)) {
      outlineRows = window.outline;
    }
    entries = new Map(outlineRows.map(row => [row.index, row]));
    if(window.cursor)entries.set(window.cursor.index,window.cursor);
    cursor=window.cursor?.index ?? 0;
    renderSource(reading);renderOutline();
    if(hadFocus)focusCursor();
    if(intent.end)source.scrollLeft=source.scrollWidth;
    if(queuedActivation && entries.has(cursor)) {queuedActivation=false;void select(entries.get(cursor));}
  }
  const reader=makeReader({transport,onWindow:accept,onError});
  function focusCursor() { const row=entries.get(cursor); if(row)outlineNodes.get(row.id)?.button.focus({preventScroll:true}); }
  function renderSource(reading) {
    if(!view?.source || !view.source.summary.lines) {
      wrappedRows=[];sourceNodes.clear();space.replaceChildren(emptyNote);emptyNote.textContent=view?.source?"Source encoding is unavailable in this inspector.":emptyText;return;
    }
    if(layout?.defer(()=>{
      if(reading)return false;
      const firstVisible=Math.floor(sourceReading().sourceTop/SOURCE_ROW)+1;
      const bottom=source.scrollTop+Math.max(0,source.clientHeight-1);
      const lastVisible=Math.min(view.source.summary.lines,Math.floor(sourceReading(bottom).sourceTop/SOURCE_ROW)+1);
      return coversRows(sourceNodes,firstVisible,lastVisible)||!coversRows(sourceRows,firstVisible,lastVisible);
    }))return;
    reading??=sourceReading();
    const first=Math.max(1,Math.floor(source.scrollTop/SOURCE_ROW)-16+1),last=Math.min(view.source.summary.lines+1,Math.ceil((source.scrollTop+source.clientHeight)/SOURCE_ROW)+17,first+96);
    const numbers=[...sourceRows.keys()].filter(number=>reflow||number>=first&&number<last).sort((a,b)=>a-b);
    const wanted=new Set(numbers);
    for(const [number,node] of sourceNodes)if(!wanted.has(number)){node.row.remove();sourceNodes.delete(number);}
    const width=source.clientWidth-SOURCE_GUTTER-SOURCE_PADDING;
    const planFor=layout?.planRows(width,smartReflow,{
      endInsetFor:node=>node.controls.offsetWidth+parseFloat(getComputedStyle(node.controls).marginRight),
    });
    wrappedRows=[];
    let added=0,previousRow=null,previousNumber=0;
    for(const number of numbers) {
      const line=sourceRows.get(number);
      let node=sourceNodes.get(number);
      if(!node) {
        const row=document.createElement("div"),label=document.createElement("span"),text=document.createElement("span"),controls=document.createElement("span");row.className="code-line";label.className="line-no";label.setAttribute("aria-hidden","true");text.className="source-text";controls.className="fragment-controls";
        row.dataset.line=String(number);label.textContent=number;
        const buttons=new Map();
        for(const [action,title] of [["first","First"],["previous","Earlier"],["next","Later"],["last","Last"]]) {
          const button=document.createElement("button");button.textContent=title;button.setAttribute("aria-label",`${title} text on line ${number}`);button.addEventListener("click",()=>fragment(number,action));controls.append(button);buttons.set(action,button);
        }
        row.append(label,controls,text);node={row,controls,text,buttons};sourceNodes.set(number,node);
      }
      const nextRow=previousRow?previousRow.nextSibling:space.firstChild;
      if(nextRow!==node.row)space.insertBefore(node.row,nextRow);
      previousRow=node.row;
      for(const [action,button] of node.buttons)button.hidden=action==="first" || action==="previous" ? !line.left : !line.right;
      if(reflow)node.controls.hidden=!line.left&&!line.right;
      const changedLine=node.line!==line;
      const plan=planFor?.(node,line)??null;
      const top=(number-1)*SOURCE_ROW+added;
      if(reflow)node.row.style.marginTop=`${(number-previousNumber-1)*SOURCE_ROW}px`;
      else node.row.style.top=`${top}px`;
      previousNumber=number;
      if(plan){
        wrappedRows.push({number,top,plan});
        added+=(plan.length-1)*SOURCE_ROW;
      }
      const actions=actionsByLine.get(number);
      if(changedLine||node.actions!==actions||node.plan!==plan){
        if(reflow)renderReflowText(node.text,line,plan);
        else if(changedLine||node.actions?.length)renderStyledText(node.text,line);
        if(activateAction&&actions?.length){
          const captured=view;
          decorateSourceActions(node.text,line,actions,(...args)=>activateAction(captured,...args),"Ctrl/⌘-click opens in Call workspace");
        }
        node.line=line;node.actions=actions;
      }
      node.plan=plan;
      const active=view.source.summary.active;node.row.classList.toggle("active",Boolean(active && number>=active.start_line && number<=active.end_line));
      decorateLine(node.row,number,view);
    }
    if(reflow){
      space.style.height=`${view.source.summary.lines*SOURCE_ROW+added}px`;
      restoreSource(reading);
    }
  }
  function renderOutline() {
    const total=view?.source?.summary.declarations ?? 0;
    if(!total) {outlineNodes.clear();outlineSpace.replaceChildren(emptyOutline);outline.tabIndex=0;emptyOutline.textContent="No declarations extracted";return;}
    outline.tabIndex=-1;
    const first=Math.max(0,Math.floor(outline.scrollTop/OUTLINE_ROW)-8),last=Math.min(total,Math.ceil((outline.scrollTop+outline.clientHeight)/OUTLINE_ROW)+8,first+199);
    const wanted=new Set();for(let i=first;i<last;i++)if(entries.has(i))wanted.add(i);if(entries.has(cursor))wanted.add(cursor);
    const keep=new Set([...wanted].map(index=>entries.get(index).id));const oldFocus=focused();if(!entries.has(cursor) && oldFocus)keep.add(oldFocus);
    for(const [id,node] of outlineNodes)if(!keep.has(id)){node.row.remove();outlineNodes.delete(id);}
    for(const index of wanted) {
      const fact=entries.get(index);let node=outlineNodes.get(fact.id);
      if(!node){const row=document.createElement("div"),button=document.createElement("button"),name=document.createElement("span"),detail=document.createElement("small"),scope=document.createElement("span"),depth=document.createElement("span"),parent=document.createElement("span");row.className="outline-entry";row.setAttribute("role","listitem");name.className="declaration-name";scope.className="scope-context";depth.className="scope-depth";parent.className="scope-parent";scope.append(depth,parent);button.append(name,detail,scope);row.append(button);outlineSpace.append(row);node={row,button,name,detail,scope,depth,parent};outlineNodes.set(fact.id,node);}
      node.row.style.top=`${index*OUTLINE_ROW}px`;node.row.setAttribute("aria-posinset",String(index+1));node.row.setAttribute("aria-setsize",String(total));
      node.button.dataset.declaration=fact.id;node.button.dataset.index=index;node.button.tabIndex=index===cursor?0:-1;node.button.setAttribute("aria-current",String(view.selection.entity===fact.id));node.button.title=`Reveal ${fact.kind} ${fact.name}${fact.name_truncated?"… (inspect source for full name)":""} starting at Line ${fact.start_line}`;node.button.setAttribute("aria-label",node.button.title);
      node.scope.hidden=fact.depth===undefined;node.row.dataset.depth=String(fact.depth ?? 0);
      node.depth.hidden=!fact.depth;node.depth.textContent=fact.depth?`Depth ${fact.depth}`:"";
      node.parent.textContent=fact.depth?`in ${fact.scope || "unnamed scope"}`:"File outline";
      node.scope.title=fact.depth?`Enclosed by ${fact.scope || "unnamed scope"}; declaration depth ${fact.depth}`:"No enclosing declaration in this outline";
      node.button.setAttribute("aria-description",node.scope.hidden?"":node.scope.title);
      node.name.textContent=fact.name+(fact.name_truncated?"…":"");node.detail.replaceChildren();const badge=document.createElement("span");badge.className=`kind-badge kind-${kindGroup(fact.kind)}`;badge.textContent=fact.kind;node.detail.append(badge,document.createTextNode(` · Line ${fact.start_line}`));
    }
  }
  function covered(query) {
    if(!outlineRows)return false;
    const lines=view.source.summary.lines,decls=view.source.summary.declarations;
    const lastLine=Math.min(lines,Math.ceil(sourceReading(source.scrollTop+source.clientHeight).sourceTop/SOURCE_ROW)+16,query.line_start+95);
    const lastEntry=Math.min(decls,Math.ceil((outline.scrollTop+outline.clientHeight)/OUTLINE_ROW)+8,query.outline_start+199);
    if(!coversRows(sourceRows,query.line_start,lastLine))return false;
    for(let i=query.outline_start;i<lastEntry;i++)if(!entries.has(i))return false;
    return !decls || entries.has(cursor);
  }
  function schedule() {
    if(scheduled)return;scheduled=true;
    requestAnimationFrame(()=>{scheduled=false;if(!view?.source)return;renderSource();renderOutline();const query=ranges();reader.activate(view);if(!covered(query))void reader.request(query,{focus:false});});
  }
  async function fragment(number,action) {
    const row=sourceRows.get(number);if(!row)return;
    // Leave room for UTF-8 boundary rounding so the final fragment includes the end.
    const tail=row.line_bytes>32768?row.line_bytes-32765:0;
    columnLine=number;column=action==="first"?0:action==="previous"?Math.max(0,row.column-32768):action==="last"?tail:row.end_column;
    source.scrollLeft=0;
    return reader.request({...ranges(),line_start:number,line_count:1,column_line:number,column},{fragment:true,end:action==="last"});
  }
  outline.addEventListener("click",event=>{const button=event.target.closest("button[data-declaration]");if(!button)return;const index=Number(button.dataset.index),row=entries.get(index);if(row){cursor=index;void select(row);}});
  outline.addEventListener("focusin",event=>{const button=event.target.closest("button[data-declaration]");if(button){cursor=Number(button.dataset.index);renderOutline();}});
  outline.addEventListener("keydown",event=>{
    const total=view?.source?.summary.declarations ?? 0;if(!total)return;
    if((event.key==="Enter" || event.key===" ") && !entries.has(cursor)){event.preventDefault();queuedActivation=true;return;}
    const page=Math.max(1,Math.floor(outline.clientHeight/OUTLINE_ROW));const destination={Home:0,End:total-1,ArrowDown:cursor+1,ArrowUp:cursor-1,PageDown:cursor+page,PageUp:cursor-page}[event.key];if(destination===undefined)return;
    event.preventDefault();queuedActivation=false;cursor=Math.max(0,Math.min(total-1,destination));revealOutline(cursor);reader.activate(view);renderOutline();focusCursor();
    if(!covered(ranges()))void reader.request(ranges(),{focus:true});
  });
  let sourceTop=source.scrollTop;
  source.addEventListener("scroll",()=>{if(source.scrollTop!==sourceTop){sourceTop=source.scrollTop;schedule();}},{passive:true});outline.addEventListener("scroll",schedule,{passive:true});
  const resize=new ResizeObserver(schedule);resize.observe(source);resize.observe(outline);
  return {
    setReflow(enabled) {smartReflow=enabled;renderSource();},
    position() {
      return { ...sourceReading(), sourceLeft:source.scrollLeft, outlineTop:outline.scrollTop,
        cursorIndex:cursor, columnLine, column };
    },
    async prepare(next,options={}) {
      if(!next.source)return {...next,inspection:null};
      const changedFile=view?.source?.id!==next.source.id,changedSelection=changedFile || view.selection.entity!==next.selection.entity;
      const sameDocument=!changedFile && view.source.version===next.source.version;
      const active=next.source.summary.active;
      const restore=options.restore;
      const reveal=Boolean(options.reveal);
      const fromOutline = options.fromOutline && !changedFile;
      const sourceLine=options.line??(!restore&&active&&(changedSelection||reveal||fromOutline)?active.start_line:undefined);
      const reading=sourceLine!==undefined?{sourceTop:Math.max(0,(sourceLine-4)*SOURCE_ROW)}:restore??(changedFile||reveal?{sourceTop:0}:sourceReading());
      const sourceTop=Math.max(0,Math.min(reading.sourceTop,next.source.summary.lines*SOURCE_ROW-(reflow?1:source.clientHeight)));
      let outlineTop=restore?restore.outlineTop:changedFile || reveal?0:outline.scrollTop;
      if(!restore && !fromOutline && (changedSelection || reveal) && active)outlineTop=Math.max(0,active.index*OUTLINE_ROW-outline.clientHeight/2);
      outlineTop=Math.max(0,Math.min(outlineTop,next.source.summary.declarations*OUTLINE_ROW-outline.clientHeight));
      const query=ranges(sourceTop,outlineTop);query.cursor_index=changedFile?(active?.index ?? 0):changedSelection && active?active.index:cursor;
      if(reveal)query.cursor_index=active?.index ?? 0;
      if(restore) {
        query.cursor_index=restore.cursorIndex;
        query.column_line=restore.columnLine;query.column=restore.column;
      }
      if(!restore && (!sameDocument || reveal)){delete query.column_line;query.column=0;}
      if(!restore && !reveal && !changedFile && view.source.version!==next.source.version) {
        const top=Math.floor(outline.scrollTop/OUTLINE_ROW),anchor=entries.get(top),focus=focused() ?? entries.get(cursor)?.id;
        query.previous_snapshot=view.snapshot;query.cursor=focus;if(anchor){query.anchor=anchor.id;query.anchor_offset=top-query.outline_start;}
      }
      const window=await reader.prepare(next,query,options);
      outlineTop+=(window.outline_start-query.outline_start)*OUTLINE_ROW;
      const keepOutline = fromOutline && sameDocument && !restore;
      return {...next,inspection:window,inspectionPosition:{sourceTop,sourceByte:reading.sourceByte,sourceLine,
        outlineTop: keepOutline ? undefined : outlineTop,
        sourceLeft:restore?restore.sourceLeft:sameDocument && !reveal?source.scrollLeft:0,
        columnLine:restore?restore.columnLine:sameDocument && !reveal?columnLine:undefined,
        column:restore?restore.column:sameDocument && !reveal?column:0}};
    },
    apply(next) {
      const hadFocus=outline.contains(document.activeElement);
      const position=next.inspectionPosition;
      const sameDocument = view?.source?.id === next.source?.id && view?.source?.version === next.source?.version;
      const retainedFragment = sameDocument ? sourceRows.get(position?.columnLine) : undefined;
      view=next;queuedActivation=false;reader.activate(view);
      if(!sourceNodes.size)space.replaceChildren();if(!outlineNodes.size)outlineSpace.replaceChildren();sizes();
      if(next.inspection){
        const preserveOutline = sameDocument && position.outlineTop === undefined;
        if(!reflow)source.scrollTop=position.sourceTop;
        if (position.outlineTop !== undefined) outline.scrollTop = position.outlineTop;
        columnLine=position.columnLine;column=position.column;
        accept(next.inspection, {}, {}, retainedFragment, reflow ? position : undefined, preserveOutline);
        source.scrollLeft=position.sourceLeft;
        if (reflow || preserveOutline) schedule();
      }
      else {refreshActions();outlineRows=undefined;sourceRows=new Map();entries=new Map();cursor=0;columnLine=undefined;column=0;renderSource();renderOutline();}
      if(hadFocus && !view.source?.summary.declarations)outline.focus({preventScroll:true});
    },
  };
}
