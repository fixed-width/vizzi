import assert from 'node:assert/strict';
import {json} from './support.mjs';
import {validateMaintainer} from './maintainer-cases.mjs';
const directory='experiments/qa/library/maintainer-2026-11-05';
const questions=await json(directory+'/questions.json'),library=await json(directory+'/cases.json');
assert.equal(validateMaintainer(questions,library).length,3);
for(const mutation of [
 d=>{d.cases[0].question+=' (native Rust)';},
 d=>{d.cases[1].id=d.cases[0].id;},
 d=>{d.cases[0].repository='vizzi';},
 d=>{d.cases[0].required_facts=[];},
 d=>{d.sources['lpc-rs'].revision='changed';},
 d=>{d.cases[0].required_facts[0].evidence[0].path='../outside.rs';},
]){
 const changed=structuredClone(library);mutation(changed);assert.throws(()=>validateMaintainer(questions,changed));
}
console.log('maintainer probes: exact independent question wording, source pin, identities and source-bound facts');
