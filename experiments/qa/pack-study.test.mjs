import assert from 'node:assert/strict';
import {json} from './support.mjs';
import {packStudy} from './pack-study.mjs';
const original=await json('experiments/qa/library/maintainer-2026-11-05/cases.json'),packs=await json('experiments/qa/library/source-packs/packs.json'),suite=await json('experiments/qa/library/decomposition/cases.json');
const legacy=packStudy(original.cases,packs);
assert.deepEqual(legacy.arms.map(a=>a.id),['retrieved_ids','curated_ids','curated_long']);
assert.equal(legacy.cases,original.cases);
const selected=packStudy(original.cases,packs,suite);
assert.equal(selected.cases.length,15);assert.equal(selected.cases.filter(c=>c.granularity==='narrow').length,12);
assert.deepEqual(selected.arms.map(a=>a.fieldOrder),['status_first','answer_first']);
assert.ok(selected.arms.every(a=>a.selection==='pack' && a.wordLimit===300 && a.outputTokens===1024));
for(const change of [
 s=>{s.sourceRevision='wrong';},s=>{s.cases[0].question+=' altered';},s=>{s.cases[1].id=s.cases[0].id;},
 s=>{s.cases[1].parent_case='unknown';},s=>{s.cases[1].required_facts=[];},s=>{s.cases[1].expected_status='unsure';},
 s=>{s.cases[1].required_facts[0].evidence[0].path='../outside';},s=>{s.cases=s.cases.filter(c=>c.granularity!=='broad');},
]){const broken=structuredClone(suite);change(broken);assert.throws(()=>packStudy(original.cases,packs,broken));}
console.log('pack studies: unchanged legacy conditions, exact broad controls, guided subquestions, identical-pack output-order comparison');
