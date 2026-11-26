const ROW=28,LIMIT=64;
export function createDirectoryControls({section,toggle,tree,breadcrumbs,onChange,onOpen}) {
  const document=tree.ownerDocument,space=document.createElement('div');space.className='directory-space';tree.append(space);tree.setAttribute('role','tree');tree.setAttribute('aria-label','Map directories');
  let model,rows=[],focused,dragging,lastVisibleRows,lastBreadcrumbs;
  function change(action){if(action()===false)return;render();onChange();}
  function open(path){if(model.project().paged)onOpen(path);else change(()=>model.focus(path));}
  function render(){
    if(!model)return;const scene=model.project();
    const activeElement=document.activeElement;
    const hadFocus=tree.contains(activeElement),breadcrumbFocus=activeElement?.dataset.breadcrumb;
    rows=scene.tree;space.style.height=`${rows.length*ROW}px`;
    section.hidden=!rows.length;
    if(!section.hidden && !tree.hidden && lastVisibleRows!==rows){
      if(scene.paged)tree.scrollTop=0;
      lastVisibleRows=rows;
    }
    const first=Math.max(0,Math.floor(tree.scrollTop/ROW)-4),last=Math.min(rows.length,first+LIMIT,Math.ceil((tree.scrollTop+tree.clientHeight)/ROW)+5);
    const active=rows.some(row=>row.path===focused)?focused:rows[first]?.path;
    const fragment=document.createDocumentFragment();
    for(let i=first;i<last;i++){
      const dir=rows[i],row=document.createElement('div');row.className='directory-row';row.style.top=`${i*ROW}px`;row.style.paddingLeft=`${Math.min(dir.depth*12,120)}px`;row.dataset.directory=dir.path;row.draggable=!dir.summary;row.setAttribute('role','treeitem');row.setAttribute('aria-level',String(dir.depth+1));row.setAttribute('aria-label',dir.path);row.title=dir.path;
      const button=(text,label,action)=>{const b=document.createElement('button');b.type='button';b.textContent=text;b.setAttribute('aria-label',label);b.onclick=()=>{focused=dir.path;action();};row.append(b);return b;};
      if(!dir.summary){
        row.setAttribute('aria-expanded',String(!dir.collapsed));
        button(dir.collapsed?'▸':'▾',`${dir.collapsed?'Expand':'Collapse'} directory ${dir.path}`,()=>change(()=>model.toggle(dir.path)));
      }
      const count=dir.count.toLocaleString();
      const name=button(`${dir.label} (${count})`,dir.summary?`Open directory ${dir.path}, ${count} ${dir.count===1?'file':'files'}`:`Focus directory ${dir.path}`,
        ()=>open(dir.path));
      name.className='directory-name';name.dataset.focusDirectory=dir.path;name.tabIndex=dir.path===active?0:-1;name.onfocus=()=>{focused=dir.path;};
      if(!dir.summary){
        const siblings=rows.filter(r=>r.parent===dir.parent),at=siblings.findIndex(r=>r.path===dir.path);
        button('↑',`Move ${dir.path} earlier`,()=>change(()=>model.move(dir.path,siblings[at-1].path))).disabled=!dir.earlier;
        button('↓',`Move ${dir.path} later`,()=>change(()=>model.move(dir.path,siblings[at+2]?.path ?? null))).disabled=!dir.later;
      }
      fragment.append(row);
    }
    space.replaceChildren(fragment);
    if(lastBreadcrumbs!==scene.breadcrumbs){
      lastBreadcrumbs=scene.breadcrumbs;
      const buttons=scene.breadcrumbs.map(dir=>{
        const button=document.createElement('button');button.type='button';button.textContent=dir.label;button.dataset.breadcrumb=dir.path;
        if(dir.path==='.' || !scene.paged)button.setAttribute('aria-label',dir.path==='.'?'Repository root':`Focus directory ${dir.path}`);
        if(dir.path===scene.focus)button.setAttribute('aria-current','location');
        button.onclick=()=>open(dir.path);
        return button;
      });
      breadcrumbs.replaceChildren(...buttons);
      if(breadcrumbFocus!==undefined)(buttons.find(button=>button.dataset.breadcrumb===breadcrumbFocus) ?? breadcrumbs.lastElementChild)?.focus({preventScroll:true});
    }
    if(section.hidden && section.contains(activeElement))breadcrumbs.lastElementChild?.focus({preventScroll:true});
    else if(hadFocus){const node=[...space.querySelectorAll('[data-focus-directory]')].find(b=>b.dataset.focusDirectory===active);(node ?? breadcrumbs.lastElementChild)?.focus({preventScroll:true});}
  }
  toggle.addEventListener('click',()=>{
    if(tree.contains(document.activeElement))toggle.focus({preventScroll:true});
    tree.hidden=!tree.hidden;
    toggle.setAttribute('aria-expanded',String(!tree.hidden));
    if(!tree.hidden)render();
  });
  tree.addEventListener('scroll',render,{passive:true});new ResizeObserver(render).observe(tree);
  tree.addEventListener('dragstart',event=>{const row=event.target.closest('[data-directory]');if(!row)return;dragging=row.dataset.directory;event.dataTransfer.setData('text/plain',dragging);event.dataTransfer.effectAllowed='move';});
  tree.addEventListener('dragover',event=>{const row=event.target.closest('[data-directory]');if(row && dragging && rows.find(r=>r.path===dragging)?.parent===rows.find(r=>r.path===row.dataset.directory)?.parent)event.preventDefault();});
  tree.addEventListener('drop',event=>{const path=event.target.closest('[data-directory]')?.dataset.directory;if(!path || !dragging)return;event.preventDefault();change(()=>model.move(dragging,path));dragging=undefined;});tree.addEventListener('dragend',()=>{dragging=undefined;});
  tree.addEventListener('keydown',event=>{
    const path=event.target.closest('[data-directory]')?.dataset.directory,at=rows.findIndex(r=>r.path===path);if(at<0)return;
    const dir=rows[at];
    if(!dir.summary && event.altKey && ['ArrowUp','ArrowDown'].includes(event.key)){event.preventDefault();const siblings=rows.filter(r=>r.parent===dir.parent),index=siblings.findIndex(r=>r.path===path);if(event.key==='ArrowUp' && dir.earlier)change(()=>model.move(path,siblings[index-1].path));if(event.key==='ArrowDown' && dir.later)change(()=>model.move(path,siblings[index+2]?.path ?? null));return;}
    if(event.key==='ArrowLeft' || event.key==='ArrowRight'){event.preventDefault();if(dir.summary){if(event.key==='ArrowRight')open(path);}else if((event.key==='ArrowLeft')!==dir.collapsed)change(()=>model.toggle(path));return;}
    const next={ArrowUp:at-1,ArrowDown:at+1,Home:0,End:rows.length-1}[event.key];if(next===undefined)return;event.preventDefault();const index=Math.max(0,Math.min(rows.length-1,next));focused=rows[index].path;tree.scrollTop=Math.max(0,Math.min(index*ROW,Math.max(tree.scrollTop,(index+1)*ROW-tree.clientHeight)));render();[...space.querySelectorAll('[data-focus-directory]')].find(b=>b.dataset.focusDirectory===focused)?.focus({preventScroll:true});
  });
  return {apply(next){model=next;const scene=model.project();if(scene.tree!==rows || scene.breadcrumbs!==lastBreadcrumbs)render();}};
}
