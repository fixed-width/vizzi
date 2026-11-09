import {renderReflowText} from './reflow-text.js';
import {callRanges} from './call-ranges.js';

// Keep source characters and syntax spans intact; controls wrap only native identifier ranges.
export function renderCallText(node,line,actions,activate,plan=null) {
    renderReflowText(node,line,plan);
    const ranges=callRanges(line,actions);if(!ranges.length)return;
    const document=node.ownerDocument,offsets=new Map([[0,0]]);let byte=0,index=0;
    for(const ch of line.text){const value=ch.codePointAt(0);byte+=value<128?1:value<2048?2:value<65536?3:4;index+=ch.length;offsets.set(byte,index);}
    const nodes=[];const walker=document.createTreeWalker(node,4);let text,total=0;
    while((text=walker.nextNode())){const length=text.textContent.length;if(length)nodes.push({node:text,start:total,end:total+length});total+=length;}
    const endpoint=(point,end)=>{
        let low=0,high=nodes.length;while(low<high){const middle=(low+high)>>1;if(end?nodes[middle].end<point:nodes[middle].end<=point)low=middle+1;else high=middle;}
        return nodes[low];
    };
    for(const range of ranges.toReversed()){
        const start=offsets.get(range.start),end=offsets.get(range.end);if(start===undefined||end===undefined)continue;
        const first=endpoint(start,false),last=endpoint(end,true);if(!first||!last)continue;
        const selection=document.createRange();selection.setStart(first.node,start-first.start);selection.setEnd(last.node,end-last.start);
        const button=document.createElement('span');button.className='code-call';button.dataset.callId=range.id;button.setAttribute('role','button');button.tabIndex=0;
        button.setAttribute('aria-label',`Actions for ${range.name}`);button.setAttribute('aria-haspopup','menu');button.setAttribute('aria-expanded','false');button.title='Call actions · Ctrl/⌘-click opens the definition';
        button.append(selection.extractContents());selection.insertNode(button);
        const selected=()=>{const value=document.getSelection();if(!value||value.isCollapsed)return false;for(let i=0;i<value.rangeCount;i++)if(value.getRangeAt(i).intersectsNode(button))return true;return false;};
        const disabled=()=>button.getAttribute('aria-disabled')==='true';
        button.addEventListener('click',event=>{if(disabled()||selected()||event.detail>1)return;event.preventDefault();activate(range.action,button,Boolean(event.ctrlKey||event.metaKey),event.detail===0);});
        button.addEventListener('contextmenu',event=>{if(disabled()||selected())return;event.preventDefault();activate(range.action,button,false,true);});
        button.addEventListener('keydown',event=>{if(disabled())return;if(event.key==='Enter'){event.preventDefault();activate(range.action,button,Boolean(event.ctrlKey||event.metaKey),true);}else if(event.key==='ArrowDown'||event.key==='ContextMenu'||(event.shiftKey&&event.key==='F10')){event.preventDefault();activate(range.action,button,false,true);}else if(event.key===' ')event.preventDefault();});
        button.addEventListener('keyup',event=>{if(event.key===' '&&!disabled()){event.preventDefault();activate(range.action,button,false,true);}});
    }
}

export function createCallMenu(root) {
    const document=root.ownerDocument;let trigger=null;
    root.setAttribute('role','menu');root.hidden=true;
    const hide=(restore=false)=>{const previous=trigger;trigger?.setAttribute('aria-expanded','false');trigger=null;root.hidden=true;root.replaceChildren();if(restore&&previous?.isConnected)previous.focus({preventScroll:true});};
    const show=(element,name,actions,keyboard=false)=>{
        hide();trigger=element;element.setAttribute('aria-expanded','true');root.setAttribute('aria-label',`Actions for ${name}`);
        for(const action of actions){const button=document.createElement('button');button.type='button';button.setAttribute('role','menuitem');button.append(document.createTextNode(action.label));if(action.hint){const hint=document.createElement('small');hint.textContent=action.hint;button.append(hint);}button.addEventListener('click',()=>{hide();action.run();});root.append(button);}
        root.hidden=false;const anchor=element.getBoundingClientRect(),menu=root.getBoundingClientRect(),view=document.defaultView;const left=Math.max(8,Math.min(anchor.left,view.innerWidth-menu.width-8));const top=anchor.bottom+6+menu.height<=view.innerHeight-8?anchor.bottom+6:Math.max(8,anchor.top-menu.height-6);Object.assign(root.style,{left:left+'px',top:top+'px'});if(keyboard)root.querySelector('[role=menuitem]')?.focus();
    };
    root.addEventListener('keydown',event=>{const items=[...root.querySelectorAll('[role=menuitem]')],index=items.indexOf(document.activeElement);if(event.key==='Escape'){event.preventDefault();event.stopPropagation();hide(true);}else if(['ArrowDown','ArrowUp','Home','End'].includes(event.key)&&items.length){event.preventDefault();const next=event.key==='Home'?0:event.key==='End'?items.length-1:(index+(event.key==='ArrowDown'?1:items.length-1))%items.length;items[next].focus();}else if(event.key==='Tab')hide(true);});
    document.addEventListener('pointerdown',event=>{if(trigger&&!root.contains(event.target)&&!trigger.contains(event.target))hide();});
    document.addEventListener('focusin',event=>{if(trigger&&!root.contains(event.target)&&!trigger.contains(event.target))hide();});
    document.addEventListener('keydown',event=>{if(event.key==='Escape'&&trigger){event.preventDefault();hide(true);}});
    document.addEventListener('selectionchange',()=>{if(trigger&&!document.getSelection()?.isCollapsed)hide();});
    document.defaultView.addEventListener('resize',()=>hide());
    return {show,hide};
}
