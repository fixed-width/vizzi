const opaque=s=>typeof s==='string'&&/^[a-f0-9]{64}$/.test(s);
const encoder = new TextEncoder();
let moduleKinds = [];

export function configureModuleKinds(kinds) {
 moduleKinds = kinds;
}

function moduleKind(module) {
 if (!module || !opaque(module.id) ||
     typeof module.label !== 'string' || !module.label || encoder.encode(module.label).length > 512 ||
     typeof module.root !== 'string' || !module.root || encoder.encode(module.root).length > 4096 ||
     module.root.includes('\\') || (module.root !== '.' && module.root.split('/').some(part => !part || part === '.' || part === '..'))) {
  return undefined;
 }
 return moduleKinds.find(kind => kind.language === module.language && kind.basis === module.basis);
}

function colour(module,views){
 const assigned=new Map(),used=new Set();
 for(const m of [...views.map(v=>v.symbol?.module),module]){
  if(!m||!validModule(m)||assigned.has(m.id))continue;
  const preferred=parseInt(m.id.slice(-8),16)%6;
  const next=Array.from({length:6},(_,n)=>(preferred+n)%6).find(n=>!used.has(n))??preferred;
  assigned.set(m.id,next);used.add(next);
 }
 return assigned.get(module.id);
}
export function validModule(module) {
 return module === null || module === undefined || Boolean(moduleKind(module));
}
export function modulePresentation(view,views){
 const m = view.symbol?.module;
 const kind = moduleKind(m);
 if (!kind) return null;
 const peers=views.map(v=>v.symbol?.module).filter(n=>n&&n.label===m.label&&n.id!==m.id);
 let label=m.label;
 if(peers.length){const parts=m.root.split('/');let suffix=m.root;for(let n=1;n<=parts.length;n++){const candidate=parts.slice(-n).join('/');if(peers.every(p=>!p.root.endsWith(candidate))){suffix=candidate;break;}}label=`${suffix} · ${label}`;}
 return {id:m.id,label,title:`${m.label}\n${kind.tooltip}\nRoot: ${m.root}`,colour:colour(m,views)};
}
export function modulePeers(views,id){return new Set(views.filter(v=>v.symbol?.module?.id===id).map(v=>v.id));}
export function moduleGeometryKey(views){return JSON.stringify(views.map(v=>[v.id,v.symbol?.module?.id??null,v.x,v.y,v.w,v.collapsed?54:v.h]));}
