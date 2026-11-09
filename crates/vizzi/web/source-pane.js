import {planReflow} from './reflow.js';

// Owns local display layout, source selection/copy, and resize reading anchors.
export class SourcePane {
    constructor(element,render) {
        this.element=element;this.render=render;this.rows=[];this.enabled=true;this.pending=false;this.frame=null;this.closed=false;
        const document=element.ownerDocument;this.document=document;this.view=document.defaultView;
        const canvas=document.createElement('canvas');try{this.context=canvas.getContext('2d');}catch{this.context=null;}
        this.resize=new ResizeObserver(()=>this.schedule());this.resize.observe(element);
        this.selection=()=>{if(this.pending&&!this.selected())this.schedule();};document.addEventListener('selectionchange',this.selection);
        this.copy=event=>this.copySelection(event);document.addEventListener('copy',this.copy);
    }
    set(lines,reading={top:0,left:0}) {
        this.rows=lines.map(line=>{
            const row=this.document.createElement('div');row.className='code-line';row.dataset.line=String(line.number);
            const number=this.document.createElement('span');number.className='line-number';number.textContent=String(line.number);
            const text=this.document.createElement('span');text.className='code-text';row.append(number,text);return {line,row,text,key:null};
        });
        this.element.replaceChildren(...this.rows.map(r=>r.row));
        const legacy=this.rows.find(r=>r.row.offsetTop+r.row.offsetHeight>reading.top);
        const anchor=reading.anchor??(legacy?{line:legacy.line.number,byte:0,offset:reading.top-legacy.row.offsetTop}:null);
        this.update(false,true);if(anchor)this.restore(anchor);else this.element.scrollTop=reading.top;this.element.scrollLeft=reading.left;
    }
    setReflow(enabled){if(this.enabled===enabled)return;this.enabled=enabled;this.update(true);}
    selected(){const selection=this.document.getSelection();if(!selection||selection.isCollapsed)return false;for(let i=0;i<selection.rangeCount;i++)if(selection.getRangeAt(i).intersectsNode(this.element))return true;return false;}
    schedule(){if(this.closed||this.frame!==null)return;this.frame=this.view.requestAnimationFrame(()=>{this.frame=null;this.update(true);});}
    anchor(){
        const top=this.element.scrollTop;
        const row=this.rows.find(r=>r.row.offsetTop+r.row.offsetHeight>top);if(!row)return null;
        const segments=[...row.text.querySelectorAll('.reflow-segment')];const segment=segments.find(s=>row.row.offsetTop+s.offsetTop+s.offsetHeight>top)??segments.at(-1);
        return segment?{line:row.line.number,byte:Number(segment.dataset.start),offset:top-row.row.offsetTop-segment.offsetTop}:null;
    }
    reading(){return {top:this.element.scrollTop,left:this.element.scrollLeft,anchor:this.anchor()};}
    restore(anchor){const row=this.rows.find(r=>r.line.number===anchor.line),segment=row&&[...row.text.querySelectorAll('.reflow-segment')].find(s=>Number(s.dataset.start)<=anchor.byte&&(anchor.byte<Number(s.dataset.end)||Number(s.dataset.start)===Number(s.dataset.end)));if(segment)this.element.scrollTop=row.row.offsetTop+segment.offsetTop+Math.min(anchor.offset,Math.max(0,segment.offsetHeight-1));}
    update(preserve,force=false){
        if(this.closed||!this.rows.length)return;
        if(!force&&this.selected()){this.pending=true;return;}this.pending=false;
        const width=this.element.clientWidth-63;if(width<=0)return;
        const style=this.view.getComputedStyle(this.element);if(this.context){
        this.context.font=`${style.fontStyle} ${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;this.context.fontKerning='none';}
        const measure=text=>{const parts=text.split('\t'),tab=this.context.measureText('    ').width;let result=0;for(const [i,part] of parts.entries()){result+=this.context.measureText(part).width;if(i<parts.length-1)result=(Math.floor(result/tab)+1)*tab;}return result;};
        const anchor=preserve?this.anchor():null;let changed=false;
        for(const row of this.rows){const plan=this.enabled&&this.context?planReflow(row.line,width,measure):null,key=JSON.stringify(plan);if(key!==row.key){this.render(row.text,row.line,plan);row.key=key;changed=true;}}
        if(changed&&anchor)this.restore(anchor);
    }
    copySelection(event){
        const selection=this.document.getSelection();if(!event.clipboardData||!selection||selection.isCollapsed||selection.rangeCount!==1)return;
        const range=selection.getRangeAt(0);if(!this.element.contains(range.commonAncestorContainer))return;
        const rows=this.rows.filter(r=>range.intersectsNode(r.text));if(!rows.length||rows.some(r=>r.line.left||r.line.right))return;
        const pieces=[];
        for(const [index,row] of rows.entries()){
            const walker=this.document.createTreeWalker(row.text,4);let node,text='';
            while((node=walker.nextNode())){if(!node.parentElement.closest('.source-chunk')||!range.intersectsNode(node))continue;const start=range.startContainer===node?range.startOffset:0,end=range.endContainer===node?range.endOffset:node.length;text+=node.textContent.slice(start,end);}
            const contents=this.document.createRange();contents.selectNodeContents(row.text);
            if(index<rows.length-1||range.compareBoundaryPoints(2,contents)>0)text+=row.line.ending??'\n';pieces.push(text);
        }
        event.clipboardData.setData('text/plain',pieces.join(''));event.preventDefault();
    }
    dispose(){this.closed=true;this.resize.disconnect();if(this.frame!==null)this.view.cancelAnimationFrame(this.frame);this.document.removeEventListener('selectionchange',this.selection);this.document.removeEventListener('copy',this.copy);}
}
