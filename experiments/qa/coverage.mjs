import assert from 'node:assert/strict';
import {mkdir,writeFile,readFile,appendFile,readdir} from 'node:fs/promises';
import {join,resolve,dirname} from 'node:path';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {corpus,json,digest} from './support.mjs';
import {captureHashes,verifyHashes} from './compact-support.mjs';
import {coveragePlan,locateTarget,assessCoverage} from './coverage-support.mjs';

const [command,...args]=process.argv.slice(2),allowed=new Set(['--output','--prepared','--source-archive']);
assert.ok(['prepare','run'].includes(command));for(let i=0;i<args.length;i+=2)assert.ok(allowed.has(args[i])&&args[i+1]&&!args[i+1].startsWith('--'));
const option=name=>{const i=args.indexOf(name);assert.ok(i>=0,`missing ${name}`);return resolve(args[i+1]);};
const output=option('--output'),specPath='experiments/qa/library/search-coverage/cohort.json',spec=await json(specPath),save=(path,value)=>writeFile(path,JSON.stringify(value,null,2)+'\n',{flag:'wx'});
assert.equal(spec.schema,1);assert.equal(spec.split,'known_development');assert.equal(spec.reservedGenerations,0);assert.equal(spec.repeats,3);assert.deepEqual(spec.selectors,['compact','search']);
const plan=coveragePlan(spec.cases);
if(command==='prepare'){
 const source=await json(spec.sourceRecord),archive=args.includes('--source-archive')?option('--source-archive'):source.sourceRoot+'.tar',root=join(output,'source');
 assert.equal(await digest(archive),source.archiveSha256);
 await mkdir(output);await mkdir(root);
 const child=spawn('python3',['-c',"import sys,tarfile; tarfile.open(sys.argv[1]).extractall(sys.argv[2],filter='data')",archive,root],{stdio:['ignore','ignore','inherit']});const [code]=await once(child,'exit');assert.equal(code,0);
 const audit=[];
 for(const c of spec.cases)for(const target of c.targets){
  locateTarget({evidence:[]},target);
  for(const alternative of target.alternatives){const text=await readFile(join(root,alternative.path),'utf8');assert.ok(text.includes(alternative.needle),`missing source oracle: ${c.id}/${target.id}`);}
  audit.push({case:c.id,target:target.id,sourceMarkerPresent:true});
 }
 await save(join(output,'source-audit.json'),audit);
 const index=await corpus(root);
 try{
  assert.equal(index.ready.stats.files,source.initial.stats.files);assert.equal(index.ready.stats.bytes,source.initial.stats.bytes);
  const rust=[];for(const crate of ['vizzi-core','vizzi-qa'])for(const file of await readdir('crates/'+crate+'/src'))if(file.endsWith('.rs'))rust.push(join('crates',crate,'src',file));
  const files=await captureHashes([specPath,spec.sourceRecord,spec.protocol,archive,...rust,'Cargo.toml','Cargo.lock','experiments/qa/coverage.mjs','experiments/qa/coverage-support.mjs','experiments/qa/support.mjs','experiments/qa/compact-support.mjs','target/release/vizzi-qa','target/release/vizzi-envelope',join(output,'source-audit.json')]);
  await save(join(output,'prepared.json'),{schema:1,createdAt:new Date().toISOString(),files,sourceRoot:root,sourceRevision:source.sourceRevision,sourceArchiveSha256:source.archiveSha256,previousIndexSnapshot:source.initial.snapshot,previousRankingIndexSnapshot:(await json('experiments/qa/results/search/prepared.json')).expectedSnapshot,expectedSnapshot:index.ready.snapshot,capture:index.ready,expectedObservations:plan.length,reservedGenerations:0});
  console.log(`Froze ${spec.cases.length} known questions, ${audit.length} audited source targets and ${plan.length} paired observations; no queries run.`);
 }finally{await index.close();}
}else{
 const preparedPath=option('--prepared'),prepared=await json(preparedPath);await verifyHashes(prepared.files);assert.equal(prepared.expectedObservations,plan.length);assert.equal(prepared.reservedGenerations,0);
 await mkdir(output);await save(join(output,'freeze.json'),{schema:1,createdAt:new Date().toISOString(),files:{...prepared.files,[preparedPath]:await digest(preparedPath)},reservedGenerations:0});
 const index=await corpus(prepared.sourceRoot),rows=[];
 try{
  assert.equal(index.ready.snapshot,prepared.expectedSnapshot);await save(join(output,'native.json'),{pid:index.pid,ready:index.ready});
  for(const p of plan){
   const started=performance.now(),response=await index.request({op:'retrieve',id:p.id,question:p.question,selector:p.selector,limit:spec.limit,budget:spec.budget}),roundTripMs=performance.now()-started;
   await appendFile(join(output,'responses.jsonl'),JSON.stringify({...p,response,roundTripMs})+'\n');assert.ok(!response.error,response.error);
   const context=response.context;assert.equal(context.snapshot,prepared.expectedSnapshot);assert.ok(context.evidence.length<=spec.limit);assert.ok(context.evidence.reduce((n,e)=>n+Buffer.byteLength(e.text),0)<=spec.budget);
   const checked=await index.request({op:'reorder',id:p.id,order:context.evidence.map(e=>e.label)});assert.ok(!checked.error,checked.error);assert.deepEqual(checked.context,context);
   const row={...p,context,elapsedMs:response.elapsed_ms,roundTripMs};rows.push(row);await appendFile(join(output,'retrieval.jsonl'),JSON.stringify(row)+'\n');
   console.log(`${rows.length}/${plan.length} ${p.id}: ${context.evidence.length} sources, ${response.elapsed_ms.toFixed(1)} ms`);
  }
  await save(join(output,'assessment.json'),assessCoverage(rows,spec.cases,prepared.expectedSnapshot));
  await save(join(output,'manifest.json'),{schema:1,preparedSha256:await digest(preparedPath),sourceRevision:prepared.sourceRevision,snapshot:prepared.expectedSnapshot,observations:rows.length,modelRequests:0,reservedGenerations:0,complete:true});
 }catch(error){await save(join(output,'failure.json'),{completed:rows.length,error:error.message});throw error;}
 finally{await index.close();}
}
