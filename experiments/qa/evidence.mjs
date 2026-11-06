import assert from 'node:assert/strict';
import {mkdir,readFile,writeFile,appendFile,readdir} from 'node:fs/promises';
import {spawnSync} from 'node:child_process';
import {join,resolve,dirname} from 'node:path';
import {cpus,totalmem} from 'node:os';
import {json,digest,corpus} from './support.mjs';
import {captureHashes,verifyHashes,evidenceHash} from './compact-support.mjs';
import {checkedRange,checkAudit} from './packs-support.mjs';
import {chooseModel} from './model-config.mjs';
import {server,generate,schemaFor,generationRequest} from './model.mjs';
import {validateEvidenceStudy,developmentRuns,modelQuestion} from './evidence-study.mjs';
import {decodingPlan,responseShape} from './decoding-study.mjs';

const [command,...args]=process.argv.slice(2),allowed=new Set(['--output','--prepared','--runtime','--models-root','--pytorch','--vizzi','--decoding']);
assert.ok(['prepare','run'].includes(command));
for(let i=0;i<args.length;i+=2)assert.ok(allowed.has(args[i]) && args[i+1] && !args[i+1].startsWith('--'),'unknown/missing argument');
const option=(name,required=true)=>{const i=args.indexOf(name);assert.ok(i>=0 || !required,`missing ${name}`);return i<0?undefined:resolve(args[i+1]);};
const libraryRoot=resolve('experiments/qa/library/evidence-use'),configPath=join(libraryRoot,'cohort.json'),developmentPath=join(libraryRoot,'development.json'),reservedPath=join(libraryRoot,'reserved.json');
const config=await json(configPath),development=await json(developmentPath),models=await json(config.models),model=chooseModel(models,config.model);
const decodingPath=option('--decoding',false),decoding=decodingPath?await json(decodingPath):undefined;
const plan=decodingPath?decodingPlan(development,decoding):developmentRuns(development,config.repeats),caseIds=[...new Set(plan.map(p=>p.case.id))];
const output=option('--output'),limits={...models.limits,threads:2,context_tokens:8192,output_tokens:1024,request_timeout_ms:360000,batch_tokens:512,ubatch_tokens:128};
const save=(path,value)=>writeFile(path,JSON.stringify(value,null,2)+'\n',{flag:'wx'});
async function select(index,source,item,id){
 const expected=[],ranges=[];
 for(const range of item.ranges){
  const bytes=await readFile(join(source.root,range.path));
  const selected=checkedRange(bytes,{...range,sha256:config.sources[item.source].files[range.path].sha256,content:source.identities[range.path]});
  expected.push(bytes.subarray(selected.start,selected.end).toString('utf8'));ranges.push(selected);
 }
 const response=await index.request({op:'select',id,snapshot:index.ready.snapshot,ranges,budget:16000});assert.ok(!response.error,response.error);
 assert.deepEqual(response.context.evidence.map(e=>e.text),expected);return response.context;
}
if(command==='prepare'){
 const reserved=await json(reservedPath);validateEvidenceStudy(config,development,reserved);
 await mkdir(output);const sources={},inputs=[],reservedInputs=[],audit=[];
 for(const [name,source]of Object.entries(config.sources)){
  const root=join(output,'sources',name);await mkdir(root,{recursive:true});
  for(const [path,file]of Object.entries(source.files)){
   let bytes;
   if(source.kind==='fixture')bytes=await readFile(resolve(source.root,path));
   else{
    const repository=option('--'+name,name!=='vizzi')??resolve('.');
    const result=spawnSync('git',['-C',repository,'show',`${source.revision}:${path}`],{maxBuffer:16*1024*1024,env:{...process.env,GIT_OPTIONAL_LOCKS:'0'}});
    assert.equal(result.status,0,result.stderr?.toString());bytes=result.stdout;
   }
   const target=join(root,path);await mkdir(dirname(target),{recursive:true});await writeFile(target,bytes,{flag:'wx'});assert.equal(await digest(target),file.sha256,'pinned source bytes changed');
  }
  const index=await corpus(root);sources[name]={root,revision:source.revision,initial:index.ready,identities:{}};
  try{
   for(const [path,file]of Object.entries(source.files)){
    const response=await index.request({op:'oracle',id:`identity/${name}/${path}`,anchors:[[path,file.identityNeedle]]});assert.ok(!response.error,`${name}/${path}: ${response.error}`);
    const evidence=response.context.evidence.find(e=>e.path===path);assert.ok(evidence,`source not captured: ${path}`);sources[name].identities[path]=evidence.content;
   }
   for(const item of [...development.cases,...reserved.cases].filter(c=>c.source===name)){
    const context=await select(index,sources[name],item,item.id);checkAudit(item,context);
    const record={case:item.id,question:modelQuestion(item),context,contextSha256:evidenceHash(context)};
    (item.split==='development'?inputs:reservedInputs).push(record);
    audit.push({id:item.id,split:item.split,source:name,rangeKeys:item.ranges.map(r=>r.key),complete:true,expectedStatus:item.expected_status,facts:item.required_facts.length,minimumSupportingLabels:item.expected_support.map(key=>item.ranges.findIndex(r=>r.key===key)+1)});
   }
  }finally{await index.close();}
 }
 inputs.sort((a,b)=>development.cases.findIndex(c=>c.id===a.case)-development.cases.findIndex(c=>c.id===b.case));
 await save(join(output,'inputs.json'),inputs);await save(join(output,'reserved.json'),{generations:0,inputs:reservedInputs});await save(join(output,'audit.json'),audit);
 const rust=(await readdir('crates/vizzi-qa/src')).filter(p=>p.endsWith('.rs')).map(p=>'crates/vizzi-qa/src/'+p);
 const sourceFiles=Object.entries(config.sources).flatMap(([name,s])=>Object.keys(s.files).map(path=>join(sources[name].root,path)));
 const files=await captureHashes([...rust,'Cargo.lock','crates/vizzi-qa/Cargo.toml',configPath,developmentPath,reservedPath,config.models,'docs/qa/EVIDENCE_USE.md',...(decodingPath?[decodingPath,'docs/qa/DECODING.md']:[]),...['evidence.mjs','evidence-study.mjs','decoding-study.mjs','model.mjs','model-config.mjs','runtime-profile.mjs','support.mjs','compact-support.mjs','compact-assess.mjs','packs-support.mjs'].map(p=>'experiments/qa/'+p),'target/release/vizzi-qa','target/release/vizzi-envelope',...sourceFiles,...['inputs.json','reserved.json','audit.json'].map(p=>join(output,p))]);
 await save(join(output,'prepared.json'),{schema:1,createdAt:new Date().toISOString(),sources,model,limits,files,caseIds,decodingSha256:decodingPath?await digest(decodingPath):null,reservedCases:reserved.cases.length,expectedResponses:plan.length,reservedGenerations:0,limitations:['Agent-authored synthetic development fixtures; real-source reserved questions have no model outputs.','Preparation audits gold/source; inference uses development questions/source only.','Greedy repeats and prefix caching are diagnostic replays, not independent samples.']});
 console.log(`Audited ${inputs.length} development and ${reservedInputs.length} reserved contexts; zero generations.`);
}else{
 const preparedPath=option('--prepared'),prepared=await json(preparedPath);await verifyHashes(prepared.files);assert.deepEqual(prepared.model,model);assert.deepEqual(prepared.limits,limits);
 const inputs=await json(join(dirname(preparedPath),'inputs.json'));
 assert.deepEqual(prepared.caseIds,caseIds);assert.equal(prepared.decodingSha256,decodingPath?await digest(decodingPath):null);assert.equal(plan.length,prepared.expectedResponses);assert.equal(prepared.reservedGenerations,0);
 const runtime=option('--runtime'),weights=join(option('--models-root'),model.directory,model.filename);assert.equal(await digest(weights),model.sha256);assert.equal(await digest(runtime),config.runtime_binary_sha256);
 await mkdir(output);const indexes={};let host;
 const manifest={schema:1,createdAt:new Date().toISOString(),preparedSha256:await digest(preparedPath),model,runtimeSha256:await digest(runtime),limits,condition:'unaltered base; no prompt/weight tune',decodingSha256:prepared.decodingSha256,repeats:config.repeats,caseIds:prepared.caseIds,reservedGenerations:0,environment:{cpu:cpus()[0].model,physicalMemoryBytes:totalmem()},completed:0,incomplete:true};await save(join(output,'manifest.json'),manifest);
 try{
  for(const source of new Set(plan.map(p=>p.case.source))){indexes[source]=await corpus(prepared.sources[source].root);assert.equal(indexes[source].ready.snapshot,prepared.sources[source].initial.snapshot);}
  host=await server(runtime,weights,limits,join(output,'runtime.log'));await save(join(output,'runtime.json'),{pid:host.pid,args:host.args,startupMs:host.startupMs});
  const rendered=new Map();
  if(decoding){
   for(const id of caseIds){
    const item=development.cases.find(c=>c.id===id),input=inputs.find(i=>i.case===id),prompts=[],requests=[];
    for(const arm of decoding.arms){
     const request=generationRequest(modelQuestion(item),input.context,limits,{protocol:'ids',metadata:true,wordLimit:300,citationLimit:8,fieldOrder:'answer_first',constraint:arm.constraint});
     const response=await fetch(host.url+'/apply-template',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(request),signal:AbortSignal.timeout(limits.request_timeout_ms)});assert.ok(response.ok,`prompt rendering failed: ${response.status}`);
     const value=await response.json();assert.ok(typeof value.prompt==='string' && value.prompt.length);rendered.set(id+'/'+arm.id,{request,prompt:value.prompt});prompts.push(value.prompt);requests.push(request);
    }
    assert.equal(prompts[0],prompts[1],'response_format changes rendered prompt');const {response_format,...without}=requests[0];assert.ok(response_format);assert.deepEqual(without,requests[1]);
   }
   await save(join(output,'rendered.json'),Object.fromEntries(rendered));
  }
  for(const p of plan){
   const item=p.case,index=indexes[item.source],context=await select(index,prepared.sources[item.source],item,p.id),input=inputs.find(i=>i.case===item.id);
   assert.deepEqual(context,input.context);assert.deepEqual(modelQuestion(item),input.question);
   const row={id:p.id,case:item.id,repeat:p.repeat,family:item.family,condition:item.condition,source:item.source,repository:item.repository,split:'development',model:model.id,arm:p.arm?.id??'answer_first',constraint:p.arm?.constraint??'schema',fieldOrder:'answer_first',protocol:'ids',context,contextSha256:evidenceHash(context),wordLimit:300,outputTokenLimit:1024,evidenceBytes:context.evidence.reduce((n,e)=>n+Buffer.byteLength(e.text),0)};
   if(decoding){const value=rendered.get(item.id+'/'+p.arm.id);row.request=value.request;row.renderedPrompt=value.prompt;}
   try{
    Object.assign(row,await generate(host,modelQuestion(item),context,limits,{protocol:'ids',metadata:true,wordLimit:300,citationLimit:8,fieldOrder:'answer_first',constraint:row.constraint}));
    row.orderCompliant=JSON.stringify(Object.keys(row.answer??{}))===JSON.stringify(Object.keys(schemaFor('ids',8,'answer_first').properties));
    if(decoding)row.responseShape=responseShape(row.answer);
    if(row.answer){const validation=await index.request({op:'validate_ids',id:p.id,answer:row.answer});row.provenanceValid=validation.provenance_valid===true;row.references=validation.references;row.provenanceError=validation.error;}else row.provenanceValid=false;
   }catch(e){row.runtimeError=e.message;row.provenanceValid=false;}
   await appendFile(join(output,'answers.jsonl'),JSON.stringify(row)+'\n');manifest.completed++;await writeFile(join(output,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
   console.log(`${manifest.completed}/${plan.length} ${p.id}: ${row.answer?.status??'error'}; ${Math.round(row.totalMs??0)} ms; references ${row.provenanceValid?'pass':'fail'}; finish ${row.finish??'error'}`);
  }
  manifest.incomplete=false;await writeFile(join(output,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
 }finally{await host?.close();for(const index of Object.values(indexes))await index.close();}
}
