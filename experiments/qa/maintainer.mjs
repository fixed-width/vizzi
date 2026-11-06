// A subsequent question cohort reuses the frozen v2 native selector and model adapter.
import assert from 'node:assert/strict';
import {mkdir,writeFile,appendFile,stat} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {cpus,platform,arch,totalmem} from 'node:os';
import {json,digest,corpus,anchors,coverage,modelPath} from './support.mjs';
import {captureHashes,verifyHashes,verifyPrepared,indexesFor,evidenceHash,cohortPath} from './compact-support.mjs';
import {server,generate} from './model.mjs';
import {validateMaintainer} from './maintainer-cases.mjs';
const directory=resolve('experiments/qa/library/maintainer-2026-11-05');
const questionsPath=join(directory,'questions.json'),libraryPath=join(directory,'cases.json');
const [command,...args]=process.argv.slice(2);
function option(name){const i=args.indexOf(name);assert.ok(i>=0 && args[i+1] && !args[i+1].startsWith('--'),`missing ${name}`);return resolve(args[i+1]);}
const output=option('--output');
const save=(path,value)=>writeFile(path,JSON.stringify(value,null,2)+'\n',{flag:'wx'});
const library=await json(libraryPath),cases=validateMaintainer(await json(questionsPath),library),cohort=await json(cohortPath);
let freeze,baseFreezePath;
if(command==='prepare')baseFreezePath=option('--base-freeze');
else {freeze=await json(option('--freeze'));await verifyHashes(freeze.files);baseFreezePath=freeze.baseFreezePath;assert.equal(await digest(baseFreezePath),freeze.baseFreezeSha256);}
const base=await json(baseFreezePath);await verifyHashes(base.files);
const prepared=await verifyPrepared(base.preparedPath);
assert.equal(library.sources['lpc-rs'].revision,prepared.sources['lpc-rs'].revision);
if(command==='prepare'){
 const index=await corpus(prepared.sources['lpc-rs'].root),audit=[];
 try{
  assert.equal(index.ready.snapshot,prepared.sources['lpc-rs'].initial.snapshot);
  for(const item of cases){
   for(const [i,anchor]of anchors(item).entries()){
    const checked=await index.request({op:'oracle',id:`audit/${item.id}/${i}`,anchors:[anchor]});assert.ok(!checked.error,`${item.id}: ${checked.error}`);
   }
   audit.push({id:item.id,sourceAnchors:anchors(item).length,audited:true,candidateRetrievalInspected:false});
  }
 }finally{await index.close();}
 await save(output,{schema:1,createdAt:new Date().toISOString(),baseFreezePath,baseFreezeSha256:await digest(baseFreezePath),files:await captureHashes([questionsPath,libraryPath,'experiments/qa/maintainer.mjs','experiments/qa/maintainer-cases.mjs']),audit,conditions:'Same fixed native selector, prompt adapter, runtime, 2B weights, resource limits and three arms as the v2 freeze. No tuning on maintainer questions.'});
 console.log(`Audited and froze ${cases.length} maintainer questions without candidate retrieval.`);
}else if(command==='run'){
 const models=await json(modelPath),model=models.models.find(m=>m.id===cohort.model),runtime=option('--runtime'),weights=join(option('--models-root'),model.directory,model.filename);
 assert.equal((await stat(weights)).size,model.bytes);assert.equal(await digest(weights),model.sha256);assert.equal(await digest(runtime),cohort.runtime_binary_sha256);
 await mkdir(output);
 const manifest={schema:1,createdAt:new Date().toISOString(),freezeSha256:await digest(option('--freeze')),baseFreezeSha256:freeze.baseFreezeSha256,questionsSha256:await digest(questionsPath),casesSha256:await digest(libraryPath),model,runtime:models.runtime,runtimeSha256:await digest(runtime),limits:models.limits,qaBinarySha256:prepared.qaBinarySha256,sources:prepared.sources,arms:cohort.arms,caseIds:cases.map(c=>c.id),environment:{cpu:cpus()[0].model,os:platform(),arch:arch(),physicalMemoryBytes:totalmem()},completed:0,incomplete:true,limitations:['Three exact maintainer-authored workflow questions, not a representative benchmark. Gold facts and grading remain source-aware agent work.','Retrieval/model/prompt are unchanged from the v2 freeze; question scope notes and gold facts are never supplied to inference.','Same 100-word prompt and 384-token output allowance, including the all-checks trace task.','Source binding is separate from claim support and task completeness. No GPU or source execution.']};
 await save(join(output,'manifest.json'),manifest);
 const indexes=await indexesFor(prepared);let host;
 try{
  host=await server(runtime,weights,models.limits,join(output,'runtime.log'));await save(join(output,'runtime.json'),{pid:host.pid,args:host.args,startupMs:host.startupMs});
  for(const [number,item]of cases.entries()){
   const index=indexes[item.repository],contexts=new Map();
   for(const arm of cohort.arms){const id=`${item.id}/${arm.id}`,response=await index.request({op:'retrieve',id,question:item.question,selector:arm.selector,limit:arm.limit,budget:arm.budget});assert.ok(!response.error,response.error);contexts.set(arm.id,response);}
   assert.equal(evidenceHash(contexts.get('compact_quotes').context),evidenceHash(contexts.get('compact_ids').context));
   const order=[...cohort.arms.slice(number%3),...cohort.arms.slice(0,number%3)];
   for(const arm of order){
    const id=`${item.id}/${arm.id}`,response=contexts.get(arm.id),context=response.context;
    const record={id,case:item.id,repository:item.repository,split:item.split,model:model.id,arm:arm.id,protocol:arm.protocol,order:order.map(a=>a.id),contextSha256:evidenceHash(context),context,retrievalMs:response.elapsed_ms,evidenceBytes:context.evidence.reduce((n,e)=>n+Buffer.byteLength(e.text),0),anchorCoverage:coverage(item,context)};
    try{
     Object.assign(record,await generate(host,item,context,models.limits,{protocol:arm.protocol,metadata:arm.selector==='compact'}));
     if(record.answer){const validation=await index.request({op:arm.protocol==='ids'?'validate_ids':'validate',id,answer:record.answer});record.provenanceValid=validation.provenance_valid===true;record.provenanceError=validation.error;record.references=validation.references;}else record.provenanceValid=false;
    }catch(e){record.runtimeError=e.message;record.provenanceValid=false;}
    await appendFile(join(output,'answers.jsonl'),JSON.stringify(record)+'\n');manifest.completed++;await writeFile(join(output,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
    console.log(`${manifest.completed}/${cases.length*3} ${id}: ${record.answer?.status??'error'}; ${Math.round(record.totalMs??0)} ms; source checks ${record.provenanceValid?'pass':'fail'}`);
   }
  }
  manifest.incomplete=false;await writeFile(join(output,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
 }finally{await host?.close();for(const index of Object.values(indexes))await index.close();}
}else throw new Error('command must be prepare or run');
