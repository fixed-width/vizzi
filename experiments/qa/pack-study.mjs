import assert from 'node:assert/strict';

export function packStudy(original,packs,suite){
 if(!suite)return {cases:original,arms:[{id:'retrieved_ids',selection:'compact',wordLimit:100,outputTokens:384},{id:'curated_ids',selection:'pack',wordLimit:100,outputTokens:384},{id:'curated_long',selection:'pack',wordLimit:300,outputTokens:1024}]};
 assert.equal(suite.schema,1);assert.equal(suite.kind,'decomposition_order');assert.equal(suite.sourceRevision,packs.sourceRevision);
 assert.equal(suite.wordLimit,300);assert.equal(suite.outputTokens,1024);
 const parents=new Map(original.map(c=>[c.id,c])),knownPacks=new Set(packs.cases.map(c=>c.id)),seen=new Set(),broad=new Set();
 assert.ok(Array.isArray(suite.cases) && suite.cases.length>original.length);
 for(const item of suite.cases){
  assert.ok(item.id && !seen.has(item.id));seen.add(item.id);
  assert.ok(parents.has(item.parent_case) && knownPacks.has(item.parent_case));
  assert.equal(item.repository,'lpc-rs');assert.equal(item.split,'diagnostic');assert.equal(item.expected_status,'answered');
  assert.ok(['broad','narrow'].includes(item.granularity));assert.ok(item.question && item.author);
  assert.ok(item.required_facts.length && Array.isArray(item.forbidden_claims));
  for(const fact of item.required_facts){assert.ok(fact.text && fact.evidence.length);for(const anchor of fact.evidence){assert.ok(anchor.path && anchor.needle && !anchor.path.startsWith('/') && !anchor.path.split('/').includes('..'));}}
  if(item.granularity==='broad'){
   assert.ok(!broad.has(item.parent_case));broad.add(item.parent_case);
   const parent=parents.get(item.parent_case);
   assert.equal(item.question,parent.question);assert.deepEqual(item.required_facts,parent.required_facts);assert.deepEqual(item.forbidden_claims,parent.forbidden_claims);
  }
 }
 assert.equal(broad.size,original.length,'retain every original broad question');
 return {cases:suite.cases,arms:['status_first','answer_first'].map(fieldOrder=>({id:fieldOrder,selection:'pack',fieldOrder,wordLimit:suite.wordLimit,outputTokens:suite.outputTokens}))};
}
