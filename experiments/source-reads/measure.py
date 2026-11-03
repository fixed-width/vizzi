#!/usr/bin/env python3
"""Compare resident and private-backed source history, then verify admitted workflows."""
import argparse
import hashlib
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import uuid
sys.dont_write_bytecode=True
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[2]))
from experiments.qualification.native import execute,policy

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=pathlib.Path,required=True)
    parser.add_argument('--cache',type=pathlib.Path,required=True)
    parser.add_argument('--profiles',type=pathlib.Path,default=pathlib.Path('experiments/qualification/profiles.json'))
    args=parser.parse_args()
    if args.output.exists():parser.error('output must be new')
    args.output.parent.mkdir(parents=True,exist_ok=True)
    storage=pathlib.Path('target/release/examples/source_storage').resolve();admit=pathlib.Path('target/release/examples/admit').resolve();host=pathlib.Path('target/release/vizzi').resolve()
    wrapper=pathlib.Path('experiments/admission/limit.py').resolve();results=[];preparation=[]
    for profile in json.loads(args.profiles.read_text()):
        name=profile['name'];reference=json.loads(pathlib.Path(f'experiments/recovery/results/browser-{name}.json').read_text())['results'][0]
        large=[f['path'] for f in reference['heavyFiles']]
        for workflow in ['storage','admission']:
            if workflow=='admission':
                cache=args.cache.resolve()/name
                print(f'Preparing {name} current-context facts before admitted workflow (untimed)',flush=True)
                proc=subprocess.run([str(host),'--root',profile['source'],'--cache',str(cache),'--once',*profile['source_arguments']],capture_output=True,text=True,check=True)
                preparation.append({'profile':name,'status':json.loads(proc.stdout)})
            for backing in ([0,16384] if workflow=='storage' else [16384]):
                with tempfile.TemporaryDirectory(prefix='vizzi-source-reads-owned-') as temporary:
                    root=pathlib.Path(temporary)/'source';shutil.copytree(profile['source'],root,symlinks=True)
                    owner=str(uuid.uuid4());targets=large if workflow=='storage' else [('kernel/locking/mutex.c' if name=='linux' else 'c10/core/Device.cpp')]+large
                    original={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in targets}
                    marker='.vizzi-source-owned' if workflow=='storage' else '.vizzi-admission-owned';(root/marker).write_text(owner)
                    config={'root':str(root),'policy':policy(profile['source_arguments']),'targets':targets,'owner':owner,'probe':str(uuid.uuid4()),'spool_mib':backing}
                    binary=storage if workflow=='storage' else admit
                    if workflow=='storage':config['mode']='workspace'
                    else:config.update({'cache':str(cache),'input_mib':32 if name=='linux' else 12,'recovery_ms':2000})
                    stem=args.output.parent/f'{args.output.stem}-{name}-{workflow}-{backing}'
                    config_path=stem.with_suffix('.config.json')
                    with config_path.open('x') as f:json.dump(config,f,indent=2)
                    command=[sys.executable,str(wrapper),'12288' if workflow=='storage' else '8192',str(binary),str(config_path)]
                    print(f'{name} {workflow} spool={backing} MiB',flush=True)
                    run=execute(command,1200,stem);report=run['events'][-1]['data'].pop('report');run['events'][-1]['data']['report_ref']='report'
                    if workflow=='storage':assert report['external_pin_bytes_match'] and report['restored_snapshot_matches'] and report['released_handles_dead']
                    else:assert report['final_clean_matches'] and report['old_pin_source_matches'] and all(c['all_admissible_completed'] for c in report['cases'])
                    assert all(hashlib.sha256((root/p).read_bytes()).hexdigest()==h for p,h in original.items())
                    entry={'profile':profile,'workflow':workflow,'configuration':config,'original_target_sha256':original,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'run':run,'report':report}
                    results.append(entry)
                    with stem.with_suffix('.completed.json').open('x') as f:json.dump(entry,f,indent=2)
                    print(f'  sampled peak RSS={run["peaks_kib"].get("VmRSS",0)/1048576:.3f} GiB; verification passed',flush=True)
    output={'schema':1,'base_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'preparation':preparation,'results':results,
      'limits':['Storage runs compare resident/backed current-code source-only workspace churn, omitting facts/parsers/UI; address-space limit 12 GiB.',
                'Admission workflows use backed source with four workers, input quotas and periodic recovery; address-space limit 8 GiB.',
                'Private spool capacity is cumulative per session, not live disk usage, residency, capture streaming or a process-memory cap.',
                'No analysed project tooling. Copies/writes/preparation are excluded from capture/update timing. Peaks include independent verification/probe input construction.',
                'Single sequential process per condition with unique prefixes; not randomized speedup, latency tails or baseline-laptop qualification.']}
    with args.output.open('x') as f:json.dump(output,f,indent=2);f.write('\n')
    print(args.output,flush=True)
if __name__=='__main__':main()
