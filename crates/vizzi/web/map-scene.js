const ROOT='.',MAX_FILES=2000,MAX_DEPTH=10;
const parent=path=>path.includes('/')?path.slice(0,path.lastIndexOf('/')):ROOT;
const within=(path,ancestor)=>ancestor===ROOT || path===ancestor || path.startsWith(ancestor+'/');
const alphabetic=(a,b)=>a<b?-1:a>b?1:0;
function common(a,b) {
  if(within(b,a))return a;if(within(a,b))return b;
  let i=0;while(i<Math.min(a.length,b.length) && a[i]===b[i])i++;
  const slash=a.lastIndexOf('/',i-1);return slash<0?ROOT:a.slice(0,slash);
}
function hierarchy(files) {
  if(files.length>MAX_FILES)throw new Error('Map exceeds the 2,000-file projection');
  const directories=new Set([ROOT]);
  for(const file of files)directories.add(parent(file.path));
  const paths=[...directories].filter(p=>p!==ROOT).sort(alphabetic);
  for(let i=1;i<paths.length;i++)directories.add(common(paths[i-1],paths[i]));
  const nodes=new Map([...directories].sort(alphabetic).map(path=>[path,{path,parent:null,label:path,children:[],files:[],count:0}]));
  const root=nodes.get(ROOT),stack=[root];
  for(const node of nodes.values()) {
    if(node===root)continue;
    while(!within(node.path,stack.at(-1).path))stack.pop();
    const ancestor=stack.at(-1);node.parent=ancestor.path;node.label=ancestor.path===ROOT?node.path:node.path.slice(ancestor.path.length+1);ancestor.children.push(node);stack.push(node);
  }
  for(const file of files)nodes.get(parent(file.path)).files.push(file);
  for(const node of [...nodes.values()].reverse())node.count=node.files.length+node.children.reduce((n,c)=>n+c.count,0);
  return {nodes,root};
}
function partition(items,rect,emit) {
  if(!items.length)return;
  if(items.length===1){emit(items[0],rect);return;}
  const total=items.reduce((n,item)=>n+item.weight,0);let sum=items[0].weight,cut=1;
  while(cut<items.length-1 && Math.abs(sum+items[cut].weight-total/2)<Math.abs(sum-total/2)){sum+=items[cut].weight;cut++;}
  const ratio=sum/total;
  if(rect.width>=rect.height){const width=rect.width*ratio;partition(items.slice(0,cut),{...rect,width},emit);partition(items.slice(cut),{...rect,x:rect.x+width,width:rect.width-width},emit);}
  else {const height=rect.height*ratio;partition(items.slice(0,cut),{...rect,height},emit);partition(items.slice(cut),{...rect,y:rect.y+height,height:rect.height-height},emit);}
}

