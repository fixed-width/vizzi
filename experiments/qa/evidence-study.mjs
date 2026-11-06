import assert from 'node:assert/strict';
import {assess} from './compact-assess.mjs';

const safePath=path=>!path.includes('\\') && !path.includes(':') && !path.split('/').some(p=>['','..','.'].includes(p));
export function validateEvidenceStudy(config,development,reserved){
 assert.equal(config.schema,1);assert.equal(config.repeats,2);
 for(const source of Object.values(config.sources)){
  assert.ok(['fixture','git'].includes(source.kind));
  if(source.kind==='fixture')assert.ok(safePath(source.root));else assert.match(source.revision,/^[a-f0-9]{40}$/);
  for(const [path,file]of Object.entries(source.files)){assert.ok(safePath(path));assert.match(file.sha256,/^[a-f0-9]{64}$/);assert.ok(file.identityNeedle);}
 }
 const ids=new Set(),families=new Map(),hashes=new Map();
 for(const [split,library]of [['development',development],['reserved',reserved]]){
  assert.equal(library.schema,1);assert.ok(library.cases.length);
  const groups=new Map();
  for(const item of library.cases){
   assert.ok(item.id && !ids.has(item.id));ids.add(item.id);assert.equal(item.split,split);
   assert.ok(item.family && item.question && item.repository);assert.ok(config.sources[item.source]);
   if(families.has(item.family))assert.equal(families.get(item.family),split,'source family crosses splits');families.set(item.family,split);
   assert.equal(item.expected_status,item.condition==='missing'?'insufficient_evidence':'answered');
   assert.equal(item.required_facts.length>0,item.expected_status==='answered');assert.ok(Array.isArray(item.forbidden_claims));
   assert.ok(item.ranges.length>=1 && item.ranges.length<=8);
   const keys=new Set();
   for(const [index,range]of item.ranges.entries()){
    assert.ok(range.key && !keys.has(range.key));keys.add(range.key);
    const file=config.sources[item.source].files[range.path];assert.ok(file);
    assert.ok(Number.isInteger(range.first) && range.first>=1 && Number.isInteger(range.last) && range.last>=range.first);
    assert.ok(!item.ranges.slice(0,index).some(other=>other.path===range.path && other.first<=range.last && range.first<=other.last),'overlapping source ranges');
    if(hashes.has(file.sha256))assert.equal(hashes.get(file.sha256),split,'source bytes cross splits');hashes.set(file.sha256,split);
   }
   assert.equal(keys.has('target'),item.condition!=='missing','missing-source case must omit decisive definition');
   assert.ok(item.expected_support.every(key=>keys.has(key)));assert.equal(item.expected_support.length>0,item.expected_status==='answered');
   for(const fact of item.required_facts){assert.ok(fact.text && fact.evidence.length);for(const anchor of fact.evidence)assert.ok(item.ranges.some(r=>r.path===anchor.path) && anchor.needle);}
   if(!groups.has(item.family))groups.set(item.family,[]);groups.get(item.family).push(item);
  }
  for(const group of groups.values()){
   const conditions=group.map(c=>c.condition).sort();assert.deepEqual(conditions,split==='development'?['changed','missing','original','relabel']:['missing','original']);
   const original=group.find(c=>c.condition==='original');assert.ok(group.every(c=>c.question===original.question));
   if(split==='development'){
    const changed=group.find(c=>c.condition==='changed'),relabel=group.find(c=>c.condition==='relabel');
    assert.notDeepEqual(changed.required_facts,original.required_facts,'changed source must change gold facts');
    const fingerprint=c=>c.ranges.map(r=>config.sources[c.source].files[r.path].sha256).sort();assert.notDeepEqual(fingerprint(changed),fingerprint(original));
    assert.equal(relabel.source,original.source);assert.deepEqual(relabel.required_facts,original.required_facts);assert.deepEqual(relabel.expected_support,original.expected_support);
    assert.notDeepEqual(relabel.ranges,original.ranges,'relabel must permute source order');
    const canonical=c=>c.ranges.map(r=>JSON.stringify(r)).sort();assert.deepEqual(canonical(relabel),canonical(original));
   }
  }
 }
 return {config,development,reserved};
}
export function developmentRuns(library,repeats){
 assert.equal(repeats,2);assert.ok(library.cases.every(c=>c.split==='development'),'inference only accepts development cases');
 return Array.from({length:repeats},(_,i)=>library.cases.map(item=>({id:`${item.id}/repeat-${i+1}`,repeat:i+1,case:item}))).flat();
}
export function modelQuestion(item){return {repository:item.repository,question:item.question};}

export function diagnoseEvidence(rows,library,grading){
 const plan=developmentRuns(library,2),expected=new Map(plan.map(p=>[p.id,p]));assert.equal(rows.length,plan.length);
 for(const row of rows){const p=expected.get(row.id);assert.ok(p);assert.equal(row.case,p.case.id);assert.equal(row.repeat,p.repeat);}
 const assessment=assess(rows,library,grading),decisions=new Map(assessment.decisions.map(d=>[d.id,d]));
 const flags=row=>{const d=decisions.get(row.id);return {factual:d.supportedFacts!==d.facts.length,status:!d.statusCorrect,unsupported_claims:d.unsupportedClaims.length>0,citation_support:d.answerable && !d.citationsSupport,reference_binding:!row.provenanceValid,execution:Boolean(row.runtimeError || row.formatError || row.finish!=='stop')};};
 const failures=rows.map(row=>({id:row.id,case:row.case,repeat:row.repeat,...flags(row)})),contrasts=[],reproducible=[];
 for(const family of new Set(library.cases.map(c=>c.family))){
  const cases=library.cases.filter(c=>c.family===family),condition=name=>cases.find(c=>c.condition===name);
  for(const repeat of [1,2]){
   const decision=name=>decisions.get(`${condition(name).id}/repeat-${repeat}`),pass=name=>decision(name).taskSuccess;
   const factual=name=>decision(name).supportedFacts===decision(name).facts.length && !decision(name).unsupportedClaims.length;
   contrasts.push({family,repeat,sourceChange:pass('original') && pass('changed'),factualChange:factual('original') && factual('changed'),labelPermutation:pass('original') && pass('relabel'),missingAbstention:pass('missing')});
  }
 }
 for(const failure of ['factual','status','unsupported_claims','citation_support','reference_binding','execution']){
  const repeated=library.cases.filter(c=>failures.filter(f=>f.case===c.id).every(f=>f[failure]));
  if(repeated.length>=2)reproducible.push({failure,cases:repeated.map(c=>c.id).sort(),families:[...new Set(repeated.map(c=>c.family))].sort()});
 }
 return {...assessment,contrasts,reproducible,failures};
}
