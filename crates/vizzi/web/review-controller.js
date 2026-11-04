import { validateReviewWindow } from "./review-windows.js";

const integer = value => Number.isSafeInteger(value) && value >= 0;
const opaque = value => typeof value === "string" && value.length > 0 && value.length <= 256;
const copy = value => structuredClone(value);
const params = value => new URLSearchParams(Object.entries(value).filter(([,v])=>v!=null));
const defaults = () => ({filter:"",unchanged:false,offset:0,side:"working",hunk_start:0,edit_start:0,mode:"hunk",entity:null,hunk:null,file:null});
function pair(metadata, response) {
  if(response?.schema!==1 || response.token!==metadata.token || response.comparison!==metadata.comparison)throw new Error("Mismatched comparison response");
  if(response.origin!==undefined && metadata.origin!==undefined && response.origin!==metadata.origin)throw new Error("Mismatched comparison origin");
  return response;
}
const sameOrigin = (before,after) => opaque(after?.origin) && after.origin===before?.origin;
function refreshSelection(input,files,previous,saved) {
  if(previous && (files.selected===undefined || (files.selected!==null && files.selected?.file!==previous.file)))throw new Error("Invalid refreshed file selection");
  const file=(previous?files.selected:null) ?? files.files[0] ?? null;
  const query={...input,file,entity:null,hunk:null,hunk_start:0,edit_start:0,mode:"file"},positions={},notices=[];
  if(!previous || file?.file!==previous.file) {
    query.side=file?.working?"working":"base";
    if(previous)notices.push(file?"Selected file is absent from the refreshed inventory. Showing the first available file.":"Selected file is absent from the refreshed inventory. No file matches the current view.");
    return {query,file,positions,notices};
  }
  const other=query.side==="base"?"working":"base";
  if(!file[query.side] && file[other]) {
    notices.push(`${query.side} side is absent; showing ${other}.`);query.side=other;
  }
  for(const side of ["base","working"]) {
    const version=file[`${side}_version`];
    if(opaque(version) && version===previous[`${side}_version`] && saved?.[side])positions[side]=copy(saved[side]);
  }
  if(input.mode==="declaration" && input.entity && query.side===input.side) {
    if(opaque(file[`${query.side}_version`])){query.entity=copy(input.entity);query.mode="declaration";}
    else notices.push("Saved declaration source is unavailable in the refreshed capture.");
  }
  for(const side of ["base","working"])if(file[`${side}_version`]!==previous[`${side}_version`] && !(query.entity && query.side===side))notices.push(`${side} source changed; showing file start.`);
  if(input.hunk)notices.push("Hunk selection cleared for the new comparison.");
  return {query,file,positions,notices};
}
function validateDiff(file,diff) {
  if(diff.file!==file.file || diff.base_version!==file.base_version || diff.working_version!==file.working_version || !Array.isArray(diff.hunks) || diff.hunks.length>200 || !Array.isArray(diff.edits) || diff.edits.length>200)throw new Error("Invalid review diff anchors or bounds");
  return diff;
}
function notification(change) {
  if(change?.schema!==1 || !opaque(change.stream) || typeof change.revision!=="string" || !/^(0|[1-9]\d{0,19})$/.test(change.revision) || BigInt(change.revision)>18446744073709551615n)throw new Error("Invalid review notification");
  const s=change.status;
  if(s?.schema!==1 || typeof s.pending!=="boolean" || !(s.ticket===null || opaque(s.ticket)) || !(s.error===null || (typeof s.error?.code==="string" && typeof s.error?.message==="string")))throw new Error("Invalid review status");
  const m=s.active;
  if(m!==null && (!m || ![m.token,m.comparison,m.provider,m.revision,m.resolved,m.scope].every(value=>typeof value==="string") || !opaque(m.token) || !opaque(m.comparison) || !integer(m.working_started_ms) || !integer(m.working_finished_ms)))throw new Error("Invalid review metadata");
  if(m?.origin!==undefined && !opaque(m.origin))throw new Error("Invalid comparison origin");
  return BigInt(change.revision);
}

