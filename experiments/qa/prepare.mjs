import assert from 'node:assert/strict';
import {resolve} from 'node:path';
import {prepare} from './support.mjs';
const args=process.argv.slice(2),option=name=>{const i=args.indexOf(name);return i<0?undefined:args[i+1];};
assert.ok(args.includes('--output') && args.includes('--lpc-root'));
const result=await prepare(resolve(option('--output')),{vizzi:resolve(option('--vizzi-root')??'.'),'lpc-rs':resolve(option('--lpc-root'))});
console.log(JSON.stringify({sources:Object.fromEntries(Object.entries(result.sources).map(([k,v])=>[k,{revision:v.revision,stats:v.initial.stats,capture:v.initial.capture}])),questions:result.checks.length,audited:result.checks.every(c=>c.evidence_audited)}));
