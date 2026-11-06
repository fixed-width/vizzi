import assert from 'node:assert/strict';
import {validateEvidenceStudy,developmentRuns,modelQuestion,diagnoseEvidence} from './evidence-study.mjs';
import {prompt} from './model.mjs';
import {json} from './support.mjs';

const files=sha=>({'src/x.rs':{sha256:sha.repeat(64),identityNeedle:'fn check'}});
const config={schema:1,repeats:2,sources:{a:{kind:'fixture',root:'fixtures/a',files:files('a')},b:{kind:'fixture',root:'fixtures/b',files:files('b')},real:{kind:'git',revision:'c'.repeat(40),files:{'src/y.py':{sha256:'c'.repeat(64),identityNeedle:'def other'}}}}};
const ranges=['target','helper','noise'].map((key,i)=>({key,path:'src/x.rs',first:i*2+1,last:i*2+2}));
const fact=text=>({text,evidence:[{path:'src/x.rs',needle:'fn check'}]});
const original={id:'f.original',family:'f',repository:'fixture',source:'a',split:'development',condition:'original',question:'Which relation rejects the request?',expected_status:'answered',required_facts:[fact('Greater')],forbidden_claims:[],expected_support:['target'],ranges};
const dev={schema:1,cases:[original,{...original,id:'f.changed',condition:'changed',source:'b',required_facts:[fact('Less')]},{...original,id:'f.relabel',condition:'relabel',ranges:[ranges[2],ranges[1],ranges[0]]},{...original,id:'f.missing',condition:'missing',expected_status:'insufficient_evidence',required_facts:[],expected_support:[],ranges:ranges.slice(1)}]};
const reservedOriginal={...original,id:'r.original',family:'r',source:'real',split:'reserved',ranges:[{key:'target',path:'src/y.py',first:1,last:2}],required_facts:[{text:'Other',evidence:[{path:'src/y.py',needle:'def other'}]}]};
const reserved={schema:1,cases:[reservedOriginal,{...reservedOriginal,id:'r.missing',condition:'missing',expected_status:'insufficient_evidence',required_facts:[],expected_support:[],ranges:[{key:'noise',path:'src/y.py',first:3,last:4}]}]};
const study=validateEvidenceStudy(config,dev,reserved);
assert.equal(study.development.cases.length,4);
assert.equal(study.reserved.cases.length,2);
for(const mutate of [
 s=>{s.reserved.cases.forEach(c=>c.family='f');},
 s=>{s.config.sources.real.files['src/y.py'].sha256='a'.repeat(64);},
 s=>{s.development.cases[3].ranges=ranges;},
 s=>{s.development.cases[1].required_facts=original.required_facts;},
 s=>{s.development.cases[2].ranges=ranges;},
 s=>{s.development.cases[0].ranges=[{...ranges[0],path:'../x.rs'}];},
 s=>{s.development.cases[0].ranges=[ranges[0],{...ranges[1],first:2}];},
 s=>{s.development.cases[0].source='absent';},
 s=>{s.config.repeats=0;}
]){const s=structuredClone({config,development:dev,reserved});mutate(s);assert.throws(()=>validateEvidenceStudy(s.config,s.development,s.reserved));}
const rejects=mutate=>{const s=structuredClone({config,development:dev,reserved});mutate(s);assert.throws(()=>validateEvidenceStudy(s.config,s.development,s.reserved));};
for(const root of ['', '/', '../a', './a', 'a/../b', 'a//b', 'a/.', 'a/', 'a\\b', 'c:folder', null, 12])rejects(s=>{s.config.sources.a.root=root;});
for(const invalid of ['','a'.repeat(63),'a'.repeat(65),'x'+'a'.repeat(64),'a'.repeat(64)+'x','g'.repeat(64)])rejects(s=>{s.config.sources.a.files['src/x.rs'].sha256=invalid;});
for(const invalid of ['','c'.repeat(39),'x'+'c'.repeat(40),'c'.repeat(40)+'x'])rejects(s=>{s.config.sources.real.revision=invalid;});
for(const change of [
 s=>{s.config.schema=2;},s=>{s.development.schema=2;},s=>{s.reserved.schema=2;},
 s=>{s.development.cases=[];},s=>{s.reserved.cases=[];},
 s=>{s.config.sources.a.kind='other';},s=>{s.config.sources.a.files['src/x.rs'].identityNeedle='';},
 s=>{s.config.sources.a.files['../escape.rs']=s.config.sources.a.files['src/x.rs'];},
 s=>{s.development.cases[0].id='';},s=>{s.reserved.cases[0].id=original.id;},
 s=>{s.development.cases[0].split='reserved';},s=>{s.development.cases[0].family='';},
 s=>{s.development.cases.forEach(c=>c.question='');},s=>{s.development.cases[0].repository='';},
 s=>{s.development.cases[0].expected_status='unknown';},s=>{s.development.cases[0].required_facts=[];},
 s=>{s.development.cases[0].forbidden_claims=null;},s=>{s.development.cases[0].ranges=[];},
 s=>{s.development.cases[3].ranges=[];},
 s=>{s.development.cases[0].ranges[1].key='target';},s=>{s.development.cases[0].ranges[1].key='';},
 s=>{s.development.cases[0].ranges[0].path='src/absent.rs';},
 s=>{s.development.cases[0].expected_support=[];},s=>{s.development.cases[0].expected_support=['absent'];},
 s=>{s.development.cases[0].required_facts[0].text='';},s=>{s.development.cases[0].required_facts[0].evidence=[];},
 s=>{s.development.cases[0].required_facts[0].evidence[0].needle='';},s=>{s.development.cases[0].required_facts[0].evidence[0].path='other.rs';},
 s=>{s.development.cases[1].question='A different question?';},
 s=>{s.development.cases[1].source='a';s.development.cases[1].ranges=[ranges[2],ranges[1],ranges[0]];},
 s=>{s.development.cases[2].required_facts=[fact('Changed')];},s=>{s.development.cases[2].source='b';},
 s=>{s.development.cases[2].expected_support=['helper'];},s=>{s.development.cases[2].ranges[0]={...s.development.cases[2].ranges[0],last:7};}
])rejects(change);
for(const field of ['first','last'])for(const value of [0,1.5,'2'])rejects(s=>{s.development.cases[0].ranges[0][field]=value;});
rejects(s=>{s.development.cases[0].ranges[0].first=3;});
rejects(s=>{s.development.cases[0].ranges[1].first=2;});
rejects(s=>{s.development.cases[0].ranges[1].last=5;});
const wide=structuredClone(dev),eight=[...ranges,...Array.from({length:5},(_,i)=>({key:'extra'+i,path:'src/x.rs',first:10+i,last:10+i}))];
for(const c of wide.cases)c.ranges=c.condition==='missing'?eight.slice(1):c.condition==='relabel'?[...eight].reverse():eight;
validateEvidenceStudy(config,wide,reserved);
const tooWide=structuredClone(wide),nine=[...eight,{key:'ninth',path:'src/x.rs',first:99,last:99}];
for(const c of tooWide.cases)c.ranges=c.condition==='missing'?nine.slice(1):c.condition==='relabel'?[...nine].reverse():[...nine];
assert.throws(()=>validateEvidenceStudy(config,tooWide,reserved));
for(const reverse of [false,true])rejects(s=>{const overlapping=[{key:'target',path:'src/y.py',first:1,last:2},{key:'helper',path:'src/y.py',first:2,last:3}];s.reserved.cases[0].ranges=reverse?overlapping.reverse():overlapping;});
const single=structuredClone(dev);for(const c of single.cases)for(const r of c.ranges)r.last=r.first;validateEvidenceStudy(config,single,reserved);
const multiConfig=structuredClone(config),multi=structuredClone(dev);
for(const source of ['a','b'])multiConfig.sources[source].files['src/helper.rs']={sha256:(source==='a'?'d':'e').repeat(64),identityNeedle:'helper'};
for(const c of multi.cases)for(const r of c.ranges)if(r.key==='helper'){r.path='src/helper.rs';r.first=1;r.last=2;}
validateEvidenceStudy(multiConfig,multi,reserved);
const sameBytes=structuredClone(multi);sameBytes.cases[1].source='a';sameBytes.cases[1].ranges=[sameBytes.cases[1].ranges[1],sameBytes.cases[1].ranges[0],sameBytes.cases[1].ranges[2]];assert.throws(()=>validateEvidenceStudy(multiConfig,sameBytes,reserved));
validateEvidenceStudy(config,{schema:1,cases:[...dev.cases].reverse()},reserved);
const plan=developmentRuns(study.development,2);
assert.equal(plan.length,8);assert.equal(new Set(plan.map(r=>r.id)).size,8);
assert.deepEqual(plan.map(r=>r.repeat),[1,1,1,1,2,2,2,2]);
assert.throws(()=>developmentRuns(reserved,2),/development/);
assert.throws(()=>developmentRuns({cases:[original,reservedOriginal]},2),/development/);
for(const repeat of [1,3])assert.throws(()=>developmentRuns(dev,repeat));
assert.deepEqual(modelQuestion({...original,secret:'HIDDEN_GOLD'}),{repository:'fixture',question:original.question});
assert.ok(!JSON.stringify(prompt(modelQuestion({...original,question:'Where is check?',required_facts:[{text:'HIDDEN_GOLD'}]}),{snapshot:'s',evidence:[]})).includes('HIDDEN_GOLD'));

