import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {json,libraryPath} from './support.mjs';
const args=process.argv.slice(2),option=name=>{const i=args.indexOf(name);return i<0?undefined:args[i+1];};
assert.ok(option('--input') && option('--output'));
const cases=new Map((await json(libraryPath)).cases.map(c=>[c.id,c]));
const rows=(await readFile(option('--input'),'utf8')).trim().split('\n').filter(Boolean).map(line=>JSON.parse(line));
let text='# Source-aware response review\n\nThis evaluator artifact contains gold facts. Do not index it or supply it to candidates.\n\n';
for(const row of rows){
 const item=cases.get(row.case);text+=`## ${row.id}\n\n${item.question}\n\nExpected: ${item.expected_status}; split: ${item.split}; category: ${item.category}.\n\n`;
 text+=item.required_facts.map((f,i)=>`- Fact ${i+1}: ${f.text}\n`).join('');
 text+=item.forbidden_claims.map(f=>`- Forbidden: ${f}\n`).join('');
 const answer=row.answer?.answer??row.raw??row.runtimeError??'';const fence='`'.repeat(Math.max(3,...(answer.match(/`+/g)??[]).map(s=>s.length))+1);
 text+=`\nCandidate status: ${row.answer?.status??'format/runtime error'}\n\n${fence}text\n${answer}\n${fence}\n\n`;
 for(const cite of row.answer?.citations??[]){const source=row.context.evidence.find(e=>e.label===cite.label);text+=`- Citation ${cite.label}: ${source?.path??'unavailable'} — ${JSON.stringify(cite.quote)}\n`;}
 text+=`\nProvenance: ${row.provenanceValid}; ${row.provenanceError??''}. Preferred anchors: ${row.anchorCoverage.matched}/${row.anchorCoverage.facts}.\n\n`;
}
await writeFile(option('--output'),text);console.log(`${rows.length} responses exported for separate rubric review`);
