import {kindGroup,renderStyledText} from "/presentation.js";
import { createWindowReader } from "/windows.js";

const SOURCE_ROW = 22, OUTLINE_ROW = 48;
export function createInspector({ source, space, outline, transport, select, onError, makeReader=createWindowReader, decorateLine=()=>{}, emptyText="Select a file in the repository map." }) {
  const outlineSpace = outline.ownerDocument.createElement("div"); outline.replaceChildren(outlineSpace);
  outline.setAttribute("role","list"); outline.setAttribute("aria-label","Declarations");
  outline.style.maxHeight = `${180 * OUTLINE_ROW}px`; source.style.maxHeight = `${64 * SOURCE_ROW}px`;
  let view, data, sourceRows = new Map(), entries = new Map(), cursor = 0, scheduled = false, queuedActivation = false;
  let columnLine, column = 0;
  const sourceNodes = new Map(), outlineNodes = new Map();
  const emptyNote=document.createElement("p");emptyNote.className="source-empty";
  const emptyOutline=document.createElement("p");emptyOutline.className="outline-empty";
  const focused = () => outline.contains(document.activeElement) ? document.activeElement.closest("button[data-declaration]")?.dataset.declaration : undefined;
  function ranges(sourceTop = source.scrollTop, outlineTop = outline.scrollTop) {
    return {line_start:Math.max(1,Math.floor(sourceTop/SOURCE_ROW)-16+1),line_count:96,outline_start:Math.max(0,Math.floor(outlineTop/OUTLINE_ROW)-8),outline_count:200,cursor_index:cursor,column_line:columnLine,column};
  }
  function revealOutline(index) {
    const top=index*OUTLINE_ROW;
    outline.scrollTop=Math.max(0,Math.min(top,Math.max(outline.scrollTop,top+OUTLINE_ROW-outline.clientHeight)));
  }
  function sizes() {
    space.style.height=`${Math.max(1,view?.source?.summary.lines ?? 0)*SOURCE_ROW}px`;
    outlineSpace.style.height=`${Math.max(1,view?.source?.summary.declarations ?? 0)*OUTLINE_ROW}px`;
  }
  function accept(window, query, intent = {}, retainedFragment) {
    const hadFocus=outline.contains(document.activeElement);
    if (intent.fragment) {
      for(const row of window.lines)sourceRows.set(row.number,row);
    } else sourceRows=new Map(window.lines.map(row=>[row.number,row]));
    if(retainedFragment) {
      const row=sourceRows.get(retainedFragment.number);
      if(row && row.column===retainedFragment.column && row.end_column<retainedFragment.end_column)sourceRows.set(row.number,retainedFragment);
    }
    data=window; entries=new Map(window.outline.map(row=>[row.index,row]));
    if(window.cursor)entries.set(window.cursor.index,window.cursor);
    cursor=window.cursor?.index ?? 0;
    renderSource();renderOutline();
    if(hadFocus)focusCursor();
    if(intent.end)source.scrollLeft=source.scrollWidth;
    if(queuedActivation && entries.has(cursor)) {queuedActivation=false;void select(entries.get(cursor));}
  }
  const reader=makeReader({transport,onWindow:accept,onError});
  function focusCursor() { const row=entries.get(cursor); if(row)outlineNodes.get(row.id)?.button.focus({preventScroll:true}); }
  function renderSource() {
    if(!view?.source || !view.source.summary.lines) {
      sourceNodes.clear();space.replaceChildren(emptyNote);emptyNote.textContent=view?.source?"Source encoding is unavailable in this inspector.":emptyText;return;
    }
    const first=Math.max(1,Math.floor(source.scrollTop/SOURCE_ROW)-16+1),last=Math.min(view.source.summary.lines+1,Math.ceil((source.scrollTop+source.clientHeight)/SOURCE_ROW)+17,first+96);
    for(const [number,node] of sourceNodes)if(number<first || number>=last || !sourceRows.has(number)){node.row.remove();sourceNodes.delete(number);}
    for(let number=first;number<last;number++) {
      const line=sourceRows.get(number);if(!line)continue;
      let node=sourceNodes.get(number);
      if(!node) {
        const row=document.createElement("div"),label=document.createElement("span"),text=document.createElement("span"),controls=document.createElement("span");row.className="code-line";label.className="line-no";label.setAttribute("aria-hidden","true");text.className="source-text";controls.className="fragment-controls";
        const buttons=new Map();
        for(const [action,title] of [["first","First"],["previous","Earlier"],["next","Later"],["last","Last"]]) {
          const button=document.createElement("button");button.textContent=title;button.setAttribute("aria-label",`${title} text on line ${number}`);button.addEventListener("click",()=>fragment(number,action));controls.append(button);buttons.set(action,button);
        }
        row.append(label,controls,text);space.append(row);node={row,label,text,buttons};sourceNodes.set(number,node);
      }
      node.row.style.top=`${(number-1)*SOURCE_ROW}px`;node.label.textContent=number;if(node.line!==line){renderStyledText(node.text,line);node.line=line;}
      const active=view.source.summary.active;node.row.classList.toggle("active",Boolean(active && number>=active.start_line && number<=active.end_line));
      decorateLine(node.row,number,view);
      for(const [action,button] of node.buttons)button.hidden=action==="first" || action==="previous" ? !line.left : !line.right;
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
    if(!data)return false;
    const lines=view.source.summary.lines,decls=view.source.summary.declarations;
    const lastLine=Math.min(lines,Math.ceil((source.scrollTop+source.clientHeight)/SOURCE_ROW)+16,query.line_start+95);
    const lastEntry=Math.min(decls,Math.ceil((outline.scrollTop+outline.clientHeight)/OUTLINE_ROW)+8,query.outline_start+199);
    for(let n=query.line_start;n<=lastLine;n++)if(!sourceRows.has(n))return false;
    for(let i=query.outline_start;i<lastEntry;i++)if(!entries.has(i))return false;
    return !decls || entries.has(cursor);
  }
  function schedule() {
    if(scheduled)return;scheduled=true;
    requestAnimationFrame(()=>{scheduled=false;if(!view?.source)return;const query=ranges();reader.activate(view);renderSource();renderOutline();if(!covered(query))void reader.request(query,{focus:false});});
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
    position() {
      return { sourceTop:source.scrollTop, sourceLeft:source.scrollLeft, outlineTop:outline.scrollTop,
        cursorIndex:cursor, columnLine, column };
    },
    async prepare(next,options={}) {
      if(!next.source)return {...next,inspection:null};
      const changedFile=view?.source?.id!==next.source.id,changedSelection=changedFile || view.selection.entity!==next.selection.entity;
      const sameDocument=!changedFile && view.source.version===next.source.version;
      const active=next.source.summary.active;
      const restore=options.restore;
      const reveal=Boolean(options.reveal);
      const desiredSourceTop=restore?restore.sourceTop:active && (changedSelection || reveal)?Math.max(0,(active.start_line-4)*SOURCE_ROW):changedFile || reveal?0:source.scrollTop;
      const sourceTop=Math.max(0,Math.min(desiredSourceTop,next.source.summary.lines*SOURCE_ROW-source.clientHeight));
      let outlineTop=restore?restore.outlineTop:changedFile || reveal?0:outline.scrollTop;
      if(!restore && (changedSelection || reveal) && active)outlineTop=Math.max(0,active.index*OUTLINE_ROW-outline.clientHeight/2);
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
      return {...next,inspection:window,inspectionPosition:{sourceTop,outlineTop,
        sourceLeft:restore?restore.sourceLeft:sameDocument && !reveal?source.scrollLeft:0,
        columnLine:restore?restore.columnLine:sameDocument && !reveal?columnLine:undefined,
        column:restore?restore.column:sameDocument && !reveal?column:0}};
    },
    apply(next) {
      const hadFocus=outline.contains(document.activeElement);
      const position=next.inspectionPosition;
      const retainedFragment=view?.source?.id===next.source?.id && view?.source?.version===next.source?.version?sourceRows.get(position?.columnLine):undefined;
      view=next;queuedActivation=false;reader.activate(view);
      if(!sourceNodes.size)space.replaceChildren();if(!outlineNodes.size)outlineSpace.replaceChildren();sizes();
      if(next.inspection){source.scrollTop=next.inspectionPosition.sourceTop;outline.scrollTop=next.inspectionPosition.outlineTop;accept(next.inspection,{},{},retainedFragment);columnLine=next.inspectionPosition.columnLine;column=next.inspectionPosition.column;source.scrollLeft=next.inspectionPosition.sourceLeft;}
      else {data=null;sourceRows=new Map();entries=new Map();cursor=0;columnLine=undefined;column=0;renderSource();renderOutline();}
      if(hadFocus && !view.source?.summary.declarations)outline.focus({preventScroll:true});
    },
    async revealActive() {
      const active=view?.source?.summary.active;if(!active)return;
      source.scrollTop=Math.max(0,(active.start_line-4)*SOURCE_ROW);revealOutline(active.index);
      const query=ranges();if(!covered(query))await reader.request(query,{});else {renderSource();renderOutline();}
    },
  };
}
