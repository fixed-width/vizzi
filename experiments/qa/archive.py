"""Archive a completed local QA pilot; grading remains explicit source-aware review."""
import argparse,collections,gzip,hashlib,json,pathlib,shutil
p=argparse.ArgumentParser();p.add_argument('--input',required=True);p.add_argument('--memory',required=True);p.add_argument('--output',required=True);args=p.parse_args()
source=pathlib.Path(args.input);out=pathlib.Path(args.output);out.mkdir(parents=True)
read=lambda name:json.loads((source/name).read_text())
manifest=read('manifest.json');assert not manifest['incomplete'] and manifest['completed']==96
answers=[json.loads(x) for x in (source/'answers.jsonl').read_text().splitlines()]
assert len(answers)==96
library=json.loads(pathlib.Path('experiments/qa/library/cases.json').read_text());cases={c['id']:c for c in library['cases']}
allgrades=[]
for name in ['grades-08.json','grades-2b.json']:
 grades=read(name)['grades'];assert len(grades)==32;allgrades.extend(grades)
ids={r['id']:r for r in answers}
for grade in allgrades:
 row=ids[grade['response_id']];assert row['condition']=='retrieved';assert len(grade['required_facts_supported'])==len(cases[row['case']]['required_facts']);assert grade['provenance_valid']==row['provenanceValid']
assert len({g['response_id'] for g in allgrades})==64
memory=json.loads(pathlib.Path(args.memory).read_text());assert memory['success'] and memory['child_exit_code']==0
summaries=[]
for definition in manifest['modelDefinitions']:
 model=definition['id'];runtime=read(model+'.runtime.json');pid=runtime['pid'];samples=[s for s in memory['samples'] if any(row['process']['pid']==pid for row in s['processes'])];assert samples
 grades=[g for g in allgrades if g['model']==model];group=[r for r in answers if r['model']==model and r['condition']=='retrieved'];assert len(group)==32
 expected={g['response_id']:cases[g['case']]['expected_status'] for g in grades}
 answerable=[g for g in grades if expected[g['response_id']]=='answered'];absent=[g for g in grades if expected[g['response_id']]=='insufficient_evidence']
 times=sorted(r['totalMs'] for r in group);first=sorted(r['firstAnswerMs'] for r in group)
 summaries.append({'model':model,'responses':32,'answerableCases':len(answerable),'unanswerableCases':len(absent),'reviewer':'Codex, reference-aware seed author; not independent','taskResults':dict(collections.Counter(g['task_result'] for g in grades)),'answerableTaskSuccess':sum(g['task_result']=='correct' for g in answerable),'unanswerableTaskSuccess':sum(g['task_result']=='correct' for g in absent),'supportedFacts':sum(sum(g['required_facts_supported']) for g in grades),'requiredFacts':sum(len(g['required_facts_supported']) for g in grades),'strictSourceChecks':sum(r['provenanceValid'] for r in group),'medianFirstOutputMs':first[16],'medianCompleteMs':times[16],'p95CompleteMs':times[30],'peakCombinedPssKiBWhileModelLive':max(s['usage']['pss_kib'] for s in samples),'peakCombinedRssKiBWhileModelLive':max(s['usage']['rss_kib'] for s in samples),'modelStartupWarmFileCacheMs':runtime['startupMs']})
report={'schema':1,'summaries':summaries,'globalMemory':{k:v for k,v in memory.items() if k not in ['samples','command']},'latencyDefinition':'Upper median and nearest-rank p95; first non-reasoning JSON output includes the preamble. Complete latency is inference HTTP delivery, with retrieval recorded separately.','scoringLimits':['Seed-author agent review, not independent human evaluation.','Strict task score includes response status/source protocol; supported-fact counts are reported separately.','Oracle-source diagnostics are not a proven upper bound; missing linkage and seed ambiguity remain possible.','Both candidates use deterministic greedy decoding; this is not the authors recommended stochastic sampling or a best-capability claim.','Memory is sampled process-tree PSS/RSS with cap_verified=false; physical 16 GB laptop and kernel/PyTorch scale remain unqualified.']}
(out/'assessment.json').write_text(json.dumps(report,indent=2)+'\n')
(out/'grades.json').write_text(json.dumps({'schema':1,'grades':allgrades},indent=2)+'\n')
for name in ['manifest.json','summary.json','citation-diagnostics.json','review.md']+[x['id']+'.runtime.json' for x in manifest['modelDefinitions']]:shutil.copyfile(source/name,out/name)
checks={}
for original,name in [(source/'answers.jsonl','answers.jsonl.gz'),(pathlib.Path(args.memory),'memory.json.gz')]+[(source/(m['id']+'.runtime.log'),m['id']+'.runtime.log.gz') for m in manifest['modelDefinitions']]:
 data=original.read_bytes();checks[name]={'sha256_uncompressed':hashlib.sha256(data).hexdigest(),'uncompressed_bytes':len(data)}
 with gzip.GzipFile(filename=str(out/name),mode='wb',mtime=0) as writer:writer.write(data)
(out/'checksums.json').write_text(json.dumps(checks,indent=2)+'\n')
print(json.dumps(summaries))
