import assert from 'node:assert/strict';
import {spawn,spawnSync} from 'node:child_process';
import {createInterface} from 'node:readline';
import {once} from 'node:events';
import {createHash} from 'node:crypto';
import {createReadStream} from 'node:fs';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {join,resolve} from 'node:path';

export const libraryPath=resolve('experiments/qa/library/cases.json');
export const templatesPath=resolve('experiments/qa/library/templates.json');
export const modelPath=resolve('experiments/qa/models.json');
export async function digest(path){const hash=createHash('sha256');for await(const bytes of createReadStream(path))hash.update(bytes);return hash.digest('hex');}
export async function json(path){return JSON.parse(await readFile(path,'utf8'));}
export function validateLibrary(library,templates){
 assert.equal(library.schema,1);assert.equal(templates.schema,1);
 const known=new Set(templates.templates.map(t=>t.id)),ids=new Set();
 assert.equal(known.size,templates.templates.length);
 for(const item of library.cases){
  assert.ok(typeof item.category==='string' && item.category.length>0);
  assert.ok(item.id && !ids.has(item.id),`duplicate/empty case ${item.id}`);ids.add(item.id);
  assert.ok(library.sources[item.repository],`unknown source ${item.id}`);
  assert.ok(typeof item.question==='string' && item.question.length>5);
  assert.ok(['development','held_out'].includes(item.split));
  assert.ok(item.template===null || known.has(item.template));
  assert.ok(['answered','insufficient_evidence'].includes(item.expected_status));
  assert.ok(Array.isArray(item.required_facts) && Array.isArray(item.forbidden_claims));
  assert.equal(item.required_facts.length>0,item.expected_status==='answered');
  for(const fact of item.required_facts){assert.ok(fact.text && fact.evidence.length);for(const anchor of fact.evidence){assert.ok(anchor.path && anchor.needle);assert.ok(!anchor.path.split('/').includes('..') && !anchor.path.startsWith('/'));}}
 }
 assert.ok(library.cases.some(c=>c.template));assert.ok(library.cases.some(c=>!c.template));
 assert.ok(library.cases.some(c=>c.expected_status==='insufficient_evidence'));
 return library;
}
export async function corpus(root,binary=resolve('target/release/vizzi-qa')){
 const child=spawn(binary,[root],{stdio:['pipe','pipe','pipe']});let stderr='',failure;
 const pending=[],queued=[];child.stderr.on('data',b=>{stderr=(stderr+b).slice(-12000);});
 const lines=createInterface({input:child.stdout});
 lines.on('line',line=>{try{const data=JSON.parse(line);const next=pending.shift();if(next)next.resolve(data);else queued.push(data);}catch(e){failure=e;for(const next of pending.splice(0))next.reject(e);}});
 child.on('error',e=>{failure=e;for(const next of pending.splice(0))next.reject(e);});
 child.on('exit',(code,signal)=>{failure=new Error(`QA process ended ${code}/${signal}: ${stderr}`);for(const next of pending.splice(0))next.reject(failure);});
 function next(){if(queued.length)return Promise.resolve(queued.shift());if(failure)return Promise.reject(failure);return new Promise((resolve,reject)=>{const entry={resolve:data=>{clearTimeout(timer);resolve(data);},reject:e=>{clearTimeout(timer);reject(e);}};const timer=setTimeout(()=>{const i=pending.indexOf(entry);if(i>=0)pending.splice(i,1);reject(new Error('QA process response timed out'));},120000);pending.push(entry);});}
 const ready=await next();assert.equal(ready.ready,true,JSON.stringify(ready));
 return {ready,pid:child.pid,async request(body){const response=next();child.stdin.write(JSON.stringify(body)+'\n');return response;},async close(){if(child.exitCode!==null)return;const stopped=once(child,'exit');child.stdin.end();const timer=setTimeout(()=>child.kill('SIGKILL'),5000);try{await stopped;}finally{clearTimeout(timer);lines.close();}}};
}
export const anchors=item=>item.required_facts.flatMap(f=>f.evidence.slice(0,1).map(a=>[a.path,a.needle]));
export function coverage(item,context){
 const matches=item.required_facts.map(fact=>fact.evidence.some(anchor=>context.evidence.some(e=>e.path===anchor.path && e.text.includes(anchor.needle))));
 return {facts:matches.length,matched:matches.filter(Boolean).length,all:matches.length?matches.every(Boolean):null};
}
export async function changing(root,state){
 const {rm,utimes,stat}=await import('node:fs/promises');
 await mkdir(join(root,'src'),{recursive:true});
 let modified;try{modified=(await stat(join(root,'config.toml'))).mtime;}catch{}
 await writeFile(join(root,'README.md'),'A transport fixture. Its current configuration is in config.toml.\n');
 await writeFile(join(root,'config.toml'),`retry_limit = ${state==='base'?3:7}\n`);
 if(modified)await utimes(join(root,'config.toml'),modified,modified);
 if(state==='deleted')await rm(join(root,'src/transport.rs'),{force:true});
 else await writeFile(join(root,'src/transport.rs'),`/// The packet loading handler.\npub fn ${state==='renamed'?'load_packet':'fetch_packet'}() {}\n`);
}
export async function prepare(output,roots){
 const library=validateLibrary(await json(libraryPath),await json(templatesPath));
 await mkdir(output);
 const sources={};
 for(const name of ['vizzi','lpc-rs']){
  const root=join(output,name),archive=join(output,name+'.tar'),revision=library.sources[name].revision;
  const result=spawnSync('git',['-C',roots[name],'archive','--format=tar','--output',archive,revision],{encoding:'utf8',env:{...process.env,GIT_OPTIONAL_LOCKS:'0'}});assert.equal(result.status,0,result.stderr);
  await mkdir(root);
  const extraction=spawnSync('python3',['-c',"import sys,tarfile; tarfile.open(sys.argv[1]).extractall(sys.argv[2],filter='data')",archive,root],{encoding:'utf8'});assert.equal(extraction.status,0,extraction.stderr);
  sources[name]={root,revision,archiveSha256:await digest(archive)};
 }
 const fixture=join(output,'changing');await changing(fixture,'base');sources.changing={root:fixture,revision:library.sources.changing.revision};
 const checks=[];
 for(const [name,source]of Object.entries(sources)){
  const index=await corpus(source.root);
  try{
   source.initial=index.ready;
   for(const item of library.cases.filter(c=>c.repository===name)){
    if(item.fixture_state){await changing(source.root,item.fixture_state);const update=await index.request({op:'refresh'});assert.ok(!update.error,update.error);}
    const oracle=await index.request({op:'oracle',id:item.id,anchors:anchors(item)});assert.ok(!oracle.error,`${item.id}: ${oracle.error}`);if(item.required_facts.length)assert.equal(coverage(item,oracle.context).all,true,`${item.id}: oracle excerpt omitted a required anchor`);
    const retrieved=await index.request({op:'retrieve',id:item.id,question:item.question});assert.ok(!retrieved.error,retrieved.error);
    checks.push({id:item.id,evidence_audited:true,retrieval:coverage(item,retrieved.context)});
   }
  }finally{await index.close();}
 }
 const result={schema:1,sources,checks,questionsSha256:await digest(libraryPath),templatesSha256:await digest(templatesPath),modelsSha256:await digest(modelPath),qaBinarySha256:await digest('target/release/vizzi-qa')};
 await writeFile(join(output,'prepared.json'),JSON.stringify(result,null,2)+'\n');return result;
}
