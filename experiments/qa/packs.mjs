import assert from 'node:assert/strict';
import {mkdir,readFile,writeFile,appendFile,stat,readdir} from 'node:fs/promises';
import {spawnSync} from 'node:child_process';
import {join,resolve} from 'node:path';
import {cpus,platform,arch,totalmem} from 'node:os';
import {json,digest,corpus,coverage,modelPath} from './support.mjs';
import {captureHashes,verifyHashes,evidenceHash,cohortPath} from './compact-support.mjs';
import {checkedRange,checkAudit} from './packs-support.mjs';
import {validateMaintainer} from './maintainer-cases.mjs';
import {server,generate,schemaFor} from './model.mjs';

const [command,...args]=process.argv.slice(2);
function option(name){const i=args.indexOf(name);assert.ok(i>=0 && args[i+1] && !args[i+1].startsWith('--'),`missing ${name}`);return resolve(args[i+1]);}
const output=option('--output'),specPath=resolve('experiments/qa/library/source-packs/packs.json');
const questionPath=resolve('experiments/qa/library/maintainer-2026-11-05/questions.json'),libraryPath=resolve('experiments/qa/library/maintainer-2026-11-05/cases.json');
const library=await json(libraryPath),cases=validateMaintainer(await json(questionPath),library),spec=await json(specPath),modelConfig=await json(modelPath),baseCohort=await json(cohortPath);
assert.equal(spec.sourceRevision,library.sources['lpc-rs'].revision);
assert.equal(new Set(spec.cases.map(c=>c.id)).size,cases.length);
assert.ok(cases.every(c=>spec.cases.some(p=>p.id===c.id)));
const arms=[{id:'retrieved_ids',selection:'compact',wordLimit:100,outputTokens:384},{id:'curated_ids',selection:'pack',wordLimit:100,outputTokens:384},{id:'curated_long',selection:'pack',wordLimit:300,outputTokens:1024}];
const limits={...modelConfig.limits,request_timeout_ms:360000},save=(path,data)=>writeFile(path,JSON.stringify(data,null,2)+'\n',{flag:'wx'});
async function selected(index,sourceRoot,item,id){
 const pack=spec.cases.find(p=>p.id===item.id),expected=[];
 const ranges=[];
 for(const selection of pack.ranges){
  assert.ok(!selection.path.startsWith('/') && !selection.path.split('/').some(p=>p==='..' || p===''),'invalid source path');
  const bytes=await readFile(join(sourceRoot,selection.path)),range=checkedRange(bytes,selection);
  expected.push(bytes.subarray(range.start,range.end).toString('utf8'));ranges.push(range);
 }
 const response=await index.request({op:'select',id,snapshot:index.ready.snapshot,ranges,budget:spec.selectionBudget});
 assert.ok(!response.error,`${id}: ${response.error}`);
 assert.deepEqual(response.context.evidence.map(e=>e.text),expected,'captured ranges differ from pinned bytes');
 assert.equal(response.context.evidence.reduce((n,e)=>n+Buffer.byteLength(e.text),0),pack.evidenceBytes);
 return response;
}

