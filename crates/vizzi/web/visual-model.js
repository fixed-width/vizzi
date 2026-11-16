import {validOrigin} from './source-origin.js';
import {validModule} from './module-identity.js';
import {fitViews,BUBBLE_SIZE} from './layout-growth.js';
// Presentation state is independent of captured source and relationship facts.
export const LIMIT=24,MIN_SCALE=0.02,MAX_SCALE=1.5;
const bytes=s=>new TextEncoder().encode(s).length;
const opaque=s=>typeof s==='string'&&s.length===64&&!/[^0-9a-f]/.test(s);
const finite=(n,min,max)=>Number.isFinite(n)&&n>=min&&n<=max;
const integer=(n,min,max)=>Number.isSafeInteger(n)&&n>=min&&n<=max;
const require=(value,message)=>{if(!value)throw new Error(message);};
const clone=value=>structuredClone(value);
function source(s){return s&&typeof s.path==='string'&&bytes(s.path)<=4096&&!s.path.includes('\\')&&!s.path.split('/').some(p=>!p||p==='.'||p==='..')&&opaque(s.file)&&opaque(s.version)&&opaque(s.context)&&integer(s.line,1,1e8)&&integer(s.end_line,s.line,1e8);}
function symbol(s){return s&&validOrigin(s.origin)&&validModule(s.module)&&(s.tracking==null||opaque(s.tracking))&&opaque(s.id)&&typeof s.name==='string'&&bytes(s.name)>0&&bytes(s.name)<=128&&typeof s.kind==='string'&&bytes(s.kind)<=128&&Array.isArray(s.parents)&&s.parents.length<=16&&s.parents.every(p=>typeof p==='string'&&bytes(p)<=128)&&source(s.source);}
const commit=s=>typeof s==='string'&&/^(?:[0-9a-f]{40}|[0-9a-f]{64})$/.test(s);
function history(h){return h&&opaque(h.token)&&commit(h.tip)&&commit(h.commit)&&integer(h.index,0,63)&&(h.returning==null||typeof h.returning==='boolean')&&h.base&&!Object.hasOwn(h.base,'history')&&opaque(h.base.snapshot)&&symbol(h.base.symbol)&&!h.base.symbol.origin&&reading(h.base.reading,h.base.symbol.source);}
const label=s=>typeof s==='string'&&bytes(s)>0&&bytes(s)<=256&&!/[\0\r\n]/.test(s);
function comparison(c,s){return c&&opaque(c.token)&&['older','newer'].includes(c.side)&&label(c.label)&&label(c.other_label)&&(c.returning==null||typeof c.returning==='boolean')&&Array.isArray(c.changes)&&c.changes.length<=1024&&c.changes.every((r,i)=>Array.isArray(r)&&r.length===2&&integer(r[0],s.line,s.end_line+1)&&integer(r[1],r[0],s.end_line+1)&&(i===0||c.changes[i-1][1]<=r[0]))&&integer(c.cursor,0,Math.max(0,c.changes.length-1));}
function capturedOwner(o){return o&&opaque(o.token)&&(o.kind==='history'&&integer(o.index,0,63)&&Object.keys(o).every(k=>['kind','token','index'].includes(k))||o.kind==='comparison'&&['older','newer'].includes(o.side)&&Object.keys(o).every(k=>['kind','token','side'].includes(k)));}
function captured(c){return c&&capturedOwner(c.owner)&&label(c.label)&&(c.returning==null||typeof c.returning==='boolean');}
export function frozenSource(v){const owner=v.comparison?{kind:'comparison',token:v.comparison.token,side:v.comparison.side}:v.captured?.owner??(v.history?{kind:'history',token:v.history.token,index:v.history.index}:null);return owner?{owner,snapshot:v.snapshot,file:v.symbol.source.file,version:v.symbol.source.version,symbol:v.symbol.id}:null;}
export function frozenLabel(v){return v.comparison?.label??v.captured?.label??v.history?.commit??'';}
function frozenKey(v){const s=frozenSource(v),o=s?.owner;return s?JSON.stringify([o.kind,o.token,o.kind==='history'?o.index:o.side,s.snapshot,s.file,s.version]):null;}
function retainedCapturedLink(l,views){const from=views.find(v=>v.id===l.from),to=views.find(v=>v.id===l.to);return from&&to&&from.mode==='pinned'&&to.mode==='pinned'&&from.state==='current'&&to.state==='current'&&frozenKey(from)===l.captured_scope&&frozenKey(to)===l.captured_scope&&within(l.source,from.symbol.source);}
function geometry(v){return finite(v.x,-1e5,1e5)&&finite(v.y,-1e5,1e5)&&finite(v.w,320,BUBBLE_SIZE.maxWidth)&&finite(v.h,200,BUBBLE_SIZE.maxHeight);}
function reading(r,s){return r&&finite(r.top,0,1e7)&&finite(r.left,0,1e7)&&integer(r.section,s.line,s.end_line)&&(r.anchor==null||(integer(r.anchor.line,r.section,Math.min(s.end_line,r.section+95))&&integer(r.anchor.byte,0,32768)&&finite(r.anchor.offset,-1024,1024)));}
function camera(c){return c&&finite(c.x,-1e5,1e5)&&finite(c.y,-1e5,1e5)&&finite(c.scale,MIN_SCALE,MAX_SCALE);}
function within(call,owner){return source(call)&&call.path===owner.path&&call.file===owner.file&&call.version===owner.version&&call.context===owner.context&&call.line>=owner.line&&call.end_line<=owner.end_line;}
function sameSymbol(a,b){return symbol(a)&&a.id===b.id&&a.name===b.name&&a.kind===b.kind&&JSON.stringify(a.parents)===JSON.stringify(b.parents)&&['path','file','version','context','line','end_line'].every(k=>a.source[k]===b.source[k]);}
const linkKey=(from,to,s,spelling,context)=>[from,to,s.file,s.line,spelling,...(context?[context]:[])].join('|');
function bounds(views){return{x:Math.min(...views.map(v=>v.x)),y:Math.min(...views.map(v=>v.y)),right:Math.max(...views.map(v=>v.x+v.w)),bottom:Math.max(...views.map(v=>v.y+(v.collapsed?54:v.h)))};}
export function worldPoint(point,c){return{x:point.x/c.scale+c.x,y:point.y/c.scale+c.y};}
export class Workspace {
 constructor(snapshot){require(opaque(snapshot),'Invalid source snapshot');this.snapshot=snapshot;this.views=[];this.links=[];this.camera={x:0,y:0,scale:1};this.next=1;this.selected=null;this.stale=false;}
 view(id){const v=this.views.find(v=>v.id===id);require(v,'Fragment view is absent');return v;}
 fresh(){require(!this.stale,'Source changed; start a current workspace before following calls.');}
 markSnapshot(snapshot){this.stale=snapshot!==this.snapshot;}
 place(x,y,w,h){
  for(const v of this.views.toSorted((a,b)=>a.y-b.y)){
   if(x<v.x+v.w+24&&v.x<x+w+24&&y<v.y+(v.collapsed?54:v.h)+24&&v.y<y+h+24)y=v.y+(v.collapsed?54:v.h)+32;
  }
  return{x,y};
 }
 add(s,area,parent){
  require(symbol(s),'Invalid captured symbol');require(typeof area==='string'&&bytes(area)<=4096,'Invalid source area');
  const existing=this.views.find(v=>v.symbol.id===s.id&&(!s.origin||v.snapshot===this.snapshot));if(existing){if(s.origin&&existing.state==='unresolved'){existing.state='current';existing.symbol=clone(s);}this.selected=existing.id;return existing.id;}
  require(this.views.length<LIMIT,'Close a fragment before opening more views.');
  const w=440,h=400,p=this.place(parent?parent.x+parent.w+72:64,parent?parent.y:64,w,h);
  const v={id:'v'+this.next,kind:'source',snapshot:this.snapshot,mode:parent?.mode??'live',state:'current',symbol:clone(s),area,x:p.x,y:p.y,w,h,collapsed:false,reading:{section:s.source.line,top:0,left:0}};
  require(geometry(v),'Workspace exceeds its coordinate allowance');this.next++;this.views.push(v);this.selected=v.id;return v.id;
 }
 openRoot(s,area){this.fresh();return this.add(s,area,null);}
 follow(parentId,site,target){
  this.fresh();require(this.usable(parentId),'This fragment retains an earlier source version');const parent=this.view(parentId),s=parent.symbol.source;
  require(site&&site.owner===parent.symbol.id&&within(site.source,s)&&typeof site.spelling==='string'&&bytes(site.spelling)>0&&bytes(site.spelling)<=256&&Array.isArray(site.candidates)&&site.candidates.some(c=>sameSymbol(target,c)),'Call witness differs from the visible caller');
  require(site.binding==null||(site.binding==='rust_self_receiver'&&site.candidates.length===1&&site.candidate_count===1)||(site.binding==='language_server_definition'&&opaque(site.semantic_context)&&integer(site.candidate_count,site.candidates.length,64)),'Invalid definition binding');
  require(site.binding==='language_server_definition'||site.semantic_context==null,'Unexpected semantic context');
  const existing=this.views.find(v=>v.symbol.id===target.id&&(!target.origin||v.snapshot===this.snapshot)),key=linkKey(parentId,existing?.id??'v'+this.next,site.source,site.spelling,site.binding==='language_server_definition'?site.semantic_context:null);
  require(this.links.length<96||this.links.some(l=>l.id===key),'Workspace link allowance reached');const to=this.add(target,parent.area,parent);
  const fresh={id:key,from:parentId,to,kind:site.binding==='rust_self_receiver'?'self':site.binding==='language_server_definition'?'definition':'candidate',...(site.binding==='language_server_definition'?{semantic_context:site.semantic_context}:{}),spelling:site.spelling,source:clone(site.source)};
  const previous=this.links.filter(l=>l.from===parentId&&l.to===to&&l.spelling===site.spelling&&l.state!=null&&l.state!=='current');
  const exact=this.links.findIndex(l=>l.id===key);if(exact>=0){if(this.links[exact].state!=null&&this.links[exact].state!=='current')this.links[exact]={...fresh,state:'current'};}else if(previous.length===1)this.links[this.links.indexOf(previous[0])]={...fresh,state:'current'};else this.links.push(fresh);
  return to;
 }
 usable(id){const v=this.view(id);return !v.captured&&!v.comparison&&!v.history&&!this.stale&&v.snapshot===this.snapshot&&v.state==='current';}
 navigable(id){const v=this.view(id);return this.usable(id)||Boolean(frozenSource(v)&&v.mode==='pinned'&&v.state==='current');}
 pin(id,pinned){const v=this.view(id);if((v.history||v.comparison||v.captured)&&!pinned)this.returnLive(id);else v.mode=pinned?'pinned':'live';}
 followCaptured(parentId,site,frame){
  const parent=this.view(parentId),scope=frozenKey(parent),target=frame?.symbol;
  require(this.navigable(parentId)&&scope&&opaque(frame?.snapshot)&&symbol(target)&&!target.origin&&captured(frame.captured),'Captured destination unavailable');
  const binding={snapshot:frame.snapshot,symbol:target,captured:frame.captured};require(frozenKey(binding)===scope&&frame.captured.label===frozenLabel(parent),'Captured revision differs');
  require(site&&site.owner===parent.symbol.id&&within(site.source,parent.symbol.source)&&typeof site.spelling==='string'&&bytes(site.spelling)>0&&bytes(site.spelling)<=256&&Array.isArray(site.candidates)&&site.candidates.some(candidate=>sameSymbol(candidate,target)),'Captured call witness differs');
  require(site.semantic_context==null&&(site.binding==null||site.binding==='rust_self_receiver'&&site.candidate_count===1&&site.candidates.length===1),'Invalid captured call evidence');
  const existing=this.views.find(v=>v.symbol.id===target.id&&frozenKey(v)===scope&&v.mode==='pinned'),newId=existing?.id??'v'+this.next,key=linkKey(parentId,newId,site.source,site.spelling,scope);
  require(this.links.length<96||this.links.some(l=>l.id===key),'Workspace link allowance reached');
  let added;
  if(!existing){require(this.views.length<LIMIT&&this.next<1e6,'Close a fragment before opening more views.');const position=this.place(parent.x+parent.w+72,parent.y,440,400);added={id:newId,kind:'source',snapshot:frame.snapshot,mode:'pinned',state:'current',symbol:clone(target),captured:clone(frame.captured),area:parent.area,...position,w:440,h:400,collapsed:false,reading:{section:target.source.line,top:0,left:0}};require(geometry(added),'Workspace exceeds its coordinate allowance');}
  const link={id:key,from:parentId,to:newId,kind:site.binding==='rust_self_receiver'?'self':'candidate',state:'captured',spelling:site.spelling,source:clone(site.source),captured_scope:scope,captured_label:frame.captured.label};
  if(added){this.views.push(added);this.next++;}const index=this.links.findIndex(l=>l.id===key);if(index>=0)this.links[index]=link;else this.links.push(link);this.selected=newId;return newId;
 }
 openComparison(id,pair){
  const v=this.view(id);require(!v.captured&&!v.comparison&&opaque(pair?.token)&&this.views.length<LIMIT&&this.next<1e6,'Close a comparison or bubble before comparing');
  const base=v.history?.base??{snapshot:v.snapshot,symbol:clone(v.symbol),reading:clone(v.reading)};
  const bind=(original,frame,side)=>{
   require(frame&&opaque(frame.snapshot)&&symbol(frame.symbol)&&frame.symbol.kind==='function'&&!frame.symbol.origin&&frame.symbol.source.file===v.symbol.source.file&&frame.symbol.tracking===v.symbol.tracking,'Comparison declaration differs');
   const c={token:pair.token,side,label:frame.label,other_label:frame.other_label,changes:clone(frame.changes),cursor:0};require(comparison(c,frame.symbol.source),'Invalid comparison changes');
   const next={...clone(original),snapshot:frame.snapshot,symbol:clone(frame.symbol),mode:'pinned',state:'current',comparison:c,reading:{section:frame.window?.line_start,top:0,left:original.reading.left}};
   require(reading(next.reading,next.symbol.source),'Invalid comparison reading position');
   if(frame.history!=null){next.history={...clone(frame.history),base:clone(base)};require(history(next.history),'Invalid comparison history');}else delete next.history;
   return next;
  };
  const w=440,h=400,position=this.place(v.x+v.w+72,v.y,w,h),newId='v'+this.next;
  const newer=bind(v,pair.newer,'newer'),older=bind({...v,id:newId,...position,w,h,collapsed:false},pair.older,'older');
  require(geometry(older)&&newer.comparison.changes.length===older.comparison.changes.length,'Invalid comparison pair');
  this.views=this.views.map(view=>view.id===id?newer:view);this.views.push(older);this.next++;this.selected=id;
  for(const link of this.links)if(link.from===id||link.to===id)link.state='unverified';
  return {newer:id,older:newId};
 }
 comparisonEdit(id,direction){
  const c=this.view(id).comparison;require(c&&!c.returning&&[-1,1].includes(direction),'Comparison navigation unavailable');const cursor=c.cursor+direction;require(integer(cursor,0,c.changes.length-1),'No further comparison edit');
  const targets=this.views.filter(v=>v.comparison?.token===c.token&&!v.comparison.returning);
  require(targets.every(v=>cursor<v.comparison.changes.length),'Comparison edits differ');
  for(const v of targets){const s=v.symbol.source;v.comparison.cursor=cursor;v.reading={section:Math.max(s.line,Math.min(s.end_line,v.comparison.changes[cursor][0]-3)),top:0,left:v.reading.left};}
  return targets.map(v=>v.id);
 }
 showHistory(id,revision){
  const v=this.view(id),base=v.history?.base??{snapshot:v.snapshot,symbol:clone(v.symbol),reading:clone(v.reading)};
  const h={token:revision.token,tip:revision.tip,commit:revision.commit,index:revision.index,base};
  require(history(h)&&opaque(revision.snapshot)&&symbol(revision.symbol)&&!revision.symbol.origin&&revision.symbol.source.file===base.symbol.source.file&&revision.symbol.tracking===base.symbol.tracking,'Invalid function revision');
  const offset=v.reading.section-v.symbol.source.line,s=revision.symbol.source;
  delete v.comparison;
  delete v.captured;
  v.history=clone(h);v.snapshot=revision.snapshot;v.symbol=clone(revision.symbol);v.mode='pinned';v.state='current';v.reading={section:Math.min(s.end_line,s.line+offset),top:0,left:v.reading.left};
  for(const link of this.links)if(link.from===id||link.to===id)link.state='unverified';
 }
 returnLive(id){
  const v=this.view(id);require(history(v.history)||comparison(v.comparison,v.symbol.source)||captured(v.captured),'Function capture is absent');
  if(v.history)v.history.returning=true;if(v.comparison)v.comparison.returning=true;if(v.captured)v.captured.returning=true;v.mode='live';v.state='unresolved';
  for(const link of this.links)if(link.state==='captured'&&(link.from===id||link.to===id))link.state='unverified';
 }
 refresh(snapshot,updates,checks){
  require(opaque(snapshot)&&Array.isArray(updates)&&Array.isArray(checks),'Invalid live refresh');const byId=new Map();
  for(const update of updates){require(update&&typeof update.id==='string'&&!byId.has(update.id),'Duplicate live view');byId.set(update.id,update);}
  const live=this.views.filter(v=>v.mode==='live');require(byId.size===live.length&&live.every(v=>byId.has(v.id)),'Live working set differs');
  const views=this.views.map(v=>{if(v.mode==='pinned')return clone(v);const update=byId.get(v.id);require(['current','unresolved'].includes(update.state),'Invalid live state');if(update.state==='unresolved')return {...clone(v),state:'unresolved'};
   require(symbol(update.symbol),'Invalid current declaration');const p=update.position,s=update.symbol.source;require(p&&integer(p.section,s.line,s.end_line)&&typeof p.reset==='boolean'&&(p.line===null||integer(p.line,p.section,Math.min(s.end_line,p.section+95)))&&integer(p.byte,0,32768),'Invalid live reading position');
   const next={...clone(v),snapshot,symbol:clone(update.symbol),state:'current',reading:{section:p.section,top:v.symbol.source.version===s.version?v.reading.top:0,left:v.reading.left,anchor:p.line===null?null:{line:p.line,byte:p.byte,offset:v.reading.anchor?.offset??0}}};delete next.history;delete next.comparison;delete next.captured;return next;});
  const refreshed=new Map(checks.map(c=>[c.id,c.check]));require(checks.length===this.links.length&&refreshed.size===checks.length&&this.links.every(l=>refreshed.has(l.id)),'Live relationships differ');const links=[],ids=new Set();
  for(const old of this.links){const check=refreshed.get(old.id);let link={...clone(old),state:check?.state==='removed'?'removed':'unverified'};if(old.captured_scope){link.state=retainedCapturedLink(old,views)?'captured':'unverified';}else if(check?.state==='current'){
   const from=views.find(v=>v.id===old.from),to=views.find(v=>v.id===old.to);require(from&&to&&from.snapshot===snapshot&&to.snapshot===snapshot&&from.state==='current'&&to.state==='current'&&within(check.source,from.symbol.source)&&['self','candidate'].includes(check.kind),'Invalid refreshed relationship');
   link={...link,kind:check.kind,source:clone(check.source),state:'current'};delete link.semantic_context;link.id=linkKey(link.from,link.to,link.source,link.spelling,null);
  }else require(check===null||['removed','unverified'].includes(check?.state),'Invalid relationship status');
  if(!ids.has(link.id)){links.push(link);ids.add(link.id);}}
  this.views=views;this.links=links;this.snapshot=snapshot;this.stale=false;
 }
 move(id,x,y){require(finite(x,-1e5,1e5)&&finite(y,-1e5,1e5),'Invalid fragment position');const v=this.view(id);v.x=x;v.y=y;}
 resize(id,w,h){require(finite(w,320,BUBBLE_SIZE.maxWidth)&&finite(h,200,BUBBLE_SIZE.maxHeight),'Invalid fragment size');const v=this.view(id);v.w=w;v.h=h;}
 read(id,r){const v=this.view(id);require(reading(r,v.symbol.source),'Invalid fragment reading state');v.reading=clone(r);}
 fitSize(id,axis,extent){this.views=fitViews(this.views,id,axis,extent);this.selected=id;}
 collapse(id){const v=this.view(id);if(v.collapsed)this.fitSize(id,'height',v.h);else v.collapsed=true;this.selected=id;}
 close(id){this.view(id);this.views=this.views.filter(v=>v.id!==id);this.links=this.links.filter(l=>l.from!==id&&l.to!==id);if(this.selected===id)this.selected=this.views.at(-1)?.id??null;}
 pan(dx,dy){require(finite(dx,-1e5,1e5)&&finite(dy,-1e5,1e5),'Invalid camera movement');this.camera.x=Math.max(-1e5,Math.min(1e5,this.camera.x-dx/this.camera.scale));this.camera.y=Math.max(-1e5,Math.min(1e5,this.camera.y-dy/this.camera.scale));}
 zoomAt(scale,point){require(Number.isFinite(scale)&&finite(point.x,-1e5,1e5)&&finite(point.y,-1e5,1e5),'Invalid zoom');const p=worldPoint(point,this.camera);const next=Math.max(MIN_SCALE,Math.min(MAX_SCALE,scale));this.camera={x:Math.max(-1e5,Math.min(1e5,p.x-point.x/next)),y:Math.max(-1e5,Math.min(1e5,p.y-point.y/next)),scale:next};}
 reveal(ids,width,height){
  require(finite(width,1,1e5)&&finite(height,1,1e5),'Invalid viewport');const views=ids.map(id=>this.view(id));if(!views.length)return;
  const scale=this.camera.scale,pad=24/scale,w=width/scale,h=height/scale,all=bounds(views),b=all.right-all.x+2*pad<=w&&all.bottom-all.y+2*pad<=h?all:bounds([views.at(-1)]);
  const position=(start,end,current,span)=>Math.max(-1e5,Math.min(1e5,Math.min(start-pad,Math.max(current,end+pad-span))));
  this.camera={x:position(b.x,b.right,this.camera.x,w),y:position(b.y,b.bottom,this.camera.y,h),scale};
 }
 fit(width,height){
  require(finite(width,1,1e5)&&finite(height,1,1e5),'Invalid viewport');if(!this.views.length){this.camera={x:0,y:0,scale:1};return;}
  const b=bounds(this.views),scale=Math.max(MIN_SCALE,Math.min(1,(width-64)/(b.right-b.x),(height-64)/(b.bottom-b.y)));
  this.camera={x:(b.x+b.right)/2-width/(2*scale),y:(b.y+b.bottom)/2-height/(2*scale),scale};
 }
 serialize(){return JSON.stringify({schema:1,snapshot:this.snapshot,next:this.next,selected:this.selected,views:this.views,links:this.links,camera:this.camera});}
 static restore(raw,currentSnapshot){
  require(typeof raw==='string'&&bytes(raw)<=524288,'Invalid visual workspace size');let data;try{data=JSON.parse(raw);}catch{throw new Error('Invalid visual workspace JSON');}
  require(data&&data.schema===1&&opaque(data.snapshot)&&integer(data.next,1,1e6)&&Array.isArray(data.views)&&data.views.length<=LIMIT&&Array.isArray(data.links)&&data.links.length<=96&&camera(data.camera),'Invalid visual workspace document');
  const ids=new Set();for(const v of data.views){if(v){v.snapshot??=data.snapshot;v.mode??='live';v.state??='current';}require(v&&opaque(v.snapshot)&&['live','pinned'].includes(v.mode)&&['current','unresolved'].includes(v.state)&&typeof v.id==='string'&&/^v[1-9][0-9]{0,5}$/.test(v.id)&&!ids.has(v.id)&&v.kind==='source'&&symbol(v.symbol)&&typeof v.area==='string'&&bytes(v.area)<=4096&&geometry(v)&&typeof v.collapsed==='boolean'&&reading(v.reading,v.symbol.source)&&(v.history==null||history(v.history)&&(v.mode==='pinned'&&!v.history.returning||v.mode==='live'&&v.history.returning===true&&v.state==='unresolved')&&!v.symbol.origin&&v.symbol.source.file===v.history.base.symbol.source.file),'Invalid restored fragment');ids.add(v.id);require(Number(v.id.slice(1))<data.next,'Invalid next view identity');}
  for(const v of data.views)if(v.comparison!=null)require(comparison(v.comparison,v.symbol.source)&&!v.symbol.origin&&(v.mode==='pinned'&&!v.comparison.returning||v.mode==='live'&&v.comparison.returning===true&&v.state==='unresolved'),'Invalid restored comparison');
  for(const v of data.views)if(v.captured!=null)require(captured(v.captured)&&!v.history&&!v.comparison&&!v.symbol.origin&&(v.mode==='pinned'&&!v.captured.returning||v.mode==='live'&&v.captured.returning===true&&v.state==='unresolved'),'Invalid retained capture');
  require(data.selected===null||ids.has(data.selected),'Invalid selected fragment');const linkIds=new Set();for(const l of data.links)require(l&&typeof l.id==='string'&&!linkIds.has(l.id)&&ids.has(l.from)&&ids.has(l.to)&&(((l.kind==='candidate'||l.kind==='self')&&l.semantic_context==null)||(l.kind==='definition'&&opaque(l.semantic_context)))&&typeof l.spelling==='string'&&bytes(l.spelling)>0&&bytes(l.spelling)<=256&&(l.state==null||['current','removed','unverified','captured'].includes(l.state))&&(l.state==null||l.state==='current'?within(l.source,data.views.find(v=>v.id===l.from).symbol.source):source(l.source)&&l.source.file===data.views.find(v=>v.id===l.from).symbol.source.file&&l.source.path===data.views.find(v=>v.id===l.from).symbol.source.path)&&l.id===linkKey(l.from,l.to,l.source,l.spelling,l.kind==='definition'?l.semantic_context:l.captured_scope??null)&&(linkIds.add(l.id),true),'Invalid restored link');
  for(const l of data.links){if(l.captured_scope!=null)require(typeof l.captured_scope==='string'&&bytes(l.captured_scope)<=512&&label(l.captured_label)&&l.kind!=='definition'&&l.state!=='current','Invalid captured relationship');if(l.state==='captured')require(l.captured_scope&&retainedCapturedLink(l,data.views),'Captured relationship differs from its views');}
  const model=new Workspace(data.snapshot);model.views=data.views;model.links=data.links;model.camera=data.camera;model.next=data.next;model.selected=data.selected;model.markSnapshot(currentSnapshot);return model;
 }
}
