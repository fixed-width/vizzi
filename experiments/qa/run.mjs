import assert from 'node:assert/strict';
import {readFile,writeFile,appendFile,mkdir,stat} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {cpus,platform,arch,totalmem} from 'node:os';
import {corpus,json,digest,libraryPath,templatesPath,modelPath,validateLibrary,anchors,coverage,changing} from './support.mjs';
import {server,generate,systemPrompt,schema} from './model.mjs';
const args=process.argv.slice(2),option=name=>args[args.indexOf(name)+1];
assert.ok(args.includes('--prepared') && args.includes('--runtime') && args.includes('--models-root') && args.includes('--output'));
const prepared=await json(resolve(option('--prepared'))),models=await json(modelPath),library=validateLibrary(await json(libraryPath),await json(templatesPath));
assert.equal(prepared.questionsSha256,await digest(libraryPath),'questions changed after preparation');assert.equal(prepared.modelsSha256,await digest(modelPath));assert.equal(prepared.qaBinarySha256,await digest('target/release/vizzi-qa'),'native binary changed; repeat source audit');
const output=resolve(option('--output')),runtime=resolve(option('--runtime')),modelsRoot=resolve(option('--models-root'));
await mkdir(output);
const manifest={schema:1,createdAt:new Date().toISOString(),questionsSha256:prepared.questionsSha256,templatesSha256:prepared.templatesSha256,qaBinarySha256:prepared.qaBinarySha256,runtimeSha256:await digest(runtime),runtime:models.runtime,limits:models.limits,environment:{cpu:cpus()[0].model,os:platform(),arch:arch(),physicalMemoryBytes:totalmem()},systemPrompt,schemaForModel:schema,modelDefinitions:models.models,sources:prepared.sources,caseCount:library.cases.length,completed:0,incomplete:true,limitations:['Lexical retrieval baseline; no embeddings, resolved semantic graph or model fine-tuning.','Oracle condition uses gold-selected source, never gold prose answers; it is diagnostic rather than a deployable retriever.','Semantic scoring is separate rubric review, with its author and limitations recorded; the runner has no automated judge API.','Sampled process memory is not an enforced 16 GB laptop qualification.']};
await writeFile(join(output,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
const records=[];
let activeServer;const indexes={};
try{
 for(const [name,source]of Object.entries(prepared.sources)){if(name==='changing')await changing(source.root,'base');indexes[name]=await corpus(source.root);}
 for(const model of models.models){
  if(args.includes('--model') && model.id!==option('--model'))continue;
  const file=join(modelsRoot,model.directory,model.filename);
  assert.equal((await stat(file)).size,model.bytes);assert.equal(await digest(file),model.sha256,'model weights differ from the pinned artifact');
  activeServer=await server(runtime,file,models.limits,join(output,model.id+'.runtime.log'));
  await writeFile(join(output,model.id+'.runtime.json'),JSON.stringify({pid:activeServer.pid,args:activeServer.args,startupMs:activeServer.startupMs},null,2)+'\n');
  let completed=0;
  for(const item of library.cases){
   const index=indexes[item.repository];let update;
   if(item.fixture_state){await changing(prepared.sources.changing.root,item.fixture_state);update=await index.request({op:'refresh'});assert.ok(!update.error,update.error);}
   for(const condition of ['retrieved',...(item.oracle_diagnostic?['oracle_source']:[])]){
    const id=`${model.id}/${item.id}/${condition}`;
    const retrieved=await index.request(condition==='retrieved'?{op:'retrieve',id,question:item.question}:{op:'oracle',id,anchors:anchors(item)});
    assert.ok(!retrieved.error,`${id}: ${retrieved.error}`);
    const context=retrieved.context;
    const record={id,case:item.id,repository:item.repository,split:item.split,model:model.id,condition,sourceSnapshot:context.snapshot,retrievalMs:retrieved.elapsed_ms,anchorCoverage:coverage(item,context),evidenceBytes:context.evidence.reduce((n,e)=>n+Buffer.byteLength(e.text),0),context,update};
    try{
     Object.assign(record,await generate(activeServer,item,context,models.limits));
     if(record.answer){const validation=await index.request({op:'validate',id,answer:record.answer});record.provenanceValid=validation.provenance_valid===true;record.provenanceError=validation.error;}
     else record.provenanceValid=false;
    }catch(e){record.runtimeError=e.message;record.provenanceValid=false;}
    records.push(record);await appendFile(join(output,'answers.jsonl'),JSON.stringify(record)+'\n');
    manifest.completed=records.length;await writeFile(join(output,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
    console.log(`${++completed}: ${model.id} ${item.id} ${condition}: ${record.answer?.status??'error'}; ${Math.round(record.totalMs??0)} ms; source checks ${record.provenanceValid?'pass':'fail'}`);
   }
  }
  await activeServer.close();activeServer=undefined;
 }
 manifest.incomplete=false;await writeFile(join(output,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
 const summaries=[];
 for(const model of models.models){for(const condition of ['retrieved','oracle_source']){
  const group=records.filter(r=>r.model===model.id && r.condition===condition);if(!group.length)continue;
  const times=group.map(r=>r.totalMs).filter(Number.isFinite).sort((a,b)=>a-b),ttfts=group.map(r=>r.firstAnswerMs).filter(Number.isFinite).sort((a,b)=>a-b);
  summaries.push({model:model.id,condition,cases:group.length,runtimeErrors:group.filter(r=>r.runtimeError).length,formatErrors:group.filter(r=>r.formatError).length,provenanceValid:group.filter(r=>r.provenanceValid).length,answered:group.filter(r=>r.answer?.status==='answered').length,abstained:group.filter(r=>r.answer?.status==='insufficient_evidence').length,allAnchors:group.filter(r=>r.anchorCoverage.all===true).length,medianMs:times[Math.floor(times.length/2)],p95Ms:times[Math.ceil(times.length*.95)-1],medianFirstAnswerMs:ttfts[Math.floor(ttfts.length/2)],semanticCorrectness:'requires rubric grading'});
 }}
 await writeFile(join(output,'summary.json'),JSON.stringify({schema:1,summaries},null,2)+'\n');console.log(JSON.stringify(summaries));
}finally{await activeServer?.close();for(const index of Object.values(indexes))await index.close();}
