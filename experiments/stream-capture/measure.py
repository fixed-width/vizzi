#!/usr/bin/env python3
"""Measure bounded large-file and representative-repository streamed source capture."""
import argparse
import hashlib
import json
import os
import pathlib
import platform
import shutil
import subprocess
import sys
import tempfile
import uuid
sys.dont_write_bytecode=True
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[2]))
from experiments.qualification.native import execute,policy

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=pathlib.Path,required=True)
    parser.add_argument('--fixture-mib',type=int,default=512);args=parser.parse_args()
    if args.output.exists() or not 1<=args.fixture_mib<=1024:parser.error('output must be new; fixture must be 1..1024 MiB')
    args.output.parent.mkdir(parents=True,exist_ok=True)
    fixture_binary=pathlib.Path('target/release/examples/stream_capture').resolve();storage_binary=pathlib.Path('target/release/examples/source_storage').resolve()
    wrapper=pathlib.Path('experiments/admission/limit.py').resolve();results=[]
    for mode in ['whole','stream']:
        with tempfile.TemporaryDirectory(prefix='vizzi-stream-fixture-') as temporary:
            root=pathlib.Path(temporary)/'source';root.mkdir();owner=str(uuid.uuid4());(root/'.vizzi-stream-owned').write_text(owner)
            with (root/'large.rs').open('wb') as f:
                block=b' '*65536
                for _ in range(args.fixture_mib*16):f.write(block)
            config={'root':str(root),'owner':owner,'mode':mode};stem=args.output.parent/f'{args.output.stem}-fixture-{mode}'
            path=stem.with_suffix('.config.json')
            with path.open('x') as f:json.dump(config,f,indent=2)
            print(f'Generated {args.fixture_mib} MiB fixture, {mode} capture; child AS limit 2048 MiB',flush=True)
            run=execute([sys.executable,str(wrapper),'2048',str(fixture_binary),str(path)],600,stem)
            report=run['events'][-1]['data'].pop('report');run['events'][-1]['data']['report_ref']='report'
            assert report['bounded_bytes_match'] and report['unchanged_shared']
            assert report['final_spool']['written_bytes']==args.fixture_mib*1048576
            result={'kind':'fixture','mode':mode,'configuration':config,'binary_sha256':hashlib.sha256(fixture_binary.read_bytes()).hexdigest(),'run':run,'report':report};results.append(result)
            with stem.with_suffix('.completed.json').open('x') as f:json.dump(result,f,indent=2)
            print(f'  sampled process RSS peak {run["peaks_kib"].get("VmRSS",0)/1048576:.3f} GiB',flush=True)
    for profile in json.loads(pathlib.Path('experiments/qualification/profiles.json').read_text()):
        name=profile['name'];reference=json.loads(pathlib.Path(f'experiments/recovery/results/browser-{name}.json').read_text())['results'][0]
        targets=[x['path'] for x in reference['heavyFiles']]
        with tempfile.TemporaryDirectory(prefix='vizzi-stream-corpus-') as temporary:
            root=pathlib.Path(temporary)/'source';shutil.copytree(profile['source'],root,symlinks=True);owner=str(uuid.uuid4());(root/'.vizzi-source-owned').write_text(owner)
            original={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in targets}
            config={'root':str(root),'policy':policy(profile['source_arguments']),'targets':targets,'owner':owner,'probe':str(uuid.uuid4()),'mode':'workspace','spool_mib':16384}
            stem=args.output.parent/f'{args.output.stem}-{name}';path=stem.with_suffix('.config.json')
            with path.open('x') as f:json.dump(config,f,indent=2)
            print(f'{name} streamed whole-scope history, child AS limit 12288 MiB',flush=True)
            run=execute([sys.executable,str(wrapper),'12288',str(storage_binary),str(path)],1200,stem)
            report=run['events'][-1]['data'].pop('report');run['events'][-1]['data']['report_ref']='report'
            assert report['restored_snapshot_matches'] and report['released_handles_dead'] and report['external_pin_bytes_match']
            assert report['spool']['staging_bytes']==0 and report['spool']['max_read_request']<=65536
            assert all(hashlib.sha256((root/p).read_bytes()).hexdigest()==h for p,h in original.items())
            result={'kind':'corpus','profile':profile,'configuration':config,'original_target_sha256':original,'binary_sha256':hashlib.sha256(storage_binary.read_bytes()).hexdigest(),'run':run,'report':report};results.append(result)
            with stem.with_suffix('.completed.json').open('x') as f:json.dump(result,f,indent=2)
            print(f'  sampled process RSS peak {run["peaks_kib"].get("VmRSS",0)/1048576:.3f} GiB; source/pin/restoration checks pass',flush=True)
    output={'schema':1,'base_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
      'platform':platform.platform(),'cpu_affinity':sorted(os.sched_getaffinity(0)),
      'meminfo':pathlib.Path('/proc/meminfo').read_text(),'results':results,
      'limits':['Generated fixture compares current-code whole-vector capture plus persist with streamed staging capture in separate sequential children. Verification is bounded; no parser/cache/UI work.',
                'Corpus source-only storage history omits facts/parsers and is comparable to the preceding backed-source workload, not randomized paired speedups.',
                'Child RLIMIT_AS constrains virtual address space, not RSS or physical RAM; no baseline-laptop qualification.',
                'Staging is separately bounded by configured committed capacity; combined temporary/committed file lengths can reach twice that value.',
                'Metadata, allocator retention and process RSS are distinct from bounded transient capture bytes. Corpus validation and probe orchestration contribute to peaks and elapsed phases.']}
    with args.output.open('x') as f:json.dump(output,f,indent=2);f.write('\n')
    print(args.output,flush=True)
if __name__=='__main__':main()
