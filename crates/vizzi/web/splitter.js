import {splitFraction} from '/camera.js';

export function installSplitter(root,{key,label,initial,minBefore=120,minAfter=180}) {
  const handle=root.ownerDocument.createElement('div');handle.className='splitter';handle.tabIndex=0;
  handle.setAttribute('role','separator');handle.setAttribute('aria-orientation','vertical');handle.setAttribute('aria-label',label);
  root.children[1].before(handle);root.classList.add('resizable');
  const storageKey=`vizzi.layout.v1.${key}`;
  let ratio=initial,drag;
  try {const saved=localStorage.getItem(storageKey);if(saved!==null && Number.isFinite(Number(saved)))ratio=Number(saved);}catch{}
  function apply() {
    const extent=Math.max(1,root.clientWidth-8);ratio=splitFraction(ratio,extent,minBefore,minAfter);
    root.style.setProperty('--split-first',`${ratio}fr`);root.style.setProperty('--split-second',`${1-ratio}fr`);
    handle.setAttribute('aria-valuenow',String(Math.round(ratio*100)));
    handle.setAttribute('aria-valuemin',String(Math.round(splitFraction(0,extent,minBefore,minAfter)*100)));
    handle.setAttribute('aria-valuemax',String(Math.round(splitFraction(1,extent,minBefore,minAfter)*100)));
    handle.setAttribute('aria-valuetext',`${Math.round(ratio*100)}%`);
  }
  function save(){try{localStorage.setItem(storageKey,String(ratio));}catch{}}
  handle.addEventListener('pointerdown',event=>{if(event.button!==0)return;event.preventDefault();handle.focus();drag={x:event.clientX,ratio};handle.setPointerCapture(event.pointerId);});
  handle.addEventListener('pointermove',event=>{if(!drag)return;ratio=drag.ratio+(event.clientX-drag.x)/Math.max(1,root.clientWidth-8);apply();});
  handle.addEventListener('pointerup',()=>{drag=null;save();});handle.addEventListener('pointercancel',()=>{drag=null;save();});
  handle.addEventListener('keydown',event=>{
    const step=(event.shiftKey?50:20)/Math.max(1,root.clientWidth-8);
    const next={ArrowLeft:ratio-step,ArrowRight:ratio+step,Home:0,End:1}[event.key];
    if(next===undefined)return;event.preventDefault();ratio=next;apply();save();
  });
  new ResizeObserver(apply).observe(root);apply();
}
