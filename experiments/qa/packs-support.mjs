import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';

export function lineRange(bytes,first,last){
 assert.ok(Number.isInteger(first) && Number.isInteger(last) && first>=1 && last>=first,'invalid source lines');
 const starts=[0];for(let i=0;i<bytes.length;i++)if(bytes[i]===10 && i+1<bytes.length)starts.push(i+1);
 assert.ok(last<=starts.length && bytes.length>0,'source line is unavailable');
 return {start:starts[first-1],end:starts[last]??bytes.length};
}
export function checkedRange(bytes,selection){
 assert.ok(selection.path && !selection.path.startsWith('/') && !selection.path.split('/').some(p=>p==='..' || p===''),'invalid source path');
 assert.match(selection.content,/^[a-f0-9]{64}$/);
 assert.equal(createHash('sha256').update(bytes).digest('hex'),selection.sha256,`source changed: ${selection.path}`);
 return {path:selection.path,content:selection.content,...lineRange(bytes,selection.first,selection.last)};
}
export function checkAudit(item,context){
 for(const fact of item.required_facts){
  assert.ok(fact.evidence.some(anchor=>context.evidence.some(e=>e.path===anchor.path && e.text.includes(anchor.needle))),`missing required source linkage: ${item.id} ${fact.evidence[0].needle}`);
 }
 for(const anchor of item.supporting_evidence??[])assert.ok(context.evidence.some(e=>e.path===anchor.path && e.text.includes(anchor.needle)),`missing supporting source linkage: ${item.id} ${anchor.needle}`);
 return {complete:true,facts:item.required_facts.length,supportingAnchors:item.supporting_evidence?.length??0,limitation:'Literal anchors supplement the manual whole-definition/caller/branch audit; they do not prove semantic completeness.'};
}
