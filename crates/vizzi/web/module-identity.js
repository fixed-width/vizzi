const opaque=s=>typeof s==='string'&&/^[a-f0-9]{64}$/.test(s);
function colour(module,views){
 const assigned=new Map(),used=new Set();
 for(const m of [...views.map(v=>v.symbol.module),module]){
  if(!m||!validModule(m)||assigned.has(m.id))continue;
  const preferred=parseInt(m.id.slice(-8),16)%6;
  const next=Array.from({length:6},(_,n)=>(preferred+n)%6).find(n=>!used.has(n))??preferred;
  assigned.set(m.id,next);used.add(next);
 }
 return assigned.get(module.id);
}
export function validModule(m){return m===null||m===undefined||Boolean(m&&opaque(m.id)&&typeof m.label==='string'&&m.label.length>0&&new TextEncoder().encode(m.label).length<=512&&typeof m.root==='string'&&m.root.length>0&&new TextEncoder().encode(m.root).length<=4096&&!m.root.includes('\\')&&!m.root.split('/').some(p=>!p||p==='.'||p==='..')&&((m.language==='rust'&&m.basis==='rust_declarations')||(m.language==='python'&&m.basis==='python_module')));}
export function modulePresentation(view,views){
 const m=view.symbol.module;if(!m||!validModule(m))return null;
 const peers=views.map(v=>v.symbol.module).filter(n=>n&&n.label===m.label&&n.id!==m.id);
 let label=m.label;
 if(peers.length){const parts=m.root.split('/');let suffix=m.root;for(let n=1;n<=parts.length;n++){const candidate=parts.slice(-n).join('/');if(peers.every(p=>!p.root.endsWith(candidate))){suffix=candidate;break;}}label=`${suffix} · ${label}`;}
 return {id:m.id,label,title:`${m.label}\n${m.language==='rust'?'Rust module':'Python module'}\nRoot: ${m.root}`,colour:colour(m,views)};
}
export function modulePeers(views,id){return new Set(views.filter(v=>v.symbol.module?.id===id).map(v=>v.id));}
export function moduleGeometryKey(views){return JSON.stringify(views.map(v=>[v.id,v.symbol.module?.id??null,v.x,v.y,v.w,v.collapsed?54:v.h]));}
