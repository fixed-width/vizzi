// First explicit-operation observations and an equal-seed ordinary text-search control.
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {join,resolve} from 'node:path';
import {startHost} from '../browser/support.mjs';
const [preparedPath,planPath,outputPath]=process.argv.slice(2);
assert.ok(preparedPath&&planPath&&outputPath,'Use capture.mjs PREPARED PLAN NEW_OUTPUT');
const prepared=JSON.parse(await readFile(preparedPath,'utf8')),plan=JSON.parse(await readFile(planPath,'utf8'));
const hash=b=>createHash('sha256').update(b).digest('hex');assert.equal(hash(await readFile('target/release/vizzi')),plan.candidateBinarySha256);
assert.equal(hash(await readFile('experiments/companion/library/sketchlayer.json')),plan.cohortSha256);
for(const [path,record]of Object.entries(prepared.files))assert.equal(hash(await readFile(join(prepared.sourceRoot,path))),record.sha256);
const output=resolve(outputPath);await mkdir(output);let host;const observations=[];
try{
 host=await startHost({root:prepared.sourceRoot,cache:join(output,'analysis')},1);
 const request=async(path,body)=>{const start=performance.now(),response=await fetch(host.url+path,body===undefined?{}:{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)}),text=await response.text(),elapsedMs=performance.now()-start;assert.ok(response.ok,text);return{data:JSON.parse(text),elapsedMs,responseBytes:Buffer.byteLength(text)};};
 await request('/api/workspace/start',{});let status;
 for(let i=0;;i++){status=(await request('/api/status')).data;if(status.snapshot)break;assert.ok(i<600&&!status.error);await new Promise(done=>setTimeout(done,100));}
 for(const c of plan.cases)for(const seed of c.seeds){
  const record={case:c.id,seed,operations:[]};
  for(const query of [{kind:'areas',search:seed,offset:0},{kind:'inventory',area:'',search:seed,offset:0}]){
   const result=await request('/api/structure',{snapshot:status.snapshot,query});assert.equal(result.data.snapshot,status.snapshot);assert.ok(result.data.items.length<=24);record.operations.push({query,...result});
  }
  const args=['-n','-i','-F','--glob','*.rs','--glob','*.py','--',seed,'.'];const start=performance.now();const text=spawnSync('rg',args,{cwd:prepared.sourceRoot,encoding:'utf8',maxBuffer:8*1024*1024});assert.ok(text.status===0||text.status===1,text.stderr);record.textSearch={command:['rg',...args],exitCode:text.status,stdout:text.stdout,stderr:text.stderr,elapsedMs:performance.now()-start};
  observations.push(record);await writeFile(join(output,`${c.id}.${seed}.json`),JSON.stringify(record,null,2)+'\n');
 }
 await writeFile(join(output,'first-operations.json'),JSON.stringify({schema:1,sourceRevision:prepared.sourceRevision,sourceRoot:prepared.sourceRoot,snapshot:status.snapshot,planSha256:hash(await readFile(planPath)),binarySha256:plan.candidateBinarySha256,model:null,workers:1,observations,interpretation:'Predeclared literal seeds; explicit operation choice, not a model/NL router. First metadata/text outputs before source audit; timings are single observations, not task times or distributions.'},null,2)+'\n');
 console.log(JSON.stringify(observations.map(o=>({case:o.case,seed:o.seed,areas:o.operations[0].data.matching,declarations:o.operations[1].data.matching,textLines:o.textSearch.stdout.trim()?o.textSearch.stdout.trim().split('\n').length:0})),null,2));
}finally{await host?.close();}
