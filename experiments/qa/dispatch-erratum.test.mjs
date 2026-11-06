import assert from 'node:assert/strict';
import {json} from './support.mjs';
const root='experiments/qa/results/dispatch-erratum';
const audit=await json(root+'/source-audit.json'),prepared=await json('experiments/qa/results/source-packs/prepared.json');
const block=prepared.audit.find(c=>c.id==='maintainer.lpc-rs.commit-checks').context.evidence.find(e=>e.path===audit.path && e.start_line===302);
assert.equal(block.scope.name,'dispatch');assert.deepEqual(block.scope.parents,['Committer']);
assert.ok(audit.literalSource.includes('let keep_running = self.dispatch(msg, tx);'));
assert.ok(audit.literalSource.includes('fn dispatch(&mut self, msg: CommitProtocol'));
assert.ok(audit.literalSource.includes('let result = self.commit(changeset);'));
const original=await json('experiments/qa/library/decomposition/cases.json'),corrected=await json(root+'/decomposition-library.corrected.json');
const receiver=library=>library.cases.find(c=>c.id==='decomp.commit-checks.commit-receiver');
const verify=library=>assert.ok(receiver(library).required_facts[0].text.includes('Committer::'+block.scope.name),'receiver rubric must agree with captured lexical scope and source audit');
assert.throws(()=>verify(original));verify(corrected);
assert.equal(receiver(original).question,receiver(corrected).question,'correction must not rewrite the model question');
const grades=await json(root+'/decomposition-grades.corrected.json');
for(const arm of ['status_first','answer_first']){const g=grades.grades.find(g=>g.id==='decomp.commit-checks.commit-receiver/'+arm);assert.deepEqual(g.facts,['supported']);assert.deepEqual(g.unsupportedClaims,[]);assert.equal(g.citationsSupport,true);}
console.log('dispatch erratum: captured source/scope contradict the old receiver oracle; corrected rubric and both unchanged answers agree');
