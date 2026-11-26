import {splitFraction} from '/camera.js';

export function installSplitter(root,{key,label,initial,minBefore=120,minAfter=180,secondary=root.children[1]}) {
  const handle=root.ownerDocument.createElement('div');handle.className='splitter';handle.tabIndex=0;
  handle.setAttribute('role','separator');handle.setAttribute('aria-orientation','vertical');handle.setAttribute('aria-label',label);
  const primary=secondary.previousElementSibling;
  if(primary?.id)handle.setAttribute('aria-controls',primary.id);
  secondary.before(handle);root.classList.add('resizable');
  const storageKey=`vizzi.layout.v1.${key}`;
  const width=()=>Math.max(1,root.clientWidth-8);
  let preferred,ratio,drag;
  try {const saved=localStorage.getItem(storageKey);if(saved!==null && Number.isFinite(Number(saved)))preferred=Number(saved);}catch{}
  function apply() {
    const extent=width(),fallback=typeof initial==='function'?initial(extent):initial;
    ratio=splitFraction(preferred??fallback,extent,minBefore,minAfter);
    root.style.setProperty('--split-first',`${ratio}fr`);root.style.setProperty('--split-second',`${1-ratio}fr`);
    handle.setAttribute('aria-valuenow',String(Math.round(ratio*100)));
    handle.setAttribute('aria-valuemin',String(Math.round(splitFraction(0,extent,minBefore,minAfter)*100)));
    handle.setAttribute('aria-valuemax',String(Math.round(splitFraction(1,extent,minBefore,minAfter)*100)));
    handle.setAttribute('aria-valuetext',`${Math.round(ratio*100)}%`);
  }
  function resize(next) {
    const requested=splitFraction(next,width(),minBefore,minAfter);
    if(requested===ratio)return false;
    preferred=requested;apply();return true;
  }
  function save(){try{localStorage.setItem(storageKey,String(preferred));}catch{}}
  handle.addEventListener('pointerdown',event=>{
    if(event.button!==0 || drag)return;
    event.preventDefault();handle.focus();
    drag={pointerId:event.pointerId,x:event.clientX,ratio,preferred};
    handle.setPointerCapture(event.pointerId);
  });
  handle.addEventListener('pointermove',event=>{
    if(event.pointerId!==drag?.pointerId)return;
    resize(drag.ratio+(event.clientX-drag.x)/width());
  });
  function finishDrag(event) {
    if(event.pointerId!==drag?.pointerId)return;
    if(preferred!==drag.preferred)save();
    drag=undefined;
  }
  handle.addEventListener('pointerup',finishDrag);
  handle.addEventListener('pointercancel',finishDrag);
  handle.addEventListener('lostpointercapture',finishDrag);
  handle.addEventListener('keydown',event=>{
    const step=(event.shiftKey?50:20)/width();
    const next={ArrowLeft:ratio-step,ArrowRight:ratio+step,Home:0,End:1}[event.key];
    if(next===undefined)return;event.preventDefault();if(resize(next))save();
  });
  new ResizeObserver(apply).observe(root);apply();
}
