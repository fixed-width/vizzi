// Presentation state is independent of captured source and relationship facts.
export const LIMIT=24,MIN_SCALE=0.02,MAX_SCALE=1.5;
const bytes=s=>new TextEncoder().encode(s).length;
const opaque=s=>typeof s==='string'&&s.length===64&&!/[^0-9a-f]/.test(s);
const finite=(n,min,max)=>Number.isFinite(n)&&n>=min&&n<=max;
const integer=(n,min,max)=>Number.isSafeInteger(n)&&n>=min&&n<=max;
const require=(value,message)=>{if(!value)throw new Error(message);};
const clone=value=>structuredClone(value);
function source(s){return s&&typeof s.path==='string'&&bytes(s.path)<=4096&&!s.path.includes('\\')&&!s.path.split('/').some(p=>!p||p==='.'||p==='..')&&opaque(s.file)&&opaque(s.version)&&opaque(s.context)&&integer(s.line,1,1e8)&&integer(s.end_line,s.line,1e8);}
function symbol(s){return s&&opaque(s.id)&&typeof s.name==='string'&&bytes(s.name)>0&&bytes(s.name)<=128&&typeof s.kind==='string'&&bytes(s.kind)<=128&&Array.isArray(s.parents)&&s.parents.length<=16&&s.parents.every(p=>typeof p==='string'&&bytes(p)<=128)&&source(s.source);}
function geometry(v){return finite(v.x,-1e5,1e5)&&finite(v.y,-1e5,1e5)&&finite(v.w,320,960)&&finite(v.h,200,960);}
function reading(r,s){return r&&finite(r.top,0,1e7)&&finite(r.left,0,1e7)&&integer(r.section,s.line,s.end_line);}
function camera(c){return c&&finite(c.x,-1e5,1e5)&&finite(c.y,-1e5,1e5)&&finite(c.scale,MIN_SCALE,MAX_SCALE);}
function within(call,owner){return source(call)&&call.path===owner.path&&call.file===owner.file&&call.version===owner.version&&call.context===owner.context&&call.line>=owner.line&&call.end_line<=owner.end_line;}
function sameSymbol(a,b){return symbol(a)&&a.id===b.id&&a.name===b.name&&a.kind===b.kind&&JSON.stringify(a.parents)===JSON.stringify(b.parents)&&['path','file','version','context','line','end_line'].every(k=>a.source[k]===b.source[k]);}
const linkKey=(from,to,s,spelling)=>[from,to,s.file,s.line,spelling].join('|');
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
  const existing=this.views.find(v=>v.symbol.id===s.id);if(existing){this.selected=existing.id;return existing.id;}
  require(this.views.length<LIMIT,'Close a fragment before opening more views.');
  const w=440,h=400,p=this.place(parent?parent.x+parent.w+72:64,parent?parent.y:64,w,h);
  const v={id:'v'+this.next,kind:'source',symbol:clone(s),area,x:p.x,y:p.y,w,h,collapsed:false,reading:{section:s.source.line,top:0,left:0}};
  require(geometry(v),'Workspace exceeds its coordinate allowance');this.next++;this.views.push(v);this.selected=v.id;return v.id;
 }
 openRoot(s,area){this.fresh();return this.add(s,area,null);}
 follow(parentId,site,target){
  this.fresh();const parent=this.view(parentId),s=parent.symbol.source;
  require(site&&site.owner===parent.symbol.id&&within(site.source,s)&&typeof site.spelling==='string'&&bytes(site.spelling)>0&&bytes(site.spelling)<=256&&Array.isArray(site.candidates)&&site.candidates.some(c=>sameSymbol(target,c)),'Call witness differs from the visible caller');
  require(site.binding==null||(site.binding==='rust_self_receiver'&&site.candidates.length===1&&site.candidate_count===1),'Invalid source-local binding');
  const existing=this.views.find(v=>v.symbol.id===target.id),key=linkKey(parentId,existing?.id??'v'+this.next,site.source,site.spelling);
  require(this.links.length<96||this.links.some(l=>l.id===key),'Workspace link allowance reached');const to=this.add(target,parent.area,parent);
  if(!this.links.some(l=>l.id===key))this.links.push({id:key,from:parentId,to,kind:site.binding==='rust_self_receiver'?'self':'candidate',spelling:site.spelling,source:clone(site.source)});
  return to;
 }
 move(id,x,y){require(finite(x,-1e5,1e5)&&finite(y,-1e5,1e5),'Invalid fragment position');const v=this.view(id);v.x=x;v.y=y;}
 resize(id,w,h){require(finite(w,320,960)&&finite(h,200,960),'Invalid fragment size');const v=this.view(id);v.w=w;v.h=h;}
 read(id,r){const v=this.view(id);require(reading(r,v.symbol.source),'Invalid fragment reading state');v.reading=clone(r);}
 collapse(id){const v=this.view(id);v.collapsed=!v.collapsed;this.selected=id;}
 close(id){this.view(id);this.views=this.views.filter(v=>v.id!==id);this.links=this.links.filter(l=>l.from!==id&&l.to!==id);if(this.selected===id)this.selected=this.views.at(-1)?.id??null;}
 pan(dx,dy){require(finite(dx,-1e5,1e5)&&finite(dy,-1e5,1e5),'Invalid camera movement');this.camera.x=Math.max(-1e5,Math.min(1e5,this.camera.x-dx/this.camera.scale));this.camera.y=Math.max(-1e5,Math.min(1e5,this.camera.y-dy/this.camera.scale));}
 zoomAt(scale,point){require(Number.isFinite(scale)&&finite(point.x,-1e5,1e5)&&finite(point.y,-1e5,1e5),'Invalid zoom');const p=worldPoint(point,this.camera);const next=Math.max(MIN_SCALE,Math.min(MAX_SCALE,scale));this.camera={x:Math.max(-1e5,Math.min(1e5,p.x-point.x/next)),y:Math.max(-1e5,Math.min(1e5,p.y-point.y/next)),scale:next};}
 reveal(ids,width,height,minimum=0.65){
  require(finite(width,1,1e5)&&finite(height,1,1e5),'Invalid viewport');const views=ids.map(id=>this.view(id));if(!views.length){this.camera={x:0,y:0,scale:1};return;}
  const b=bounds(views),scale=Math.max(MIN_SCALE,Math.min(1,Math.max(minimum,Math.min((width-64)/(b.right-b.x),(height-64)/(b.bottom-b.y)))));
  this.camera={x:(b.x+b.right)/2-width/(2*scale),y:(b.y+b.bottom)/2-height/(2*scale),scale};
 }
 fit(width,height){this.reveal(this.views.map(v=>v.id),width,height,MIN_SCALE);}
 serialize(){return JSON.stringify({schema:1,snapshot:this.snapshot,next:this.next,selected:this.selected,views:this.views,links:this.links,camera:this.camera});}
 static restore(raw,currentSnapshot){
  require(typeof raw==='string'&&bytes(raw)<=524288,'Invalid visual workspace size');let data;try{data=JSON.parse(raw);}catch{throw new Error('Invalid visual workspace JSON');}
  require(data&&data.schema===1&&opaque(data.snapshot)&&integer(data.next,1,1e6)&&Array.isArray(data.views)&&data.views.length<=LIMIT&&Array.isArray(data.links)&&data.links.length<=96&&camera(data.camera),'Invalid visual workspace document');
  const ids=new Set(),symbols=new Set();for(const v of data.views){require(v&&typeof v.id==='string'&&/^v[1-9][0-9]{0,5}$/.test(v.id)&&!ids.has(v.id)&&v.kind==='source'&&symbol(v.symbol)&&!symbols.has(v.symbol.id)&&typeof v.area==='string'&&bytes(v.area)<=4096&&geometry(v)&&typeof v.collapsed==='boolean'&&reading(v.reading,v.symbol.source),'Invalid restored fragment');ids.add(v.id);symbols.add(v.symbol.id);require(Number(v.id.slice(1))<data.next,'Invalid next view identity');}
  require(data.selected===null||ids.has(data.selected),'Invalid selected fragment');const linkIds=new Set();for(const l of data.links)require(l&&typeof l.id==='string'&&!linkIds.has(l.id)&&ids.has(l.from)&&ids.has(l.to)&&(l.kind==='candidate'||l.kind==='self')&&typeof l.spelling==='string'&&bytes(l.spelling)>0&&bytes(l.spelling)<=256&&within(l.source,data.views.find(v=>v.id===l.from).symbol.source)&&l.id===linkKey(l.from,l.to,l.source,l.spelling)&&(linkIds.add(l.id),true),'Invalid restored link');
  const model=new Workspace(data.snapshot);model.views=data.views;model.links=data.links;model.camera=data.camera;model.next=data.next;model.selected=data.selected;model.markSnapshot(currentSnapshot);return model;
 }
}