export function createMapModel(initialFiles,preferences={}) {
  let files=initialFiles,{nodes,root}=hierarchy(files),focusPath=ROOT,revision=0,cached;
  const valid=path=>typeof path==='string' && path.length<=16384;
  const collapsed=new Set(Array.isArray(preferences?.collapsed)?preferences.collapsed.filter(valid).slice(0,2000):[]),order=new Map();
  for(const entry of (Array.isArray(preferences?.order)?preferences.order:[]).slice(0,2000))if(Array.isArray(entry) && valid(entry[0]) && Array.isArray(entry[1]))order.set(entry[0],[...new Set(entry[1].filter(valid))].slice(0,2000));
  if(valid(preferences?.focus) && nodes.has(preferences.focus))focusPath=preferences.focus;
  const ordered=new Map();
  const invalidate=()=>{cached=undefined;ordered.clear();revision++;};
  function children(node) {
    if(ordered.has(node.path))return ordered.get(node.path);
    const ranks=new Map((order.get(node.path) ?? []).map((p,i)=>[p,i]));
    const result=[...node.children].sort((a,b)=>(ranks.get(a.path) ?? Infinity)-(ranks.get(b.path) ?? Infinity) || alphabetic(a.path,b.path));ordered.set(node.path,result);return result;
  }
  function project() {
    if(cached)return cached;
    const focus=nodes.get(focusPath),groups=[],tiles=[],tree=[],breadcrumbs=[];
    const units=(node,depth)=>collapsed.has(node.path) || depth>=MAX_DEPTH?1:node.files.length+node.children.reduce((n,c)=>n+units(c,depth+1),0);
    const count=units(focus,0),width=Math.max(320,Math.ceil(Math.sqrt(Math.max(1,count)*160*80))),height=Math.max(180,Math.ceil(Math.max(1,count)*160*80/width));
    const extent={x:0,y:0,width,height};
    function draw(node,rect,depth) {
      const limited=depth>=MAX_DEPTH,closed=collapsed.has(node.path) || limited;
      const inset=Math.min(8,rect.width/8,rect.height/8),header=node.path===ROOT?inset:Math.min(30,rect.height/4);
      if(node.path!==ROOT)groups.push({path:node.path,parent:node.parent,label:node.label,count:node.count,collapsed:closed,limited,depth,header,rect});
      if(closed)return;
      const body={x:rect.x+inset,y:rect.y+header,width:rect.width-2*inset,height:rect.height-header-inset};
      const items=[...children(node).map(node=>({node,weight:units(node,depth+1)})),...node.files.map(file=>({file,weight:1}))];
      partition(items,body,(item,cell)=>{
        if(item.node)draw(item.node,cell,depth+1);
        else {const inset=Math.min(3,cell.width/4,cell.height/4);tiles.push({...item.file,rect:{x:cell.x+inset,y:cell.y+inset,width:cell.width-2*inset,height:cell.height-2*inset}});}
      });
    }
    if(focus.count)draw(focus,extent,0);
    const pending=[];
    function enqueue(node,depth){const list=children(node);for(let i=list.length-1;i>=0;i--)pending.push({node:list[i],depth,index:i,total:list.length});}
    enqueue(root,0);
    while(pending.length){const {node,depth,index,total}=pending.pop();tree.push({path:node.path,parent:node.parent,label:node.label,count:node.count,collapsed:collapsed.has(node.path),depth,earlier:index>0,later:index<total-1});if(!collapsed.has(node.path))enqueue(node,depth+1);}
    let ancestor=focus;while(ancestor){breadcrumbs.unshift({path:ancestor.path,label:ancestor.path===ROOT?'Repository':ancestor.label});ancestor=nodes.get(ancestor.parent);}
    cached={groups,files:tiles,width,height,tree,breadcrumbs,focus:focusPath,hiddenFiles:focus.count-tiles.length};return cached;
  }
  return {
    project,
    get revision(){return revision;},
    setFiles(next){
      if(next.length===files.length && next.every((f,i)=>f.id===files[i].id && f.path===files[i].path)){
        const current=new Map(next.map(f=>[f.id,f]));files=next;
        for(const node of nodes.values())node.files=node.files.map(f=>current.get(f.id));
        if(cached)cached={...cached,files:cached.files.map(f=>({...current.get(f.id),rect:f.rect}))};
        return;
      }
      const built=hierarchy(next);files=next;({nodes,root}=built);ordered.clear();if(!nodes.has(focusPath)){focusPath=ROOT;revision++;}cached=undefined;
    },
    toggle(path){if(!nodes.has(path))return false;if(collapsed.has(path))collapsed.delete(path);else collapsed.add(path);invalidate();return true;},
    move(path,before){const node=nodes.get(path),target=before===null?null:nodes.get(before);if(!node || node.path===ROOT || (before!==null && (!target || target.parent!==node.parent)) || path===before)return false;const siblings=children(nodes.get(node.parent)).map(n=>n.path).filter(p=>p!==path);const index=before===null?siblings.length:siblings.indexOf(before);siblings.splice(index,0,path);order.set(node.parent,siblings);invalidate();return true;},
    focus(path){if(!nodes.has(path))return false;if(path!==focusPath || collapsed.has(path)){focusPath=path;collapsed.delete(path);invalidate();}return true;},
    reveal(id){const file=files.find(f=>f.id===id);if(!file)return;let changed=false,dir=nodes.get(parent(file.path));if(!within(dir.path,focusPath)){focusPath=ROOT;changed=true;}while(dir){changed=collapsed.delete(dir.path)||changed;dir=nodes.get(dir.parent);}if(changed)invalidate();},
    reset(){collapsed.clear();order.clear();focusPath=ROOT;invalidate();},
    preferences(){return {collapsed:[...collapsed],order:[...order],focus:focusPath};},
  };
}
