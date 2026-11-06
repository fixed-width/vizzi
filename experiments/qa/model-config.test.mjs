import assert from 'node:assert/strict';
import {chooseModel,chooseOrder} from './model-config.mjs';
import {json} from './support.mjs';
const original=await json('experiments/qa/models.json'),comparison=await json('experiments/qa/library/model-comparison/models.json');
assert.equal(chooseModel(original,'qwen3.5-2b-q4').sha256,comparison.models[0].sha256);
assert.equal(chooseModel(comparison,'qwen3.5-4b-q4').bytes,3013027808);
assert.throws(()=>chooseModel(comparison,'unknown'));
for(const edit of [c=>{c.models.push(c.models[0]);},c=>{c.models[1].sha256='bad';},c=>{c.models[1].revision='main';},c=>{c.models[1].filename='../model.gguf';},c=>{c.models[1].directory='/outside';}]){const c=structuredClone(comparison);edit(c);assert.throws(()=>chooseModel(c,'qwen3.5-4b-q4'));}
const arms=[{id:'status_first',fieldOrder:'status_first'},{id:'answer_first',fieldOrder:'answer_first'}];
assert.equal(chooseOrder(arms),arms);assert.deepEqual(chooseOrder(arms,'answer_first'),[arms[1]]);
assert.throws(()=>chooseOrder(arms,'unknown'));assert.throws(()=>chooseOrder([{id:'legacy'}],'answer_first'));
console.log('model comparison: pinned artifacts, safe local model paths, unchanged baseline selection and explicit single-order conditions');