const rows=plan.map(p=>({id:p.id,case:p.case.id,repeat:p.repeat,arm:'answer_first',split:'development',repository:'fixture',answer:{status:p.case.expected_status},provenanceValid:true,finish:'stop'}));
const grading={grader:'fixture',limitations:[],grades:plan.map(p=>({id:p.id,facts:p.case.required_facts.map(()=> 'supported'),unsupportedClaims:[],citationsSupport:true,roleConfusion:false,notes:'fixture'}))};
const good=diagnoseEvidence(rows,dev,grading);
assert.equal(good.reproducible.length,0);
assert.equal(good.contrasts.length,2);
assert.ok(good.contrasts.every(c=>c.factualChange));
assert.ok(good.contrasts.every(c=>c.sourceChange && c.labelPermutation && c.missingAbstention));
for(const g of grading.grades.filter(g=>/f\.(original|relabel)\//.test(g.id)))g.citationsSupport=false;
const bad=diagnoseEvidence(rows,dev,grading);
assert.deepEqual(bad.reproducible,[{failure:'citation_support',cases:['f.original','f.relabel'],families:['f']}]);
grading.grades.find(g=>g.id==='f.relabel/repeat-2').citationsSupport=true;
assert.equal(diagnoseEvidence(rows,dev,grading).reproducible.length,0,'one-repeat failures do not meet the diagnostic threshold');
assert.throws(()=>diagnoseEvidence(rows.slice(1),dev,{...grading,grades:grading.grades.slice(1)}));
const twoFamilies={schema:1,cases:[...dev.cases.map(c=>({...c,id:c.id.replace('f.','g.'),family:'g'})),...dev.cases]};
const twoPlan=developmentRuns(twoFamilies,2);
const twoRows=twoPlan.map(p=>({id:p.id,case:p.case.id,repeat:p.repeat,arm:'answer_first',split:'development',repository:'fixture',answer:{status:p.case.condition==='missing'?'answered':p.case.expected_status},provenanceValid:true,finish:'stop'}));
const twoGrades={grader:'fixture',limitations:[],grades:twoPlan.map(p=>({id:p.id,facts:p.case.required_facts.map(()=> 'supported'),unsupportedClaims:[],citationsSupport:true,roleConfusion:false,notes:'fixture'}))};
assert.equal(diagnoseEvidence(twoRows,twoFamilies,twoGrades).reproducible.filter(r=>r.failure==='status').length,1,'same failure across two families must be diagnosed');
assert.deepEqual(diagnoseEvidence(twoRows,twoFamilies,twoGrades).reproducible.find(r=>r.failure==='status'),{failure:'status',cases:['f.missing','g.missing'],families:['f','g']});
for(const g of twoGrades.grades.filter(g=>g.id.startsWith('g.changed/')))g.facts=['missing'];
const distinct=diagnoseEvidence(twoRows,twoFamilies,twoGrades);
assert.equal(distinct.contrasts.length,4);
assert.ok(distinct.contrasts.filter(c=>c.family==='f').every(c=>c.sourceChange));
assert.ok(distinct.contrasts.filter(c=>c.family==='g').every(c=>!c.sourceChange && !c.factualChange));

const fresh=()=>{
 const rows=plan.map(p=>({id:p.id,case:p.case.id,repeat:p.repeat,arm:'answer_first',split:'development',repository:'fixture',answer:{status:p.case.expected_status},provenanceValid:true,finish:'stop'}));
 const grades={grader:'fixture',limitations:[],grades:plan.map(p=>({id:p.id,facts:p.case.required_facts.map(()=> 'supported'),unsupportedClaims:[],citationsSupport:true,roleConfusion:false,notes:'fixture'}))};return {rows,grades};
};
for(const [failure,edit]of [
 ['factual',(_r,g)=>{g.facts=['missing'];}],
 ['status',(r)=>{r.answer.status='insufficient_evidence';}],
 ['unsupported_claims',(_r,g)=>{g.unsupportedClaims=['False API claim'];}],
 ['reference_binding',r=>{r.provenanceValid=false;}],
 ['execution',r=>{r.runtimeError='failed';}],
 ['execution',r=>{r.formatError='invalid JSON';}],
 ['execution',r=>{r.finish='length';}]
]){
 const {rows,grades}=fresh();for(const row of rows.filter(r=>['f.original','f.relabel'].includes(r.case)))edit(row,grades.grades.find(g=>g.id===row.id));
 const result=diagnoseEvidence(rows,dev,grades);assert.deepEqual(result.reproducible.find(r=>r.failure===failure),{failure,cases:['f.original','f.relabel'],families:['f']});
 if(['factual','unsupported_claims'].includes(failure))assert.ok(result.contrasts.every(c=>!c.factualChange));
}
for(const condition of ['changed','relabel','missing']){
 const {rows,grades}=fresh();for(const row of rows.filter(r=>r.case==='f.'+condition)){if(condition==='missing')row.answer.status='answered';else grades.grades.find(g=>g.id===row.id).facts=['missing'];}
 const result=diagnoseEvidence(rows,dev,grades);
 if(condition==='changed')assert.ok(result.contrasts.every(c=>!c.sourceChange && !c.factualChange && c.labelPermutation));
 if(condition==='relabel')assert.ok(result.contrasts.every(c=>c.sourceChange && !c.labelPermutation));
 if(condition==='missing')assert.ok(result.contrasts.every(c=>!c.missingAbstention));
}
for(const edit of [r=>{r.id='unknown';},r=>{r.case='f.changed';},r=>{r.repeat=3;}]){const f=fresh();edit(f.rows[0]);assert.throws(()=>diagnoseEvidence(f.rows,dev,f.grades));}
const actual='experiments/qa/library/evidence-use/';
const real=validateEvidenceStudy(await json(actual+'cohort.json'),await json(actual+'development.json'),await json(actual+'reserved.json'));
assert.equal(real.development.cases.length,16);assert.equal(real.reserved.cases.length,8);
assert.equal(developmentRuns(real.development,real.config.repeats).length,32);
console.log('evidence study: grouped splits, source/range contrasts, reserved inference rejection, gold isolation and repeat-aware diagnosis');
