import assert from 'node:assert/strict';
import {mkdir,writeFile,appendFile,stat} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {cpus,platform,arch,totalmem} from 'node:os';
import {prepare,json,digest,modelPath,coverage,changing} from './support.mjs';
import {casesPath,inputs,verifyPrepared,freezeInputs,verifyHashes,indexesFor,retrievalReport,evidenceHash} from './compact-support.mjs';
import {server,generate,schemaFor} from './model.mjs';

const [command,...args]=process.argv.slice(2);
function option(name,required=true){const i=args.indexOf(name);if(i<0){assert.ok(!required,`missing ${name}`);return undefined;}assert.ok(args[i+1] && !args[i+1].startsWith('--'),`missing value for ${name}`);return args[i+1];}
const output=resolve(option('--output'));
const save=(path,value)=>writeFile(path,JSON.stringify(value,null,2)+'\n',{flag:'wx'});
if(command==='prepare'){
 const result=await prepare(output,{'vizzi':resolve(option('--vizzi-root',false)??'.'),'lpc-rs':resolve(option('--lpc-root'))},{questionsPath:casesPath,retrieveHeldOut:false});
 console.log(JSON.stringify({sources:Object.keys(result.sources),audited:result.checks.length,freshRetrievalInspected:false}));
}else if(command==='development'){
 const prepared=await verifyPrepared(resolve(option('--prepared'))),{library}=await inputs();
 const report=await retrievalReport(prepared,library,'development');await save(output,report);console.log(JSON.stringify(report.summaries));
}else if(command==='freeze'){
 const freeze=await freezeInputs(resolve(option('--prepared')));await save(output,freeze);console.log(`Frozen ${Object.keys(freeze.files).length} input hashes in ${output}`);
}else if(command==='held-out'){
 const freeze=await json(resolve(option('--freeze')));await verifyHashes(freeze.files);
 const prepared=await verifyPrepared(freeze.preparedPath),{library}=await inputs();
 const report=await retrievalReport(prepared,library,'held_out');report.freezeSha256=await digest(resolve(option('--freeze')));await save(output,report);console.log(JSON.stringify(report.summaries));
}else if(command==='run'){
 const freezePath=resolve(option('--freeze')),freeze=await json(freezePath);await verifyHashes(freeze.files);
 const prepared=await verifyPrepared(freeze.preparedPath),{library,cohort}=await inputs(),models=await json(modelPath);
 const model=models.models.find(m=>m.id===cohort.model);assert.ok(model);
 const runtime=resolve(option('--runtime')),modelFile=join(resolve(option('--models-root')),model.directory,model.filename);
 assert.equal((await stat(modelFile)).size,model.bytes);assert.equal(await digest(modelFile),model.sha256);
 assert.equal(await digest(runtime),cohort.runtime_binary_sha256,'runtime differs from pinned artifact');
 await mkdir(output);
 const manifest={schema:1,createdAt:new Date().toISOString(),freezeSha256:await digest(freezePath),qaBinarySha256:prepared.qaBinarySha256,questionsSha256:prepared.questionsSha256,model,limits:models.limits,runtime:models.runtime,runtimeSha256:await digest(runtime),sources:prepared.sources,cohort,schemas:Object.fromEntries(cohort.arms.map(arm=>[arm.protocol,schemaFor(arm.protocol)])),environment:{cpu:cpus()[0].model,os:platform(),arch:arch(),physicalMemoryBytes:totalmem()},completed:0,incomplete:true,limitations:['Controlled 23-question subset, not a representative benchmark. Eight fresh questions are agent-authored with knowledge of the source.','The compact/legacy comparison bundles retrieval, excerpt size and metadata; the compact quote/ID comparison holds contexts fixed.','Canonical citation identity does not establish claim support. Case-author semantic grading is separate.','Cache reuse is enabled as in v1; arm order rotates by case. No separate cold-cache latency qualification.','CPU-only two-thread serial inference; observed memory is not an enforced device qualification.']};
 await save(join(output,'manifest.json'),manifest);
 const indexes=await indexesFor(prepared);let host;
 try{
  host=await server(runtime,modelFile,models.limits,join(output,'runtime.log'));
  await save(join(output,'runtime.json'),{pid:host.pid,args:host.args,startupMs:host.startupMs});
  const records=[];
  for(let caseNumber=0;caseNumber<cohort.case_ids.length;caseNumber++){
   const item=library.cases.find(c=>c.id===cohort.case_ids[caseNumber]),index=indexes[item.repository];let update;
   if(item.fixture_state){await changing(prepared.sources.changing.root,item.fixture_state);update=await index.request({op:'refresh'});assert.ok(!update.error,update.error);}
   const contexts=new Map();
   for(const arm of cohort.arms){
    const id=`${item.id}/${arm.id}`,response=await index.request({op:'retrieve',id,question:item.question,selector:arm.selector,limit:arm.limit,budget:arm.budget});
    assert.ok(!response.error,response.error);contexts.set(arm.id,response);
   }
   assert.equal(evidenceHash(contexts.get('compact_quotes').context),evidenceHash(contexts.get('compact_ids').context),'compact citation arms must hold evidence fixed');
   const offset=caseNumber%cohort.arms.length,order=[...cohort.arms.slice(offset),...cohort.arms.slice(0,offset)];
   for(const arm of order){
    const id=`${item.id}/${arm.id}`,response=contexts.get(arm.id),context=response.context;
    const record={id,case:item.id,repository:item.repository,split:item.split,model:model.id,arm:arm.id,protocol:arm.protocol,order:order.map(a=>a.id),contextSha256:evidenceHash(context),context,retrievalMs:response.elapsed_ms,evidenceBytes:context.evidence.reduce((n,e)=>n+Buffer.byteLength(e.text),0),anchorCoverage:coverage(item,context),update};
    try{
     Object.assign(record,await generate(host,item,context,models.limits,{protocol:arm.protocol,metadata:arm.selector==='compact'}));
     if(record.answer){const validation=await index.request({op:arm.protocol==='ids'?'validate_ids':'validate',id,answer:record.answer});record.provenanceValid=validation.provenance_valid===true;record.provenanceError=validation.error;record.references=validation.references;}
     else record.provenanceValid=false;
    }catch(e){record.runtimeError=e.message;record.provenanceValid=false;}
    records.push(record);await appendFile(join(output,'answers.jsonl'),JSON.stringify(record)+'\n');
    manifest.completed=records.length;await writeFile(join(output,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
    console.log(`${records.length}/${cohort.case_ids.length*cohort.arms.length} ${id}: ${record.answer?.status??'error'}; ${Math.round(record.totalMs??0)} ms; source checks ${record.provenanceValid?'pass':'fail'}`);
   }
  }
  manifest.incomplete=false;await writeFile(join(output,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
 }finally{await host?.close();for(const index of Object.values(indexes))await index.close();}
}else throw new Error('command must be prepare, development, freeze, held-out or run');
