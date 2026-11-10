// Local, deterministic fitting; source facts and the camera are outside this plan.
export const BUBBLE_SIZE={minWidth:320,maxWidth:1600,minHeight:200,maxHeight:2400};
const height=v=>v.collapsed?54:v.h;
const overlaps=(a,b,horizontal)=>a.x<b.x+b.w&&b.x<a.x+a.w+(horizontal?24:0)&&a.y<b.y+height(b)&&b.y<a.y+height(a)+(horizontal?0:24);
export function fitViews(views,id,axis,extent){
 if(!['width','height'].includes(axis)||!Number.isFinite(extent)||extent<=0)throw new Error('Invalid bubble fit');
 const next=views.map(v=>({...v})),chosen=next.find(v=>v.id===id);if(!chosen)throw new Error('Bubble is absent');
 const horizontal=axis==='width',position=horizontal?'x':'y',dimension=horizontal?'w':'h';
 const previous=chosen[dimension],collapsed=chosen.collapsed;chosen[dimension]=Math.max(horizontal?BUBBLE_SIZE.minWidth:BUBBLE_SIZE.minHeight,Math.min(extent,horizontal?BUBBLE_SIZE.maxWidth:BUBBLE_SIZE.maxHeight));chosen.collapsed=false;if(chosen[dimension]<=previous&&!collapsed)return next;
 const ordered=[chosen,...next.filter(v=>v!==chosen).sort((a,b)=>a[position]-b[position]||a.id.localeCompare(b.id))],touched=new Set([id]);
 for(let i=1;i<ordered.length;i++){
  const v=ordered[i],earlier=ordered.slice(0,i);
  if(!earlier.some(p=>touched.has(p.id)&&overlaps(p,v,horizontal)))continue;
  touched.add(v.id);
  for(let step=0;step<i;step++){
   const blockers=earlier.filter(p=>overlaps(p,v,horizontal));if(!blockers.length)break;
   v[position]=Math.max(...blockers.map(p=>p[position]+(horizontal?p.w:height(p))+24));
  }
 }
 if(next.some(v=>!Number.isFinite(v.x)||!Number.isFinite(v.y)||Math.abs(v.x)>1e5||Math.abs(v.y)>1e5))throw new Error('Expansion exceeds workspace bounds');
 return next;
}
