// Serial command-runner fallback for environments that prohibit Stryker's log socket.
import assert from "node:assert/strict";
import {Instrumenter} from "@stryker-mutator/instrumenter";
import {mkdtemp,readFile,writeFile,readdir,rm} from "node:fs/promises";
import {spawnSync} from "node:child_process";
import {tmpdir} from "node:os";
import {join,resolve,basename} from "node:path";
import {createHash} from "node:crypto";
import {openSync,closeSync,readFileSync} from "node:fs";

const config=JSON.parse(await readFile(process.argv[2],"utf8")),output=resolve(process.argv[3]);
const files=config.mutate;
const localScript=name=>/^(crates\/vizzi\/web|experiments\/review-workflow)\/[^/]+\.(js|mjs)$/.test(name);
assert.ok(files.every(localScript));
const testFiles=config.offlineTestFiles ?? ["review-controller.test.js","review-windows.test.js","windows.test.js","updates.test.js"].map(name=>`crates/vizzi/web/${name}`);
assert.ok(testFiles.length>0 && testFiles.every(localScript));
const tests=testFiles.map(name=>basename(name));
const temporary=await mkdtemp(join(tmpdir(),"vizzi-offline-mutations-"));
const sha=value=>createHash("sha256").update(value).digest("hex");
const logger={debug(){},info(){},warn(){},trace(){},error(){},isDebugEnabled:()=>false};
const instrumenter=new Instrumenter(logger);
const report={schema:1,runner:"Serial Node commands using Stryker 10.1.1 instrumenter; no Stryker worker/log sockets",node:process.version,started:new Date().toISOString(),files,tests,sourceSha256:{},outcomes:[]};
const options={plugins:null,excludedMutations:[],ignorers:[]};
function execute(mutant) {
  for(const test of tests) {
    const env={...process.env};delete env.__STRYKER_ACTIVE_MUTANT__;
    if(mutant!=null)env.__STRYKER_ACTIVE_MUTANT__=mutant;
    const log=join(temporary,"test.log"),fd=openSync(log,"w");
    let result;try{result=spawnSync(process.execPath,[join(temporary,test)],{env,stdio:["ignore",fd,fd],timeout:config.timeoutMS});}finally{closeSync(fd);}
    const detail=readFileSync(log,"utf8").slice(-1800);
    if(result.error || result.status!==0)return {status:result.error?.code==="ETIMEDOUT"?"Timeout":result.error?"RunnerError":"Killed",test,detail:String(result.error ?? "")+detail};
  }
  return {status:"Survived"};
}
async function save() {report.counts=Object.fromEntries([...new Set(report.outcomes.map(o=>o.status))].map(status=>[status,report.outcomes.filter(o=>o.status===status).length]));await writeFile(output,JSON.stringify(report,null,2)+"\n");}
try {
  for(const name of await readdir("crates/vizzi/web"))if(name.endsWith(".js") || name==="package.json")await writeFile(join(temporary,name),await readFile(join("crates/vizzi/web",name)));
  const extras=[...new Set([...files,...testFiles].filter(name=>!name.startsWith("crates/vizzi/web/")))];
  for(const name of extras)await writeFile(join(temporary,basename(name)),await readFile(name));
  for(const name of [...files,...testFiles])report.sourceSha256[name]=sha(await readFile(name));
  const baseline=execute();assert.equal(baseline.status,"Survived",`unmodified baseline failed: ${JSON.stringify(baseline)}`);report.unmodifiedBaseline="passed";
  const control=await instrumenter.instrument([{name:"control.js",mutate:true,content:"export const matches = a => a === 1;"}],options);
  await writeFile(join(temporary,"control.js"),control.files[0].content);
  const probe=control.mutants.find(m=>m.replacement.includes("!=="));assert.ok(probe);
  const check=env=>spawnSync(process.execPath,["--input-type=module","-e",`import {matches} from ${JSON.stringify(join(temporary,"control.js"))};if(!matches(1))process.exit(17);`],{env:{...process.env,__STRYKER_ACTIVE_MUTANT__:env},stdio:"ignore"});
  assert.equal(check("").status,0);assert.equal(check(probe.id).status,17);report.activationControl="unmodified passes; known mutant fails";
  const result=await instrumenter.instrument(await Promise.all(files.map(async name=>({name,mutate:true,content:await readFile(name,"utf8")}))),options);
  for(const file of result.files)await writeFile(join(temporary,basename(file.name)),file.content);
  assert.equal(execute().status,"Survived","instrumented baseline failed");report.instrumentedBaseline="passed";report.generated=result.mutants.length;
  const selected=process.argv[4]?new Set(JSON.parse(await readFile(process.argv[4],"utf8"))):null;
  const mutants=result.mutants.filter(mutant=>!selected || selected.has(mutant.id));report.selected=mutants.length;
  for(const mutant of mutants) {
    const outcome=execute(mutant.id);report.outcomes.push({...mutant,...outcome});
    if(outcome.status==="Killed")delete report.outcomes.at(-1).detail;
    if(report.outcomes.length%20===0){await save();console.log(`${report.outcomes.length}/${report.selected}: ${JSON.stringify(report.counts)}`);}
  }
  report.finished=new Date().toISOString();await save();console.log(JSON.stringify(report.counts));
  if(report.counts.Survived || report.counts.RunnerError)process.exitCode=1;
} finally {await rm(temporary,{recursive:true,force:true});}