if(command==='prepare'){
 await mkdir(output);const archive=join(output,'lpc-rs.tar'),sourceRoot=join(output,'lpc-rs');
 const exported=spawnSync('git',['-C',option('--repo'),'archive','--format=tar','--output',archive,spec.sourceRevision],{encoding:'utf8',env:{...process.env,GIT_OPTIONAL_LOCKS:'0'}});assert.equal(exported.status,0,exported.stderr);
 const original=await json('experiments/qa/results/compact/prepared.json');assert.equal(await digest(archive),original.sources['lpc-rs'].archiveSha256);
 await mkdir(sourceRoot);const extracted=spawnSync('python3',['-c',"import sys,tarfile; tarfile.open(sys.argv[1]).extractall(sys.argv[2],filter='data')",archive,sourceRoot],{encoding:'utf8'});assert.equal(extracted.status,0,extracted.stderr);
 const index=await corpus(sourceRoot),audit=[];
 try{
  for(const item of cases){const response=await selected(index,sourceRoot,item,'audit/'+item.id);audit.push({id:item.id,...checkAudit(item,response.context),context:response.context,manualAudit:spec.cases.find(p=>p.id===item.id).audit});}
  const rust=(await readdir('crates/vizzi-qa/src')).filter(p=>p.endsWith('.rs')).map(p=>'crates/vizzi-qa/src/'+p);
  const files=await captureHashes([...rust,'Cargo.lock','crates/vizzi-qa/Cargo.toml',specPath,questionPath,libraryPath,modelPath,cohortPath,'experiments/qa/packs.mjs','experiments/qa/packs-support.mjs','experiments/qa/model.mjs','experiments/qa/support.mjs','experiments/qa/compact-support.mjs','experiments/qa/maintainer-cases.mjs','target/release/vizzi-qa','target/release/vizzi-envelope']);
  await save(join(output,'prepared.json'),{schema:1,createdAt:new Date().toISOString(),sourceRoot,sourceRevision:spec.sourceRevision,archiveSha256:await digest(archive),initial:index.ready,files,audit,arms,limits,citationLimit:8,schema:schemaFor('ids',8),limitations:['Known maintainer questions; source-complete diagnostic, not new held-out accuracy.','Curated evidence changes relevance, completeness and size together.','The uniform eight-ID schema and request timeout differ from earlier cohorts; defaults of older runners are unchanged.','Long response changes word and token allowances together; schema answer length stays 2,000 characters.']});
 }finally{await index.close();}
 console.log('Audited all source packs and froze native/adapter/data inputs before inference.');
}else if(command==='run'){
 const preparedPath=option('--prepared'),prepared=await json(preparedPath);await verifyHashes(prepared.files);assert.deepEqual(prepared.arms,arms);assert.deepEqual(prepared.limits,limits);
 const model=modelConfig.models.find(m=>m.id===baseCohort.model),runtime=option('--runtime'),weights=join(option('--models-root'),model.directory,model.filename);
 assert.equal((await stat(weights)).size,model.bytes);assert.equal(await digest(weights),model.sha256);assert.equal(await digest(runtime),baseCohort.runtime_binary_sha256);
 await mkdir(output);
 const manifest={schema:1,createdAt:new Date().toISOString(),preparedSha256:await digest(preparedPath),qaBinarySha256:await digest('target/release/vizzi-qa'),model,runtime:modelConfig.runtime,runtimeSha256:await digest(runtime),limits,arms,citationLimit:8,environment:{cpu:cpus()[0].model,os:platform(),arch:arch(),physicalMemoryBytes:totalmem()},sourceRevision:prepared.sourceRevision,initial:prepared.initial,limitations:prepared.limitations,completed:0,incomplete:true};await save(join(output,'manifest.json'),manifest);
 const index=await corpus(prepared.sourceRoot);let host;
 try{
  assert.equal(index.ready.snapshot,prepared.initial.snapshot);
  host=await server(runtime,weights,limits,join(output,'runtime.log'));await save(join(output,'runtime.json'),{pid:host.pid,args:host.args,startupMs:host.startupMs});
  for(const [number,item]of cases.entries()){
   const contexts=new Map();
   for(const arm of arms){const id=`${item.id}/${arm.id}`,response=arm.selection==='pack'?await selected(index,prepared.sourceRoot,item,id):await index.request({op:'retrieve',id,question:item.question,selector:'compact',limit:3,budget:2500});assert.ok(!response.error,response.error);contexts.set(arm.id,response);}
   assert.equal(evidenceHash(contexts.get('curated_ids').context),evidenceHash(contexts.get('curated_long').context));
   const order=[...arms.slice(number%3),...arms.slice(0,number%3)];
   for(const arm of order){
    const id=`${item.id}/${arm.id}`,response=contexts.get(arm.id),context=response.context;
    const record={id,case:item.id,repository:item.repository,split:'diagnostic',model:model.id,arm:arm.id,protocol:'ids',order:order.map(a=>a.id),contextSha256:evidenceHash(context),context,retrievalMs:response.elapsed_ms,evidenceBytes:context.evidence.reduce((n,e)=>n+Buffer.byteLength(e.text),0),anchorCoverage:coverage(item,context),wordLimit:arm.wordLimit,outputTokenLimit:arm.outputTokens};
    try{
     Object.assign(record,await generate(host,item,context,{...limits,output_tokens:arm.outputTokens},{protocol:'ids',metadata:true,wordLimit:arm.wordLimit,citationLimit:8}));
     if(record.answer){const validation=await index.request({op:'validate_ids',id,answer:record.answer});record.provenanceValid=validation.provenance_valid===true;record.provenanceError=validation.error;record.references=validation.references;}else record.provenanceValid=false;
    }catch(e){record.runtimeError=e.message;record.provenanceValid=false;}
    await appendFile(join(output,'answers.jsonl'),JSON.stringify(record)+'\n');manifest.completed++;await writeFile(join(output,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
    console.log(`${manifest.completed}/9 ${id}: ${record.answer?.status??'error'}; ${Math.round(record.totalMs??0)} ms; source checks ${record.provenanceValid?'pass':'fail'}; finish ${record.finish??'error'}`);
   }
  }
  manifest.incomplete=false;await writeFile(join(output,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
 }finally{await host?.close();await index.close();}
}else throw new Error('command must be prepare or run');