export function createReviewController({transport,onView=()=>{},onStatus=()=>{},onNotice=()=>{},onError=()=>{},onHistory=()=>{},capture=()=>({}),history={push(){},replace(){},go(){}}}) {
  const session=crypto.randomUUID();
  let latest,view,request,fetch,subscription,loading;
  let visits=[],cursor=0,shown;
  const marker=key=>({vizziReview:{schema:1,session,key}});
  const historyState=()=>onHistory({canBack:cursor>0,canForward:cursor<visits.length-1,count:visits.length,pending:Boolean(loading)});
  function invalidate() {request={};fetch?.abort();fetch=undefined;loading=undefined;}
  function save() {const visit=visits.find(v=>v.key===shown);if(visit)visit.positions=copy(capture());}
  function record(returning) {
    const location={metadata:copy(view.metadata),query:copy(view.query),positions:copy(capture())};
    if(returning) {const visit=visits.find(v=>v.key===returning);Object.assign(visit,location);shown=returning;}
    else {
      const previous=visits.find(v=>v.key===shown);
      const same=previous && previous.metadata.token===view.metadata.token && previous.query.file?.file===view.file?.file && previous.query.side===view.selection.side && previous.query.hunk===view.selection.hunk && (previous.query.entity?.id ?? null)===view.selection.entity;
      if(same && visits[cursor]?.key===shown)Object.assign(previous,location);
      else {
        visits=visits.slice(0,cursor+1);
        const key=crypto.randomUUID();visits.push({key,...location});
        if(visits.length>50)visits.shift();
        cursor=visits.length-1;shown=key;
        if(visits.length===1)history.replace(marker(key));else history.push(marker(key));
      }
    }
    historyState();
  }
  async function load(metadata, input=defaults(), options={}) {
    const current={};invalidate();request=current;const abort=fetch=new AbortController();loading=metadata.token;historyState();
    let query=copy(input),positions=options.positions;
    const notices=[];
    const root=`/api/comparisons/${encodeURIComponent(metadata.token)}`;
    const alive=()=>{if(request!==current)throw new Error("Superseded review request");};
    const read=async(endpoint,arguments_)=>{const result=await transport(`${root}/${endpoint}?${params(arguments_)}`,{signal:abort.signal});alive();return pair(metadata,result);};
    try {
      const filePage=async()=>{
        const page=await read("files",{filter:query.filter,unchanged:query.unchanged,offset:query.offset,count:200,file:options.refresh?options.previous?.file:null});
        if(!Array.isArray(page.files) || page.files.length>200 || !integer(page.total))throw new Error("Invalid review file page");
        return page;
      };
      let files=await filePage();
      if(options.refresh && query.offset>=files.total && query.offset>0) {
        query.offset=Math.max(0,Math.floor((files.total-1)/200)*200);
        if(files.total>0)files=await filePage();
      }
      let file=query.file ?? files.files[0] ?? null;
      if(options.refresh) {
        const target=refreshSelection(query,files,options.previous,positions);
        ({query,file,positions}=target);notices.push(...target.notices);
      }
      let diff=null,diffError=null;const windows={base:null,working:null};
      if(file) {
        query.file=file;
        if(!opaque(file.file) || ![file.base_version,file.working_version].every(v=>v===null || opaque(v)))throw new Error("Invalid review file anchors");
        if(!file.working_version && file.base_version && !input.file)query.side="base";
        const getDiff=async()=>validateDiff(file,await read("diff",{file:file.file,base_version:file.base_version,working_version:file.working_version,hunk_start:query.hunk_start,hunk_count:200,edit_start:query.edit_start,edit_count:200}));
        try {
          diff=await getDiff();
          if(query.mode==="hunk") {
            const selected=query.hunk?diff.hunks.find(h=>h.id===query.hunk):diff.hunks[0];
            if(query.hunk && !selected)throw new Error("Saved hunk is unavailable");
            query.hunk=selected?.id ?? null;
            if(selected && !options.keepEditPage && (selected.edits.start<query.edit_start || selected.edits.start>=query.edit_start+diff.edits.length)) {
              query.edit_start=selected.edits.start;diff=await getDiff();
            }
          }
        } catch(error) {
          alive();if(![404,413,422,503].includes(error.status))throw error;
          diffError=error.message;query.hunk=null;
        }
        const hunk=diff?.hunks.find(h=>h.id===query.hunk);
        const readWindow=async(side,arguments_)=>{
          const version=file[`${side}_version`];
          const body=await read("window",{...arguments_,file:file.file,side,version});
          const summary={lines:body.window?.line_total,declarations:body.window?.outline_total};
          if(!integer(summary.lines) || !integer(summary.declarations))throw new Error("Invalid review window totals");
          validateReviewWindow({review:{...metadata,side},source:{id:file.file,version,summary}},body);
          return body;
        };
        if(options.refresh && query.entity && !positions?.[query.side]) {
          const body=await readWindow(query.side,{cursor:query.entity.id,line_count:1,outline_count:1});
          const entry=body.window.cursor;
          if(entry?.id===query.entity.id) {
            query.entity={id:entry.id,index:entry.index,start_line:entry.start_line};
            notices.push(`${query.side} source changed; showing the current declaration.`);
          } else {
            query.entity=null;query.mode="file";
            notices.push("Saved declaration is no longer available in this capture; showing file start.");
          }
        }
        for(const side of ["base","working"]) {
          const version=file[`${side}_version`];if(!version)continue;
          const position=positions?.[side];
          const entity=query.side===side?query.entity:null;
          const line=entity?Math.max(1,entity.start_line-3):hunk?hunk[side].lines.start+1:1;
          const sourceTop=position?.sourceTop ?? (line-1)*22;
          const arguments_={file:file.file,side,version,line_start:Math.max(1,Math.floor(sourceTop/22)-16+1),line_count:96,outline_start:position?Math.max(0,Math.floor(position.outlineTop/48)-8):entity?Math.max(0,entity.index-3):0,outline_count:200,cursor:entity?.id,cursor_index:position?.cursorIndex,column_line:position?.columnLine,column:position?.column};
          try {
            const body=await readWindow(side,arguments_);
            if(entity && body.window.cursor?.id!==entity.id)throw new Error("Saved declaration is unavailable");
            windows[side]={...body,position:position ?? {sourceTop,sourceLeft:0,outlineTop:entity?Math.max(0,entity.index-3)*48:0,cursorIndex:entity?.index ?? 0,column:0}};
          } catch(error) {
            alive();if(![404,422,503].includes(error.status))throw error;
            windows[side]={error:error.message};
          }
        }
      }
      alive();if(latest && latest.active?.token!==metadata.token)throw new Error("Comparison expired; open the current comparison");
      view={metadata,query,files,file,diff,diffError,windows,selection:{side:query.side,entity:query.entity?.id ?? null,hunk:query.hunk}};
      onView(view);record(options.returning);onNotice(options.refresh?notices.join(" ") || "Refreshed comparison; file and reading positions kept.":options.notice ?? null);return true;
    } catch(error) {
      if(request===current){onError(error);onNotice(`${error.message}. Keeping the displayed comparison; retry inspection or open the current comparison.`);}
      return false;
    } finally {if(request===current){fetch=undefined;loading=undefined;historyState();}}
  }
  async function mutate(uri,body,method="POST") {
    try {await transport(uri,{method,body:JSON.stringify(body),headers:{"Content-Type":"application/json"}});return true;}
    catch(error){onError(error);onNotice(error.message);return false;}
  }
  function navigate(patch={}) {
    if(!view)return Promise.resolve(false);
    if(latest && latest.active?.token!==view.metadata.token && loading) {
      onNotice("The new comparison is loading. Keeping the displayed capture until it is ready.");return Promise.resolve(false);
    }
    save();const query={...view.query,...patch},positions=copy(capture());
    if(patch.file && patch.file.file!==view.file?.file) {
      Object.assign(query,{entity:null,hunk:null,hunk_start:0,edit_start:0,mode:"hunk",side:patch.file.working_version?"working":"base"});delete positions.base;delete positions.working;
    }
    if("hunk" in patch || "hunk_start" in patch) {
      Object.assign(query,{entity:null,mode:"hunk"});if("hunk_start" in patch)query.hunk=null;
      delete positions.base;delete positions.working;
    }
    if("entity" in patch) {query.mode="declaration";query.hunk=null;delete positions[query.side];}
    if(latest && latest.active?.token!==view.metadata.token) {
      if(sameOrigin(view.metadata,latest.active))return load(latest.active,query,{positions,refresh:true,previous:copy(query.file)});
      onNotice("This comparison expired. Open the current comparison to continue.");return Promise.resolve(false);
    }
    return load(view.metadata,query,{positions,keepEditPage:"edit_start" in patch});
  }
  function updates() {
    invalidate();historyState();const token=subscription={};let stream,revision=-1n;
    return {
      receive(change) {
        if(subscription!==token)return false;
        const value=notification(change);
        if(stream && stream!==change.stream)throw new Error("Review stream changed without reconnecting");
        if(value<=revision)return false;
        stream=change.stream;revision=value;latest=change.status;onStatus(latest);
        const metadata=latest.active;
        if(!metadata){invalidate();historyState();if(view)onNotice("This comparison expired or was closed. The displayed capture is kept.");}
        else if(metadata.token!==view?.metadata.token && loading!==metadata.token) {
          save();
          if(sameOrigin(view?.metadata,metadata))void load(metadata,view.query,{refresh:true,previous:copy(view.file),positions:copy(capture())});
          else void load(metadata,defaults(),{notice:view?"New comparison capture. Reading positions reset; previous visits require their original comparison.":null});
        }
        return true;
      },
      close(error) {if(subscription===token){subscription=undefined;invalidate();historyState();if(error){onError(error);onNotice("Review updates disconnected. The displayed capture is kept; reconnecting.");}}},
    };
  }
  return {
    updates,navigate,
    create:({revision,scope})=>mutate("/api/comparisons",{provider:"git",revision,scope}),
    refresh:()=>latest?.active?mutate(`/api/comparisons/${encodeURIComponent(latest.active.token)}/refresh`,{}):Promise.resolve(false),
    async close() {if(!latest?.active)return false;if(!await mutate(`/api/comparisons/${encodeURIComponent(latest.active.token)}`,{},"DELETE"))return false;invalidate();view=null;onView(null);historyState();return true;},
    openCurrent() {save();return latest?.active?load(latest.active,defaults()):Promise.resolve(false);},
    move(delta) {if((delta===-1 || delta===1) && cursor+delta>=0 && cursor+delta<visits.length)history.go(delta);},
    async traverse(state) {
      const m=state?.vizziReview;if(!m)return false;save();
      if(!(view && loading && latest?.active?.token!==view.metadata.token))invalidate();
      const index=m.schema===1 && m.session===session?visits.findIndex(v=>v.key===m.key):-1;
      if(index<0){visits=[];cursor=0;shown=undefined;historyState();onNotice("This saved review location expired. The displayed capture is kept; open the current comparison.");return false;}
      cursor=index;historyState();const target=visits[index];
      if(target.metadata.token!==latest?.active?.token || target.metadata.comparison!==latest.active.comparison){onNotice("This saved comparison expired. The displayed capture is kept; open the current comparison.");return false;}
      return load(target.metadata,target.query,{positions:target.positions,returning:target.key,keepEditPage:true});
    },
  };
}
