// Only exact captured identifier spans become source actions.
export function callRanges(line, actions) {
    if(line.left)return [];
    const bytes=new TextEncoder().encode(line.text),decoder=new TextDecoder('utf-8',{fatal:true}),seen=new Set(),ranges=[];
    for(const action of actions){
        const p=action.position;
        if(!p||p.line!==line.number||typeof action.id!=='string'||action.id.length!==64||/[^0-9a-f]/.test(action.id)||!Number.isSafeInteger(p.start)||!Number.isSafeInteger(p.end)||p.start<0||p.start>=p.end||p.end>bytes.length)continue;
        let text;try{text=decoder.decode(bytes.subarray(p.start,p.end));}catch{continue;}
        if(text!==action.name)continue;
        const key=action.id+':'+p.start+':'+p.end;if(seen.has(key))continue;seen.add(key);
        ranges.push({id:action.id,name:action.name,start:p.start,end:p.end,action});
    }
    ranges.sort((a,b)=>a.start-b.start||a.end-b.end);
    const result=[];let group=[],end=-1;
    const finish=()=>{if(group.length===1)result.push(group[0]);};
    for(const range of ranges){if(range.start>=end){finish();group=[range];end=range.end;}else{group.push(range);end=Math.max(end,range.end);}}
    finish();return result;
}
