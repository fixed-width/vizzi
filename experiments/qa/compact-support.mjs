import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readdir} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {corpus,json,digest,templatesPath,modelPath,validateLibrary,coverage,changing} from './support.mjs';

export const casesPath=resolve('experiments/qa/library/v2/cases.json');
export const cohortPath=resolve('experiments/qa/library/v2/cohort.json');
export const evidenceHash=context=>createHash('sha256').update(JSON.stringify(context)).digest('hex');
export const selectedCases=(library,split)=>{
 assert.ok(['development','held_out'].includes(split));
 return library.cases.filter(c=>c.split===split);
};
export async function inputs(){
 const library=validateLibrary(await json(casesPath),await json(templatesPath)),cohort=await json(cohortPath);
 assert.equal(new Set(cohort.case_ids).size,cohort.case_ids.length);
 assert.ok(cohort.case_ids.every(id=>library.cases.some(c=>c.id===id)));
 assert.ok(selectedCases(library,'held_out').every(c=>cohort.case_ids.includes(c.id)));
 return {library,cohort};
}
export async function verifyPrepared(path){
 const prepared=await json(path);
 assert.equal(prepared.questionsSha256,await digest(casesPath),'questions changed; repeat source audit');
 assert.equal(prepared.qaBinarySha256,await digest('target/release/vizzi-qa'),'native binary changed; repeat source audit');
 assert.equal(prepared.templatesSha256,await digest(templatesPath));
 assert.equal(prepared.modelsSha256,await digest(modelPath));
 const {library}=await inputs();
 assert.equal(prepared.checks.length,library.cases.length);
 for(const item of library.cases){
  const check=prepared.checks.find(c=>c.id===item.id);
  assert.equal(check?.evidence_audited,true);
  if(item.split==='held_out')assert.equal(check.retrieval,undefined,'fresh retrieval must wait for freeze');
 }
 return prepared;
}
export async function captureHashes(paths){
 const files={};for(const path of paths)files[resolve(path)]=await digest(path);return files;
}
export async function verifyHashes(files){
 for(const [path,hash] of Object.entries(files))assert.equal(await digest(path),hash,`frozen input changed: ${path}`);
}
export async function freezeInputs(preparedPath){
 const prepared=await verifyPrepared(preparedPath);
 const rust=[];for(const crate of ['vizzi-qa','vizzi-search'])for(const file of await readdir('crates/'+crate+'/src'))if(file.endsWith('.rs'))rust.push(join('crates',crate,'src',file));
 const paths=[...rust,'crates/vizzi-qa/Cargo.toml','crates/vizzi-search/Cargo.toml','Cargo.toml','Cargo.lock','experiments/qa/support.mjs','experiments/qa/model.mjs','experiments/qa/compact-support.mjs','experiments/qa/compact.mjs',casesPath,cohortPath,templatesPath,modelPath,preparedPath,'target/release/vizzi-qa','target/release/vizzi-envelope'];
 return {schema:1,createdAt:new Date().toISOString(),preparedPath:resolve(preparedPath),files:await captureHashes(paths),sourceRevisions:Object.fromEntries(Object.entries(prepared.sources).map(([name,s])=>[name,s.revision])),development:'Only the 32 previously observed formulations were available for retrieval tuning. Fresh gold anchors were audited, but their candidate retrieval/model outputs were not inspected before this freeze.',authorship:'Fresh cases are agent-authored and source-aware, not independent maintainer questions.'};
}
export async function indexesFor(prepared){
 const indexes={};
 try{
  for(const [name,source] of Object.entries(prepared.sources)){
   if(name==='changing')await changing(source.root,'base');
   const index=await corpus(source.root);indexes[name]=index;
   assert.equal(index.ready.snapshot,source.initial.snapshot,`captured source changed: ${name}`);
  }
  return indexes;
 }catch(e){for(const index of Object.values(indexes))await index.close();throw e;}
}
export async function retrievalReport(prepared,library,split){
 const indexes=await indexesFor(prepared),records=[];
 try{
  for(const item of selectedCases(library,split)){
   const index=indexes[item.repository];
   if(item.fixture_state){await changing(prepared.sources.changing.root,item.fixture_state);const update=await index.request({op:'refresh'});assert.ok(!update.error,update.error);}
   for(const selector of ['legacy','compact'])for(const budget of [8000,2500]){
    const limit=budget===2500?3:6;
    const response=await index.request({op:'retrieve',id:`${item.id}/${selector}/${budget}`,question:item.question,selector,limit,budget});
    assert.ok(!response.error,response.error);
    records.push({case:item.id,repository:item.repository,split,selector,budget,limit,expectedStatus:item.expected_status,coverage:coverage(item,response.context),elapsedMs:response.elapsed_ms,evidenceBytes:response.context.evidence.reduce((n,e)=>n+Buffer.byteLength(e.text),0),context:response.context});
   }
  }
 }finally{for(const index of Object.values(indexes))await index.close();}
 const summaries=[];
 for(const selector of ['legacy','compact'])for(const budget of [8000,2500]){
  const group=records.filter(r=>r.selector===selector && r.budget===budget),answerable=group.filter(r=>r.expectedStatus==='answered');
  summaries.push({selector,budget,cases:group.length,answerable:answerable.length,completePreferredAnchors:answerable.filter(r=>r.coverage.all).length,matchedFacts:answerable.reduce((n,r)=>n+r.coverage.matched,0),facts:answerable.reduce((n,r)=>n+r.coverage.facts,0)});
 }
 return {schema:1,split,limitations:'Literal preferred-anchor overlap is a narrow diagnostic, not semantic evidence completeness or answer correctness.',summaries,records};
}
