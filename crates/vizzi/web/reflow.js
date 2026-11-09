// Layout uses captured byte offsets; language rules stay in the native hint provider.
export function planReflow(line,width,measure,margin={bytes:0,columns:0}) {
    const bytes=new TextEncoder().encode(line.text),original=[{start:0,end:bytes.length,indent:0}],h=line.reflow;
    if(line.left||!h||!Array.isArray(h.points)||h.points.length>256||!h.points.length||bytes.length>4096||!Number.isFinite(width)||width<=0)return original;
    const indices=new Map([[0,0]]);let at=0,index=0;
    for(const char of line.text){at+=new TextEncoder().encode(char).length;index+=char.length;indices.set(at,index);}
    if(!indices.has(h.leading)||!/^[ \t]*$/.test(line.text.slice(0,indices.get(h.leading))))return original;
    let previous=h.leading;
    for(const point of h.points){if(!point||point.offset<=previous||point.offset>=bytes.length||!indices.has(point.offset)||!Number.isInteger(point.priority)||point.priority<0||point.priority>8||(point.anchor!=null&&(point.anchor>point.offset||!indices.has(point.anchor))))return original;previous=point.offset;}
    try {
        const cache=new Map();let work=0;
        const measured=text=>{if(!cache.has(text)){if(++work>4096)throw Error('Reflow work allowance');const value=measure(text);if(!Number.isFinite(value)||value<0)throw Error('Invalid font measure');cache.set(text,value);}return cache.get(text);};
        const slice=(start,end)=>start===0?(end===0?'':' '.repeat(margin.columns)+line.text.slice(indices.get(Math.min(margin.bytes,end)),indices.get(end))):line.text.slice(indices.get(start),indices.get(end));
        const size=(start,end)=>measured(slice(start,end).replace(/[ \t]+$/,''));
        const base=measured(slice(0,h.leading)),step=measured('    ');
        const indentation=point=>{const aligned=point.anchor==null?Infinity:measured(slice(0,point.anchor));return aligned<=width/2?aligned:Math.min(base+step,width/3);};
        const result=[];let start=0,indent=0,cursor=0;
        for(let row=0;row<64;row++){
            if(indent+size(start,bytes.length)<=width){result.push({start,end:bytes.length,indent});return result;}
            let best=null,score=Infinity,overflow=null;
            for(let i=cursor;i<h.points.length;i++){
                const point=h.points[i],used=size(start,point.offset);
                if(indent+used>width){overflow={point,index:i};break;}
                const cost=point.priority/8+(1-used/Math.max(1,width-indent))**2;
                if(cost<=score){score=cost;best={point,index:i};}
            }
            const chosen=best??overflow;
            if(!chosen){result.push({start,end:bytes.length,indent});return result;}
            result.push({start,end:chosen.point.offset,indent});start=chosen.point.offset;indent=indentation(chosen.point);cursor=chosen.index+1;
        }
        return original;
    }catch{return original;}
}
