import assert from 'node:assert/strict';

export function validateMaintainer(questions,library){
 assert.equal(questions.schema,1);assert.equal(library.schema,1);
 assert.equal(questions.repository,'lpc-rs');
 assert.equal(library.sources['lpc-rs'].revision,questions.source_revision);
 assert.equal(questions.questions.length,3);assert.equal(library.cases.length,3);
 const original=new Map(questions.questions.map(q=>[q.id,q.question])),seen=new Set();
 assert.equal(original.size,3);
 for(const item of library.cases){
  assert.ok(!seen.has(item.id) && original.has(item.id));seen.add(item.id);
  assert.equal(item.question,original.get(item.id),'preserve exact maintainer wording');
  assert.equal(item.repository,questions.repository);assert.equal(item.split,'held_out');
  assert.equal(item.expected_status,'answered');
  assert.ok(Array.isArray(item.required_facts) && item.required_facts.length>0);
  assert.ok(Array.isArray(item.forbidden_claims));
  for(const fact of item.required_facts){assert.ok(fact.text && fact.evidence?.length);}
  const anchors=[...item.required_facts.flatMap(f=>f.evidence),...(item.supporting_evidence??[])];
  for(const anchor of anchors){
   assert.ok(anchor.path && anchor.needle);
   assert.ok(!anchor.path.startsWith('/') && !anchor.path.split('/').includes('..'));
  }
 }
 return library.cases;
}
