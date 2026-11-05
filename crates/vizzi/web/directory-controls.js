const ROW=28,LIMIT=64;
export function createDirectoryControls({tree,breadcrumbs,onChange}) {
  const document=tree.ownerDocument,space=document.createElement('div');space.className='directory-space';tree.append(space);tree.setAttribute('role','tree');tree.setAttribute('aria-label','Map directories');
  let model,rows=[],focused,dragging,lastTree,lastBreadcrumbs;
  function change(action){if(action()===false)return;render();onChange();}
  function render(){
    if(!model)return;const scene=model.project();lastTree=scene.tree;lastBreadcrumbs=scene.breadcrumbs;rows=scene.tree;space.style.height=`${rows.length*ROW}px`;
    const first=Math.max(0,Math.floor(tree.scrollTop/ROW)-4),last=Math.min(rows.length,first+LIMIT,Math.ceil((tree.scrollTop+tree.clientHeight)/ROW)+5);
    const hadFocus=tree.contains(document.activeElement),active=focused ?? rows[first]?.path;
    const fragment=document.createDocumentFragment();
    for(let i=first;i<last;i++){
      const dir=rows[i],row=document.createElement('div');row.className='directory-row';row.style.top=`${i*ROW}px`;row.style.paddingLeft=`${Math.min(dir.depth*12,120)}px`;row.dataset.directory=dir.path;row.draggable=true;row.setAttribute('role','treeitem');row.setAttribute('aria-level',String(dir.depth+1));row.setAttribute('aria-expanded',String(!dir.collapsed));row.setAttribute('aria-label',dir.path);row.title=dir.path;
      const button=(text,label,action)=>{const b=document.createElement('button');b.type='button';b.textContent=text;b.setAttribute('aria-label',label);b.onclick=()=>{focused=dir.path;change(action);};row.append(b);return b;};
      button(dir.collapsed?'▸':'▾',`${dir.collapsed?'Expand':'Collapse'} directory ${dir.path}`,()=>model.toggle(dir.path));
      const name=button(`${dir.label} (${dir.count})`,`Focus directory ${dir.path}`,()=>model.focus(dir.path));name.className='directory-name';name.dataset.focusDirectory=dir.path;name.tabIndex=dir.path===active?0:-1;name.onfocus=()=>{focused=dir.path;};
      const siblings=rows.filter(r=>r.parent===dir.parent),at=siblings.findIndex(r=>r.path===dir.path);
      button('↑',`Move ${dir.path} earlier`,()=>model.move(dir.path,siblings[at-1].path)).disabled=!dir.earlier;
      button('↓',`Move ${dir.path} later`,()=>model.move(dir.path,siblings[at+2]?.path ?? null)).disabled=!dir.later;
      fragment.append(row);
    }
    space.replaceChildren(fragment);
    if(hadFocus){const node=[...space.querySelectorAll('[data-focus-directory]')].find(b=>b.dataset.focusDirectory===focused);node?.focus({preventScroll:true});}
    breadcrumbs.replaceChildren();for(const dir of scene.breadcrumbs){const b=document.createElement('button');b.type='button';b.textContent=dir.label;b.setAttribute('aria-label',dir.path==='.'?'Repository root':`Focus directory ${dir.path}`);if(dir.path===scene.focus)b.setAttribute('aria-current','location');b.onclick=()=>change(()=>model.focus(dir.path));breadcrumbs.append(b);}
  }
  tree.addEventListener('scroll',render,{passive:true});new ResizeObserver(render).observe(tree);
  tree.addEventListener('dragstart',event=>{const row=event.target.closest('[data-directory]');if(!row)return;dragging=row.dataset.directory;event.dataTransfer.setData('text/plain',dragging);event.dataTransfer.effectAllowed='move';});
  tree.addEventListener('dragover',event=>{const row=event.target.closest('[data-directory]');if(row && dragging && rows.find(r=>r.path===dragging)?.parent===rows.find(r=>r.path===row.dataset.directory)?.parent)event.preventDefault();});
  tree.addEventListener('drop',event=>{const path=event.target.closest('[data-directory]')?.dataset.directory;if(!path || !dragging)return;event.preventDefault();change(()=>model.move(dragging,path));dragging=undefined;});tree.addEventListener('dragend',()=>{dragging=undefined;});
  tree.addEventListener('keydown',event=>{
    const path=event.target.closest('[data-directory]')?.dataset.directory,at=rows.findIndex(r=>r.path===path);if(at<0)return;
    const dir=rows[at];
    if(event.altKey && ['ArrowUp','ArrowDown'].includes(event.key)){event.preventDefault();const siblings=rows.filter(r=>r.parent===dir.parent),index=siblings.findIndex(r=>r.path===path);if(event.key==='ArrowUp' && dir.earlier)change(()=>model.move(path,siblings[index-1].path));if(event.key==='ArrowDown' && dir.later)change(()=>model.move(path,siblings[index+2]?.path ?? null));return;}
    if(event.key==='ArrowLeft' || event.key==='ArrowRight'){event.preventDefault();if((event.key==='ArrowLeft')!==dir.collapsed)change(()=>model.toggle(path));return;}
    const next={ArrowUp:at-1,ArrowDown:at+1,Home:0,End:rows.length-1}[event.key];if(next===undefined)return;event.preventDefault();const index=Math.max(0,Math.min(rows.length-1,next));focused=rows[index].path;tree.scrollTop=Math.max(0,Math.min(index*ROW,Math.max(tree.scrollTop,(index+1)*ROW-tree.clientHeight)));render();[...space.querySelectorAll('[data-focus-directory]')].find(b=>b.dataset.focusDirectory===focused)?.focus({preventScroll:true});
  });
  return {apply(next){model=next;const scene=model.project();if(scene.tree!==lastTree || scene.breadcrumbs!==lastBreadcrumbs)render();}};
}
