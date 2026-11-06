import assert from 'node:assert/strict';

export function chooseModel(config,id){
 assert.equal(config.schema,1);assert.ok(Array.isArray(config.models) && config.models.length>0);
 const seen=new Set();
 for(const model of config.models){
  assert.ok(model.id && !seen.has(model.id));seen.add(model.id);
  assert.match(model.revision,/^[a-f0-9]{40}$/);assert.match(model.sha256,/^[a-f0-9]{64}$/);
  assert.ok(Number.isSafeInteger(model.bytes) && model.bytes>0);
  for(const path of [model.directory,model.filename])assert.ok(typeof path==='string' && path.length && !path.startsWith('/') && !path.split('/').some(p=>p==='..' || p==='' || p==='.'),'invalid local model path');
  assert.ok(!model.filename.includes('/') && model.filename.endsWith('.gguf'));
 }
 const model=config.models.find(model=>model.id===id);assert.ok(model,`unknown configured model: ${id}`);return model;
}
export function chooseOrder(arms,fieldOrder){
 if(fieldOrder===undefined)return arms;
 assert.ok(['status_first','answer_first'].includes(fieldOrder));
 const selected=arms.filter(a=>a.fieldOrder===fieldOrder);assert.equal(selected.length,1,'output-order selection requires a paired-order study');return selected;
}
