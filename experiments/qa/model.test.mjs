import assert from 'node:assert/strict';
import {prompt,schema,generate,server} from './model.mjs';
import {createServer} from 'node:http';
import {mkdtemp,writeFile,chmod,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
const hidden='HIDDEN_GOLD_SENTINEL';
const item={repository:'fixture',question:'Where is the entry?',required_facts:[{text:hidden}],forbidden_claims:[hidden]};
const context={snapshot:'snapshot',evidence:[{label:1,path:'src/a.rs',start_line:1,end_line:1,text:'fn main() {}'}]};
const encoded=JSON.stringify(prompt(item,context));assert.ok(!encoded.includes(hidden));assert.ok(encoded.includes('fn main() {}'));assert.ok(encoded.includes(item.question));assert.deepEqual(schema.properties.status.enum,['answered','insufficient_evidence']);
console.log('model prompt: only question and supplied source cross the inference boundary; gold rubric is excluded');
const observed=[];
const stub=createServer((request,response)=>{
 let body='';request.on('data',b=>body+=b);
 request.on('end',()=>{
  const data=JSON.parse(body);observed.push(data);const answerFirst=data.response_format?.schema?.required[0]==='answer';
  response.setHeader('content-type','text/event-stream');response.flushHeaders();
  if(!data.response_format){
   response.write('data: '+JSON.stringify({choices:[{delta:{content:'The definition '}}]})+'\n\n');
   setTimeout(()=>{response.write('data: '+JSON.stringify({choices:[{delta:{content:'is missing.'},finish_reason:'stop'}],usage:{prompt_tokens:10,completion_tokens:4}})+'\n\n');response.end('data: [DONE]\n\n');},200);return;
  }
  response.write('data: '+JSON.stringify({choices:[{delta:{content:answerFirst?'{"answer":"Not established",':'{"status":"insufficient_evidence",'}}]})+'\n\n');
  setTimeout(()=>{response.write('data: '+JSON.stringify({choices:[{delta:{content:answerFirst?'"status":"insufficient_evidence","citations":[]}':'"answer":"Not established","citations":[]}'},finish_reason:'stop'}],usage:{prompt_tokens:10,completion_tokens:10}})+'\n\n');response.end('data: [DONE]\n\n');},200);
 });
});
await new Promise((resolve,reject)=>{stub.once('error',reject);stub.listen(0,'127.0.0.1',resolve);});
try{
 const result=await generate({url:`http://127.0.0.1:${stub.address().port}`},item,context,{request_timeout_ms:3000,output_tokens:384});
 assert.equal(result.answer.status,'insufficient_evidence');assert.ok(result.firstAnswerMs<result.totalMs-100,'first-token timing must not wait for the whole streamed body');
 const ids=await generate({url:`http://127.0.0.1:${stub.address().port}`},item,context,{request_timeout_ms:3000,output_tokens:384},{protocol:'ids',metadata:true});
 assert.equal(ids.answer.status,'insufficient_evidence');assert.deepEqual(ids.answer.citations,[]);
 const longer=await generate({url:`http://127.0.0.1:${stub.address().port}`},item,context,{request_timeout_ms:3000,output_tokens:1024},{protocol:'ids',metadata:true,wordLimit:300,citationLimit:8});
 assert.equal(longer.answer.status,'insufficient_evidence');
 const reordered=await generate({url:`http://127.0.0.1:${stub.address().port}`},item,context,{request_timeout_ms:3000,output_tokens:1024},{protocol:'ids',metadata:true,wordLimit:300,citationLimit:8,fieldOrder:'answer_first'});
 assert.deepEqual(Object.keys(reordered.answer),['answer','status','citations']);
 const raw=await generate({url:`http://127.0.0.1:${stub.address().port}`},item,context,{request_timeout_ms:3000,output_tokens:1024},{protocol:'ids',metadata:true,wordLimit:300,citationLimit:8,fieldOrder:'answer_first',constraint:'none'});
 assert.equal(raw.raw,'The definition is missing.');assert.equal(raw.answer,undefined);assert.ok(raw.formatError);assert.equal(raw.finish,'stop');
 const {response_format,...expected}=observed[3];assert.deepEqual(observed[4],expected,'actual unconstrained HTTP request only omits response_format');
 const object=await generate({url:`http://127.0.0.1:${stub.address().port}`},item,context,{request_timeout_ms:3000,output_tokens:1024},{protocol:'ids',metadata:true,wordLimit:300,citationLimit:8,fieldOrder:'answer_first',constraint:'json_object'});
 assert.equal(object.answer.status,'insufficient_evidence');assert.deepEqual(observed[5],{...observed[3],response_format:{type:'json_object'}},'actual generic HTTP request only omits the schema property');
 await assert.rejects(generate({url:`http://127.0.0.1:${stub.address().port}`},item,context,{request_timeout_ms:3000,output_tokens:1024},{constraint:'invalid'}),/invalid output constraint/);assert.equal(observed.length,6);
 for(const [i,data]of observed.entries()){assert.equal(data.temperature,0);assert.equal(data.seed,42);if(i<4)assert.deepEqual(Object.keys(data.response_format.schema.properties),i===3?['answer','status','citations']:['status','answer','citations']);assert.ok(!JSON.stringify(data.messages).includes(hidden));}
 assert.deepEqual(observed.map(d=>d.max_tokens),[384,384,1024,1024,1024,1024]);
 assert.deepEqual(observed.slice(0,4).map(d=>d.response_format.schema.properties.citations.maxItems),[4,4,8,8]);
 assert.equal(observed[2].messages[0].content,observed[1].messages[0].content.replace('100 words','300 words'));
 assert.deepEqual(observed[2].messages[1],observed[1].messages[1]);
 assert.deepEqual(observed[3].messages,observed[2].messages,'field order must not alter prompt text');
 console.log('model streaming: first answer token is observed before complete delivery');
}finally{await new Promise(resolve=>stub.close(resolve));}
const directory=await mkdtemp(join(tmpdir(),'vizzi-qa-runtime-profile-'));let child;
try{
 const runtime=join(directory,'runtime.cjs'),log=join(directory,'runtime.log');
 await writeFile(runtime,"#!/usr/bin/env node\nconst http=require('node:http');const args=process.argv.slice(2);console.log(JSON.stringify(args));const at=n=>args[args.indexOf(n)+1];http.createServer((q,r)=>{r.writeHead(200,{'Content-Type':'application/json'});r.end('{}');}).listen(Number(at('--port')),'127.0.0.1');\n");
 await chmod(runtime,0o755);
 child=await server(runtime,'unused-model.gguf',{threads:2,context_tokens:8192,batch_tokens:512,ubatch_tokens:32},log);
 const actual=JSON.parse((await readFile(log,'utf8')).trim());assert.deepEqual(actual,child.args);
 const at=n=>actual[actual.indexOf(n)+1];assert.equal(at('--batch-size'),'512');assert.equal(at('--ubatch-size'),'32');assert.equal(at('--threads'),'2');assert.equal(at('--threads-batch'),'2');assert.equal(at('--ctx-size'),'8192');assert.equal(at('--device'),'none');assert.equal(at('--n-gpu-layers'),'0');assert.ok(actual.includes('--no-op-offload') && actual.includes('--offline') && actual.includes('--no-mmproj'));
 console.log('runtime profile: spawned CPU-only server receives the selected logical/physical batch limits');
}finally{await child?.close();await rm(directory,{recursive:true});}
