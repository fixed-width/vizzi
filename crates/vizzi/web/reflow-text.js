import {renderStyledText} from './presentation.js';

// Display segments contain only original bytes; indentation and markers are decoration.
export function renderReflowText(node,line,plan) {
    const document=node.ownerDocument,bytes=new TextEncoder().encode(line.text),decoder=new TextDecoder('utf-8',{fatal:true});
    const pieces=plan??[{start:0,end:bytes.length,indent:0}];node.replaceChildren();
    for(const [index,piece] of pieces.entries()){
        const segment=document.createElement('span');segment.className='reflow-segment';segment.dataset.start=String(piece.start);segment.dataset.end=String(piece.end);segment.style.paddingLeft=piece.indent+'px';
        if(index){const mark=document.createElement('span');mark.className='reflow-marker';mark.setAttribute('aria-hidden','true');segment.append(mark);}
        const clip=()=>{const mark=document.createElement('span');mark.className='source-clip';mark.textContent='…';segment.append(mark);};
        if(index===0&&line.left)clip();
        const literal=document.createElement('span');literal.className='source-chunk';
        const styles=(line.styles??[]).filter(s=>s.start<piece.end&&s.end>piece.start).map(s=>({...s,start:Math.max(s.start,piece.start)-piece.start,end:Math.min(s.end,piece.end)-piece.start}));
        renderStyledText(literal,{text:decoder.decode(bytes.subarray(piece.start,piece.end)),styles,left:false,right:false});segment.append(literal);
        if(index===pieces.length-1&&line.right)clip();node.append(segment);
    }
}
