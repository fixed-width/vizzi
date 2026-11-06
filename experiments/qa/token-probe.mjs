import assert from 'node:assert/strict';
import {mkdir,readFile,writeFile,appendFile} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {gunzipSync} from 'node:zlib';
import {createHash} from 'node:crypto';
import {cpus,totalmem} from 'node:os';
import {json,digest} from './support.mjs';
import {captureHashes,verifyHashes} from './compact-support.mjs';
import {chooseModel} from './model-config.mjs';
import {server} from './model.mjs';
import {probePlan,probeMessages,probeRequest,readProbe,probeEffects} from './token-probe-support.mjs';

const [command,...args]=process.argv.slice(2),allowed=new Set(['--output','--prepared','--runtime','--models-root']);assert.ok(['prepare','run'].includes(command));
for(let i=0;i<args.length;i+=2)assert.ok(allowed.has(args[i]) && args[i+1] && !args[i+1].startsWith('--'));
const option=name=>{const i=args.indexOf(name);assert.ok(i>=0,`missing ${name}`);return resolve(args[i+1]);};
const output=option('--output'),specPath=resolve('experiments/qa/library/token-probe/cohort.json'),spec=await json(specPath),plan=probePlan(spec),models=await json(spec.models),model=chooseModel(models,spec.model);
const save=(path,value)=>writeFile(path,JSON.stringify(value,null,2)+'\n',{flag:'wx'}),hash=value=>createHash('sha256').update(value).digest('hex');
const limits={...models.limits,threads:2,context_tokens:8192,batch_tokens:512,ubatch_tokens:128,request_timeout_ms:spec.request_timeout_ms};
if(command==='prepare'){
 const archive=await json(spec.archiveChecksums);assert.equal(await digest(spec.inputArchive),archive.sha256['inputs.json.gz']);
 const source=JSON.parse(gunzipSync(await readFile(spec.inputArchive))),inputs=spec.cases.map(c=>{const input=source.find(i=>i.case===c.id);assert.ok(input);return input;});
 await mkdir(output);await save(join(output,'inputs.json'),inputs);
 const files=await captureHashes([specPath,spec.models,spec.inputArchive,spec.archiveChecksums,'docs/qa/TOKEN_PROBE.md','experiments/qa/token-probe.mjs','experiments/qa/token-probe-support.mjs','experiments/qa/model.mjs','experiments/qa/runtime-profile.mjs','experiments/qa/support.mjs','experiments/qa/compact-support.mjs','target/release/vizzi-envelope',join(output,'inputs.json')]);
 await save(join(output,'prepared.json'),{schema:1,createdAt:new Date().toISOString(),files,model,limits,expectedObservations:plan.length,reservedGenerations:0,inputArchiveSha256:await digest(spec.inputArchive)});
 console.log('Froze four historical captured pin contexts and 32 conditional one-token observations.');
}else{
 const preparedPath=option('--prepared'),prepared=await json(preparedPath);await verifyHashes(prepared.files);assert.deepEqual(prepared.model,model);assert.deepEqual(prepared.limits,limits);assert.equal(prepared.expectedObservations,plan.length);assert.equal(prepared.reservedGenerations,0);
 const inputs=await json(join(resolve(preparedPath,'..'),'inputs.json')),runtimePath=option('--runtime'),weights=join(option('--models-root'),model.directory,model.filename);
 assert.equal(await digest(runtimePath),spec.runtime_binary_sha256);assert.equal(await digest(weights),model.sha256);
 await mkdir(output);let host;
 const manifest={schema:1,createdAt:new Date().toISOString(),preparedSha256:await digest(preparedPath),model,runtimeSha256:await digest(runtimePath),limits,expectedObservations:plan.length,reservedGenerations:0,environment:{cpu:cpus()[0].model,physicalMemoryBytes:totalmem()},completed:0,incomplete:true};await save(join(output,'manifest.json'),manifest);
 try{
  host=await server(runtimePath,weights,limits,join(output,'runtime.log'));await save(join(output,'runtime.json'),{pid:host.pid,args:host.args,startupMs:host.startupMs});
  const post=async(path,body)=>{const response=await fetch(host.url+path,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body),signal:AbortSignal.timeout(limits.request_timeout_ms)});assert.ok(response.ok,`${path}: ${response.status}`);return response.json();};
  const candidates={};
  for(const prefix of spec.prefixes){
   candidates[prefix.id]={};
   for(const [label,spelling]of Object.entries(prefix.choices)){
    const result=await post('/tokenize',{content:spelling,add_special:false,parse_special:false,with_pieces:true});assert.equal(result.tokens.length,1,'candidate spelling is not a single token');
    const token=result.tokens[0];assert.ok(Number.isInteger(token.id) && token.id>=0);assert.equal((await post('/detokenize',{tokens:[token.id]})).content,spelling,'candidate does not round trip');candidates[prefix.id][label]=token.id;
   }
   assert.equal(new Set(Object.values(candidates[prefix.id])).size,3);
  }
  const conditions={};
  for(const p of plan.filter(p=>p.repeat===1)){
   const input=inputs.find(i=>i.case===p.case.id),messages=probeMessages(input.question,input.context,p.system),base=probeMessages(input.question,input.context,spec.systems[0]);assert.deepEqual(messages[1],base[1]);
   const rendered=await post('/apply-template',{messages,chat_template_kwargs:{enable_thinking:false}});assert.ok(typeof rendered.prompt==='string' && rendered.prompt.length);
   const text=rendered.prompt+p.prefix.text,tokenized=await post('/tokenize',{content:text,add_special:false,parse_special:true,with_pieces:false});
   const request=probeRequest(tokenized.tokens,spec.topCount);conditions[p.case.id+'/'+p.system.id+'/'+p.prefix.id]={question:input.question,context:input.context,messages,renderedPrompt:rendered.prompt,fullPrompt:text,tokenIds:tokenized.tokens,promptSha256:hash(text),request};
  }
  await save(join(output,'preflight.json'),{candidates,conditions});
  const rows=[];
  for(const p of plan){
   const input=conditions[p.case.id+'/'+p.system.id+'/'+p.prefix.id],started=performance.now(),response=await post('/completion',input.request);
   await appendFile(join(output,'responses.jsonl'),JSON.stringify({id:p.id,request:input.request,response})+'\n');
   const measurement=readProbe(response,candidates[p.prefix.id],input.tokenIds.length,spec.topCount);
   const row={id:p.id,case:p.case.id,system:p.system.id,prefix:p.prefix.id,repeat:p.repeat,expected:p.case.expected,promptSha256:input.promptSha256,promptTokens:input.tokenIds.length,request:input.request,response,measurement,totalMs:performance.now()-started};
   rows.push(row);await appendFile(join(output,'probes.jsonl'),JSON.stringify(row)+'\n');manifest.completed++;await writeFile(join(output,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
   console.log(`${manifest.completed}/${plan.length} ${p.id}: Yes-No margin ${measurement.marginYesNo.value??'censored'}; choice winner ${measurement.winner??'unresolved'}; global top ${measurement.top.id}`);
  }
  await save(join(output,'effects.json'),probeEffects(rows,spec));manifest.incomplete=false;await writeFile(join(output,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
 }catch(error){await save(join(output,'failure.json'),{completed:manifest.completed,error:error.message});throw error;}finally{await host?.close();}
}
