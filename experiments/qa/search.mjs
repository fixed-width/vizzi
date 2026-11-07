import assert from 'node:assert/strict';
import {mkdir,writeFile,appendFile,readdir} from 'node:fs/promises';
import {join,resolve,dirname} from 'node:path';
import {cpus,totalmem} from 'node:os';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {corpus,json,digest} from './support.mjs';
import {captureHashes,verifyHashes} from './compact-support.mjs';
import {chooseModel} from './model-config.mjs';
import {server} from './model.mjs';
import {rankingRequest,resolveRanking,rankQuality,searchPlan,assessSearch} from './search-support.mjs';

const [command,...args]=process.argv.slice(2),allowed=new Set(['--output','--prepared','--runtime','--models-root','--source-archive']);
assert.ok(['prepare','run'].includes(command));
for(let i=0;i<args.length;i+=2)assert.ok(allowed.has(args[i])&&args[i+1]&&!args[i+1].startsWith('--'));
const option=name=>{const i=args.indexOf(name);assert.ok(i>=0,`missing ${name}`);return resolve(args[i+1]);};
const output=option('--output'),specPath='experiments/qa/library/search/cohort.json',spec=await json(specPath),models=await json(spec.models),model=chooseModel(models,spec.model);
assert.equal(spec.schema,1);assert.equal(spec.split,'development');assert.equal(spec.reservedGenerations,0);
const save=(path,value)=>writeFile(path,JSON.stringify(value,null,2)+'\n',{flag:'wx'});
if(command==='prepare'){
 const library=await json(spec.questions),source=await json(spec.sourceRecord),archive=args.includes('--source-archive')?option('--source-archive'):source.sourceRoot+'.tar',root=join(output,'source');
 assert.equal(library.sources['lpc-rs'].revision,source.sourceRevision);assert.equal(new Set(spec.case_ids).size,3);
 assert.equal(await digest(archive),source.archiveSha256,'source archive differs from the pinned development source');
 await mkdir(output);await mkdir(root);
 const extraction=spawn('python3',['-c',"import sys,tarfile; tarfile.open(sys.argv[1]).extractall(sys.argv[2],filter='data')",archive,root],{stdio:['ignore','ignore','inherit']});
 const [code]=await once(extraction,'exit');assert.equal(code,0,'source extraction failed');
 const index=await corpus(root),inputs=[];
 try{
  for(const id of spec.case_ids){
   const item=library.cases.find(c=>c.id===id);assert.ok(item&&item.repository==='lpc-rs');
   const retrieved=await index.request({op:'retrieve',id,question:item.question,selector:spec.selector,limit:spec.limit,budget:spec.budget});assert.ok(!retrieved.error,retrieved.error);
   inputs.push({case:id,repository:item.repository,question:item.question,context:retrieved.context,retrievalMs:retrieved.elapsed_ms});
  }
  await save(join(output,'inputs.json'),inputs);
  const rust=(await readdir('crates/vizzi-qa/src')).filter(p=>p.endsWith('.rs')).map(p=>join('crates/vizzi-qa/src',p));
  const files=await captureHashes([specPath,spec.models,spec.questions,spec.sourceRecord,archive,'docs/qa/SEARCH.md','experiments/qa/search.mjs','experiments/qa/search-support.mjs','experiments/qa/model.mjs','experiments/qa/model-config.mjs','experiments/qa/runtime-profile.mjs','experiments/qa/support.mjs','experiments/qa/compact-support.mjs','crates/vizzi-qa/Cargo.toml','Cargo.lock',...rust,'target/release/vizzi-qa','target/release/vizzi-envelope',join(output,'inputs.json')]);
  await save(join(output,'prepared.json'),{schema:1,createdAt:new Date().toISOString(),files,sourceRoot:root,sourceRevision:source.sourceRevision,archiveSha256:source.archiveSha256,previousIndexSnapshot:source.initial.snapshot,expectedSnapshot:index.ready.snapshot,capture:index.ready,model,limits:spec.limits,expectedObservations:searchPlan(inputs).length,reservedGenerations:0});
  console.log('Captured baseline source results for three known questions. Audit relevance.json before inference.');
 }finally{await index.close();}
}else{
 const preparedPath=option('--prepared'),prepared=await json(preparedPath),base=dirname(preparedPath);
 await verifyHashes(prepared.files);assert.deepEqual(prepared.model,model);assert.deepEqual(prepared.limits,spec.limits);
 const inputs=await json(join(base,'inputs.json')),ratings=await json(join(base,'relevance.json')),relevance={};
 assert.deepEqual(Object.keys(ratings.cases).sort(),inputs.map(i=>i.case).sort());
 for(const input of inputs){const rating=ratings.cases[input.case];relevance[input.case]=rating.grades;rankQuality(input.context.evidence.map(e=>e.label),rating.grades);assert.deepEqual(Object.keys(rating.reasons).sort(),Object.keys(rating.grades).sort());assert.ok(Object.values(rating.reasons).every(s=>typeof s==='string'&&s.trim().length>0));}
 const runtimePath=option('--runtime'),weights=join(option('--models-root'),model.directory,model.filename);
 assert.equal(await digest(runtimePath),spec.runtimeSha256);assert.equal(await digest(weights),model.sha256);
 await mkdir(output);
 await save(join(output,'freeze.json'),{schema:1,createdAt:new Date().toISOString(),files:{...prepared.files,[preparedPath]:await digest(preparedPath),[join(base,'relevance.json')]:await digest(join(base,'relevance.json'))},model,runtimeSha256:await digest(runtimePath),reservedGenerations:0});
 await save(join(output,'relevance.json'),ratings);
 const manifest={schema:1,createdAt:new Date().toISOString(),preparedSha256:await digest(preparedPath),model,limits:spec.limits,sourceRevision:prepared.sourceRevision,expectedSnapshot:prepared.expectedSnapshot,environment:{cpu:cpus()[0].model,physicalMemoryBytes:totalmem()},expectedObservations:searchPlan(inputs).length,reservedGenerations:0,completed:0,modelRequests:0,incomplete:true};
 await save(join(output,'manifest.json'),manifest);
 let index,host;
 try{
  index=await corpus(prepared.sourceRoot);assert.equal(index.ready.snapshot,prepared.expectedSnapshot);await save(join(output,'native.json'),{pid:index.pid,ready:index.ready});
  for(const input of inputs){const actual=await index.request({op:'retrieve',id:input.case,question:input.question,selector:spec.selector,limit:spec.limit,budget:spec.budget});assert.ok(!actual.error,actual.error);assert.deepEqual(actual.context,input.context);await appendFile(join(output,'retrieval.jsonl'),JSON.stringify({case:input.case,...actual})+'\n');}
  if(inputs.some(i=>i.context.evidence.length)){host=await server(runtimePath,weights,spec.limits,join(output,'runtime.log'));await save(join(output,'runtime.json'),{pid:host.pid,args:host.args,startupMs:host.startupMs});}
  const rows=[];
  for(const p of searchPlan(inputs)){
   const input=inputs.find(i=>i.case===p.case),validate=async order=>{const checked=await index.request({op:'reorder',id:input.case,order});assert.ok(!checked.error,checked.error);return checked.context;};
   const started=performance.now();let body=null,response=null,raw=null,transportError=null,modelMs=0,result;
   if(!input.context.evidence.length){result={kind:'baseline',context:await validate([]),failure:null};}
   else{
    body=rankingRequest(input,input.context,p.reverse);manifest.modelRequests++;
    const before=performance.now();
    try{
     const http=await fetch(host.url+'/v1/chat/completions',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body),signal:AbortSignal.timeout(spec.limits.request_timeout_ms)});
     const text=await http.text();assert.ok(http.ok,`${http.status}: ${text}`);response=JSON.parse(text);raw=response.choices?.[0]?.message?.content;
     assert.equal(response.choices?.[0]?.finish_reason,'stop','ranking did not finish normally');
    }catch(error){transportError=error.message;raw=null;}
    modelMs=performance.now()-before;
    await appendFile(join(output,'responses.jsonl'),JSON.stringify({...p,request:body,response,raw,transportError,modelMs})+'\n');
    result=await resolveRanking(input.context,raw,validate);
   }
   const row={...p,request:body,response,raw,transportError,modelMs,result,totalMs:performance.now()-started};rows.push(row);await appendFile(join(output,'rankings.jsonl'),JSON.stringify(row)+'\n');
   manifest.completed++;await writeFile(join(output,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
   console.log(`${manifest.completed}/${manifest.expectedObservations} ${p.id}: ${result.kind}; labels ${result.context.evidence.map(e=>e.label).join(',')}`);
  }
  await save(join(output,'assessment.json'),assessSearch(inputs,rows,relevance));manifest.incomplete=false;await writeFile(join(output,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
 }catch(error){await save(join(output,'failure.json'),{completed:manifest.completed,error:error.message});throw error;}
 finally{try{await host?.close();}finally{await index?.close();}}
}
