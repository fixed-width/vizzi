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
function pageHierarchy(files, page) {
  const root = {path:page.directory,parent:null,children:[],files,count:page.files};
  const nodes = new Map([[root.path, root]]);
  for (const directory of page.directories) {
    const child = {
      path:directory.path, parent:root.path, label:directory.path.split('/').at(-1),
      count:directory.files, summary:true, children:[], files:[],
    };
    root.children.push(child);
    nodes.set(child.path, child);
  }
  return {nodes, root};
}

function samePage(a, b) {
  return a === b || Boolean(a && b && a.directory === b.directory && a.files === b.files &&
    a.offset === b.offset && a.directories.length === b.directories.length &&
    a.directories.every((dir, index) => dir.path === b.directories[index].path && dir.files === b.directories[index].files));
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

export function createMapModel(initialFiles,preferences={},initialPage=null) {
  let files=initialFiles,page=initialPage;
  let {nodes,root}=page ? pageHierarchy(files,page) : hierarchy(files),focusPath=root.path,revision=0,cached;
  const valid=path=>typeof path==='string' && path.length<=16384;
  const collapsed=new Set(Array.isArray(preferences?.collapsed)?preferences.collapsed.filter(valid).slice(0,2000):[]),order=new Map();
  for(const entry of (Array.isArray(preferences?.order)?preferences.order:[]).slice(0,2000))if(Array.isArray(entry) && valid(entry[0]) && Array.isArray(entry[1]))order.set(entry[0],[...new Set(entry[1].filter(valid))].slice(0,2000));
  if(!page && valid(preferences?.focus) && nodes.has(preferences.focus))focusPath=preferences.focus;
  const ordered=new Map();
  const invalidate=()=>{cached=undefined;ordered.clear();revision++;};
  function children(node) {
    if(page)return node.children;
    if(ordered.has(node.path))return ordered.get(node.path);
    const ranks=new Map((order.get(node.path) ?? []).map((p,i)=>[p,i]));
    const result=[...node.children].sort((a,b)=>(ranks.get(a.path) ?? Infinity)-(ranks.get(b.path) ?? Infinity) || alphabetic(a.path,b.path));ordered.set(node.path,result);return result;
  }
  function project() {
    if(cached)return cached;
    const focus=nodes.get(focusPath),groups=[],tiles=[],tree=[],breadcrumbs=[];
    const closed=(node,depth)=>node.summary || (!page && collapsed.has(node.path)) || depth>=MAX_DEPTH;
    const units=(node,depth)=>closed(node,depth)?1:node.files.length+node.children.reduce((n,c)=>n+units(c,depth+1),0);
    const area=Math.max(1,units(focus,0))*160*80;
    const aspect=page && root.children.length?1.65:1;
    const width=Math.max(320,Math.ceil(Math.sqrt(area*aspect))),height=Math.max(180,Math.ceil(area/width));
    const extent={x:0,y:0,width,height};
    function draw(node,rect,depth) {
      const limited=depth>=MAX_DEPTH,isClosed=closed(node,depth);
      const inset=Math.min(8,rect.width/8,rect.height/8),header=node===root?inset:Math.min(30,rect.height/4);
      if(node!==root)groups.push({path:node.path,parent:node.parent,label:node.label,count:node.count,collapsed:isClosed,limited,summary:node.summary,depth,header,rect});
      if(isClosed)return;
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
    while(pending.length){const {node,depth,index,total}=pending.pop();tree.push({path:node.path,parent:node.parent,label:node.label,count:node.count,collapsed:collapsed.has(node.path),summary:node.summary,depth,earlier:index>0,later:index<total-1});if(!collapsed.has(node.path))enqueue(node,depth+1);}
    if(page){
      breadcrumbs.push({path:ROOT,label:'Repository'});
      if(page.directory!==ROOT){
        let path='';
        for(const label of page.directory.split('/')){path=path?`${path}/${label}`:label;breadcrumbs.push({path,label});}
      }
    }else{
      let ancestor=focus;while(ancestor){breadcrumbs.unshift({path:ancestor.path,label:ancestor.path===ROOT?'Repository':ancestor.label});ancestor=nodes.get(ancestor.parent);}
    }
    cached={groups,files:tiles,width,height,tree,breadcrumbs,focus:focusPath,paged:Boolean(page),hiddenFiles:page?0:focus.count-tiles.length};return cached;
  }
  return {
    project,
    get revision(){return revision;},
    setFiles(next,nextPage=null){
      if(samePage(page,nextPage) && next.length===files.length && next.every((f,i)=>f.id===files[i].id && f.path===files[i].path)){
        const current=new Map(next.map(f=>[f.id,f]));files=next;
        for(const node of nodes.values())node.files=node.files.map(f=>current.get(f.id));
        if(cached)cached={...cached,files:cached.files.map(f=>({...current.get(f.id),rect:f.rect}))};
        return;
      }
      const wasPaged=Boolean(page);
      const built=nextPage?pageHierarchy(next,nextPage):hierarchy(next);
      page=nextPage;files=next;({nodes,root}=built);ordered.clear();
      if(Boolean(page)!==wasPaged || (!page && !nodes.has(focusPath)))revision++;
      if(page || wasPaged || !nodes.has(focusPath))focusPath=root.path;
      cached=undefined;
    },
    toggle(path){if(page || !nodes.has(path))return false;if(collapsed.has(path))collapsed.delete(path);else collapsed.add(path);invalidate();return true;},
    move(path,before){const node=nodes.get(path),target=before===null?null:nodes.get(before);if(page || !node || node.path===ROOT || (before!==null && (!target || target.parent!==node.parent)) || path===before)return false;const siblings=children(nodes.get(node.parent)).map(n=>n.path).filter(p=>p!==path);const index=before===null?siblings.length:siblings.indexOf(before);siblings.splice(index,0,path);order.set(node.parent,siblings);invalidate();return true;},
    focus(path){if(page || !nodes.has(path))return false;if(path!==focusPath || collapsed.has(path)){focusPath=path;collapsed.delete(path);invalidate();}return true;},
    reveal(id,{focusDeep=false}={}){
      if(page)return;
      const file=files.find(f=>f.id===id);
      if(!file)return;
      const directory=nodes.get(parent(file.path));
      let changed=false,depth=0;
      if(!within(directory.path,focusPath)){focusPath=ROOT;changed=true;}
      for(let node=directory;node;node=nodes.get(node.parent)){
        if(focusDeep && node.path===focusPath && depth>=MAX_DEPTH){
          focusPath=directory.path;
          changed=true;
        }
        changed=collapsed.delete(node.path)||changed;
        depth++;
      }
      if(changed)invalidate();
    },
    reset(){collapsed.clear();order.clear();focusPath=root.path;invalidate();},
    preferences(){return {collapsed:[...collapsed],order:[...order],focus:focusPath};},
  };
}
