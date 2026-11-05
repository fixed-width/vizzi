import assert from 'node:assert/strict';
import {prompt,schema,generate} from './model.mjs';
import {createServer} from 'node:http';
const hidden='HIDDEN_GOLD_SENTINEL';
const item={repository:'fixture',question:'Where is the entry?',required_facts:[{text:hidden}],forbidden_claims:[hidden]};
const context={snapshot:'snapshot',evidence:[{label:1,path:'src/a.rs',start_line:1,end_line:1,text:'fn main() {}'}]};
const encoded=JSON.stringify(prompt(item,context));assert.ok(!encoded.includes(hidden));assert.ok(encoded.includes('fn main() {}'));assert.ok(encoded.includes(item.question));assert.deepEqual(schema.properties.status.enum,['answered','insufficient_evidence']);
console.log('model prompt: only question and supplied source cross the inference boundary; gold rubric is excluded');
const stub=createServer((request,response)=>{
 request.resume();response.setHeader('content-type','text/event-stream');response.flushHeaders();
 response.write('data: '+JSON.stringify({choices:[{delta:{content:'{"status":"insufficient_evidence",'}}]})+'\n\n');
 setTimeout(()=>{response.write('data: '+JSON.stringify({choices:[{delta:{content:'"answer":"Not established","citations":[]}'},finish_reason:'stop'}],usage:{prompt_tokens:10,completion_tokens:10}})+'\n\n');response.end('data: [DONE]\n\n');},200);
});
await new Promise((resolve,reject)=>{stub.once('error',reject);stub.listen(0,'127.0.0.1',resolve);});
try{
 const result=await generate({url:`http://127.0.0.1:${stub.address().port}`},item,context,{request_timeout_ms:3000,output_tokens:384});
 assert.equal(result.answer.status,'insufficient_evidence');assert.ok(result.firstAnswerMs<result.totalMs-100,'first-token timing must not wait for the whole streamed body');
 console.log('model streaming: first answer token is observed before complete delivery');
}finally{await new Promise(resolve=>stub.close(resolve));}
