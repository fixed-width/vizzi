import {moduleGeometryKey,modulePresentation} from './module-identity.js';
export function installModuleOutlines(root,read,{createWorker=()=>new Worker('/module-outlines.worker.js',{type:'module'}),failed=()=>{}}={}){
 let enabled=false,worker=null,key=null,latest=null,inflight=false,ticket=0,timer=null,frame=null;
 const view=root.ownerDocument.defaultView;
 function stop(){worker?.terminate();worker=null;inflight=false;clearTimeout(timer);}
 function send(){if(!enabled||inflight||!latest)return;try{
  worker??=createWorker();const instance=worker;inflight=true;const current=ticket,payload=latest;latest=null;
  worker.onmessage=event=>{if(worker!==instance)return;inflight=false;clearTimeout(timer);if(enabled&&event.data.ticket===ticket){root.replaceChildren();const views=read();for(const path of event.data.paths??[]){const v=views.find(v=>v.symbol?.module?.id===path.id),m=v&&modulePresentation(v,views);if(!m)continue;const p=root.ownerDocument.createElementNS('http://www.w3.org/2000/svg','path');p.classList.add('module-outline');p.dataset.moduleId=path.id;p.setAttribute('d',path.path);p.setAttribute('transform',path.transform);p.style.setProperty('--module-accent',`var(--module-${m.colour})`);root.append(p);}}send();};
  worker.onerror=()=>{if(worker!==instance)return;stop();latest=null;failed();};timer=setTimeout(()=>{if(worker!==instance)return;stop();latest=null;failed();},1000);worker.postMessage({ticket:current,views:payload});
 }catch{stop();latest=null;failed();}}
 function refresh(){if(!enabled)return;const views=read(),next=moduleGeometryKey(views);if(next===key)return;key=next;ticket++;root.replaceChildren();const groups=new Map();for(const v of views){const id=v.symbol?.module?.id;if(id)groups.set(id,(groups.get(id)??0)+1);}if(![...groups.values()].some(n=>n>=2)){latest=null;return;}latest=views.map(v=>({module:v.symbol?.module?.id??null,x:v.x,y:v.y,w:v.w,h:v.collapsed?54:v.h}));if(frame===null)frame=view.requestAnimationFrame(()=>{frame=null;send();});}
 return {refresh,setEnabled(value){enabled=Boolean(value);key=null;if(!enabled){ticket++;latest=null;stop();if(frame!==null)view.cancelAnimationFrame(frame);frame=null;root.replaceChildren();}else refresh();},dispose(){enabled=false;stop();if(frame!==null)view.cancelAnimationFrame(frame);root.replaceChildren();}};
}
