import {linkHighlights} from './link-highlights.js';

export function installLinkHighlights(root,read){
 const document=root.ownerDocument,view=document.defaultView;let pointer=null,frame=null;
 const applied=new Map();
 const target=element=>{
  if(!element||!root.contains(element))return null;
  const module=element.closest('.module-tag[data-module-id]');if(module)return {module:module.dataset.moduleId};
  const edge=element.closest('[data-link-id]');if(edge)return {link:edge.dataset.linkId};
  const bubble=element.closest('[data-view]');if(!bubble)return null;
  const row=element.closest('.code-line');return {view:bubble.dataset.view,...(row?{line:Number(row.dataset.line)}:{})};
 };
 const apply=(name,next)=>{const previous=applied.get(name)??new Set();for(const element of previous)if(!next.has(element))element.classList.remove(name);for(const element of next)if(!previous.has(element))element.classList.add(name);applied.set(name,next);};
 function refresh(){
  const active=document.activeElement,intent=pointer?target(document.elementFromPoint(pointer.x,pointer.y)):active?.matches(':focus-visible')?target(active):null;
  const state=read(),h=linkHighlights(state.views,state.links,[intent]),bubbles=new Set(),rows=new Set(),edges=new Set();
  for(const bubble of root.querySelectorAll('[data-view]')){
   const id=bubble.dataset.view;if(h.nodes.has(id))bubbles.add(bubble);
   const ranges=h.lines.filter(r=>r.view===id);if(ranges.length)for(const row of bubble.querySelectorAll('.code-line'))if(ranges.some(r=>Number(row.dataset.line)>=r.start&&Number(row.dataset.line)<=r.end))rows.add(row);
  }
  for(const edge of root.querySelectorAll('[data-link-id]:not(.link-hit)'))if(h.links.has(edge.dataset.linkId))edges.add(edge);
  const peers=new Set();if(intent?.module)for(const element of root.querySelectorAll('[data-module-id]'))if(element.dataset.moduleId===intent.module)peers.add(element);apply('is-module-peer',peers);
  apply('is-related',bubbles);apply('is-linked-call',rows);apply('is-linked-edge',edges);
 }
 const schedule=()=>{if(frame===null)frame=view.requestAnimationFrame(()=>{frame=null;refresh();});};
 const move=e=>{if(e.pointerType==='touch')return;pointer={x:e.clientX,y:e.clientY};refresh();},leave=()=>{pointer=null;refresh();},keyboard=()=>{pointer=null;schedule();};
 for(const event of ['pointerover','pointermove'])root.addEventListener(event,move);
 root.addEventListener('pointerleave',leave);root.addEventListener('keydown',keyboard);root.addEventListener('focusin',schedule);root.addEventListener('focusout',schedule);
 const observer=new MutationObserver(schedule);observer.observe(root,{childList:true,subtree:true});
 return {refresh,dispose(){observer.disconnect();if(frame!==null)view.cancelAnimationFrame(frame);for(const event of ['pointerover','pointermove'])root.removeEventListener(event,move);root.removeEventListener('pointerleave',leave);root.removeEventListener('keydown',keyboard);root.removeEventListener('focusin',schedule);root.removeEventListener('focusout',schedule);for(const name of applied.keys())apply(name,new Set());}};
}
