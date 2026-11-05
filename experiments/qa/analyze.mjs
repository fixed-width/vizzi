import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {json,libraryPath} from './support.mjs';
const args=process.argv.slice(2),option=name=>{const i=args.indexOf(name);return i<0?undefined:args[i+1];};
assert.ok(option('--input') && option('--output'));
const rows=(await readFile(option('--input'),'utf8')).trim().split('\n').filter(Boolean).map(line=>JSON.parse(line));
const cases=new Map((await json(libraryPath)).cases.map(c=>[c.id,c]));
const normalized=text=>text.replace(/\s+/gu,' ').trim();
function citationKind(row,citation){
 const source=row.context.evidence.find(e=>e.label===citation.label);
 if(!source)return 'unavailable_label';
 if(!citation.quote.trim())return 'empty_quote';
 if(source.text.includes(citation.quote))return 'exact';
 if(normalized(source.text).includes(normalized(citation.quote)))return 'whitespace_only';
 if(row.context.evidence.some(e=>e.label!==citation.label && normalized(e.text).includes(normalized(citation.quote))))return 'wrong_label';
 return 'edited_or_unsupported_quote';
}
const diagnostics=rows.map(row=>{
 const citations=row.answer?.citations??[],kinds=citations.map(c=>citationKind(row,c));
 const positive=row.answer?.status==='answered';
 return {id:row.id,model:row.model,condition:row.condition,case:row.case,expectedStatus:cases.get(row.case).expected_status,reportedStatus:row.answer?.status,strictSourceChecks:row.provenanceValid,citationKinds:kinds,whitespaceTolerantBinding:Boolean(row.answer && (!positive || citations.length>0) && kinds.every(k=>['exact','whitespace_only'].includes(k))),finish:row.finish,formatError:row.formatError,runtimeError:row.runtimeError};
});
const summaries=[];
for(const model of new Set(diagnostics.map(d=>d.model))){for(const condition of new Set(diagnostics.map(d=>d.condition))){
 const group=diagnostics.filter(d=>d.model===model && d.condition===condition);if(!group.length)continue;
 summaries.push({model,condition,cases:group.length,strictSourceChecks:group.filter(d=>d.strictSourceChecks).length,whitespaceTolerantBinding:group.filter(d=>d.whitespaceTolerantBinding).length,citationKinds:group.flatMap(d=>d.citationKinds).reduce((counts,k)=>(counts[k]=(counts[k]??0)+1,counts),{}),answerableRefusals:group.filter(d=>d.expectedStatus==='answered' && d.reportedStatus==='insufficient_evidence').length,unanswerableAnswered:group.filter(d=>d.expectedStatus==='insufficient_evidence' && d.reportedStatus==='answered').length,truncated:group.filter(d=>d.finish==='length').length});
}}
await writeFile(option('--output'),JSON.stringify({schema:1,postHocDiagnostic:true,limitations:['Whitespace-tolerant binding is diagnostic only; frozen strict results and raw responses are unchanged.','Neither literal nor whitespace-normalized source binding establishes semantic entailment or answer correctness.'],summaries,diagnostics},null,2)+'\n');console.log(JSON.stringify(summaries));
