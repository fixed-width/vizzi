import assert from 'node:assert/strict';
import {mkdir, readFile, writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {cpus, platform, arch} from 'node:os';
import {join, resolve} from 'node:path';
import {startHost} from '../browser/support.mjs';
const args=process.argv.slice(2), option=name=>args[args.indexOf(name)+1];
assert.ok(args.includes('--source') && args.includes('--output'));
const output=resolve(option('--output'));await mkdir(output);
const host=await startHost({root:resolve(option('--source')),cache:join(output,'analysis')},1);
try {
  await fetch(host.url+'/api/workspace/start',{method:'POST',headers:{'Content-Type':'application/json'},body:'{}'});
  for(let i=0;;i++){
    const status=await (await fetch(host.url+'/api/status')).json();
    if(status.snapshot)break;
    assert.ok(i<600 && !status.error,'Index activation failed or timed out');
    await new Promise(done=>setTimeout(done,100));
  }
  const cases=[];
  for(const [name,query] of [['root',{}],['compiler area',{path:'src/compiler'}],['name search',{search:'reader'}]]){
    const samples=[],sizes=[];let page;
    for(let i=0;i<35;i++){
      const started=performance.now(),response=await fetch(host.url+'/api/discovery?'+new URLSearchParams(query));
      const body=await response.text(),elapsed=performance.now()-started;
      assert.equal(response.status,200);page=JSON.parse(body);assert.ok(page.entries.length<=24);
      if(i>=5){samples.push(elapsed);sizes.push(Buffer.byteLength(body));}
    }
    const sorted=[...samples].sort((a,b)=>a-b);
    cases.push({name,query,samplesMs:samples,responseBytes:Math.max(...sizes),entries:page.entries.length,matching:page.matching,scopeFiles:page.scope_files,totalFiles:page.total_files,medianMs:sorted[15],p95Ms:sorted[28],maxMs:sorted[29]});
  }
  const report={schema:1,environment:{os:platform(),arch:arch(),cpu:cpus()[0].model},workers:1,binarySha256:createHash('sha256').update(await readFile('target/release/vizzi')).digest('hex'),cases,limitations:[
    'Optimized native host; tests and mutations stopped before measurement.',
    'Thirty warm localhost request/full-body-delivery observations after five warmups per case. JSON parsing occurs after timing.',
    'Pinned lpc-rs source scope only; no large-repository qualification, browser input latency or GPU measurement.',
  ]};
  await writeFile(join(output,'report.json'),JSON.stringify(report,null,2)+'\n');
  console.log(JSON.stringify(cases.map(({name,responseBytes,entries,matching,medianMs,p95Ms})=>({name,responseBytes,entries,matching,medianMs,p95Ms}))));
}finally{await host.close();}
