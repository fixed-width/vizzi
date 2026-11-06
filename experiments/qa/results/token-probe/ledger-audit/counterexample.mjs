import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {gunzipSync} from 'node:zlib';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
const records=[];
const spec={schema:1,kind:'response_format_ablation',repeats:2,caseIds:['fixture'],arms:[{id:'schema',constraint:'schema'},{id:'unconstrained',constraint:'none'}]};
const library={cases:[{id:'fixture',split:'development',repository:'fixture',question:'Where is the entry?',expected_status:'answered',required_facts:[{text:'Fixture'}]}]};
for(const [directory,id]of [['decoding','123'],['json-object','155']]){
 const path=`experiments/qa/results/${directory}/validation/mutations.json.gz`,report=JSON.parse(gunzipSync(await readFile(path))),file=report.files['experiments/qa/decoding-study.mjs'];
 const mutant=file.mutants.find(m=>m.id===id);assert.equal(mutant.status,'Survived');assert.equal(mutant.mutatorName,'CallExpression');
 const lines=file.source.split('\n'),offset=pos=>lines.slice(0,pos.line-1).reduce((n,l)=>n+l.length+1,0)+pos.column-1;
 const start=offset(mutant.location.start),end=offset(mutant.location.end),removed=file.source.slice(start,end);assert.ok(removed.includes('assert.equal(rows.length,plan.length)'));
 const mutated=file.source.slice(0,start)+mutant.replacement+file.source.slice(end);
 const load=async source=>{
  const adjusted=source.replace(/from '(\.\/[^']+)'/g,(_all,name)=>`from '${pathToFileURL(resolve('experiments/qa',name)).href}'`);
  return import('data:text/javascript;base64,'+Buffer.from(adjusted).toString('base64'));
 };
 const normal=await load(file.source),changed=await load(mutated),plan=normal.decodingPlan(library,spec);
 const messages=[{role:'user',content:'fixture'}],answer={answer:'Fixture',status:'answered',citations:[1]},body={messages,seed:42};
 const rows=plan.map(p=>({id:p.id,case:p.case.id,repeat:p.repeat,arm:p.arm.id,constraint:p.arm.constraint,split:'development',repository:'fixture',context:{snapshot:'s',evidence:[]},contextSha256:'same',messages,renderedPrompt:'same',request:p.arm.id==='schema'?{...body,response_format:{type:'json_object',schema:{type:'object'}}}:body,answer,raw:JSON.stringify(answer),provenanceValid:true,finish:'stop',usage:{prompt_tokens:1}}));
 rows.push({...rows[0],id:'unplanned'});
 const grades={grader:'counterexample fixture',limitations:[],grades:rows.map(r=>({id:r.id,facts:['supported'],unsupportedClaims:[],citationsSupport:true,roleConfusion:false,notes:'Synthetic scorer input; no model inference.'}))};
 assert.throws(()=>normal.compareDecoding(rows,library,spec,grades));
 const accepted=changed.compareDecoding(rows,library,spec,grades);assert.equal(accepted.pairs.length,2);
 records.push({archive:directory,report:path,sourceSha256:createHash('sha256').update(file.source).digest('hex'),mutantId:id,location:mutant.location,originalExpression:removed,mutation:mutant.replacement,expectedRows:4,providedRows:5,extraId:'unplanned',originalRejects:true,mutantAccepts:true,impact:'Prior equivalent classification was wrong; the count guard prevents silently accepting an extra unique graded row. Existing measured ledgers have exact planned counts, so model responses and reported QA scores are unchanged.'});
}
const result={schema:1,records};
if(process.argv[2])await writeFile(process.argv[2],JSON.stringify(result,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify(result,null,2));
