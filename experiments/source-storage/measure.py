#!/usr/bin/env python3
"""Measure verified capture and source ownership across publication and external pins."""
import argparse
import hashlib
import json
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
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profiles',type=pathlib.Path,default=pathlib.Path('experiments/qualification/profiles.json'))
    parser.add_argument('--profile',choices=['linux','pytorch','both'],default='both')
    parser.add_argument('--output',type=pathlib.Path,required=True)
    parser.add_argument('--address-space-mib',type=int,default=12288)
    args=parser.parse_args()
    if args.output.exists() or args.address_space_mib<1024:parser.error('output must be new; address-space limit must be positive and at least 1024 MiB')
    args.output.parent.mkdir(parents=True,exist_ok=True)
    binary=pathlib.Path('target/release/examples/source_storage').resolve()
    wrapper=pathlib.Path('experiments/admission/limit.py').resolve()
    results=[]
    for profile in json.loads(args.profiles.read_text()):
        name=profile['name']
        if args.profile not in ['both',name]:continue
        reference=json.loads(pathlib.Path(f'experiments/recovery/results/browser-{name}.json').read_text())['results'][0]
        targets=[f['path'] for f in reference['heavyFiles']]
        manifests=set()
        for mode in ['headers','workspace']:
            print(f'Copying owned {name} source for {mode} churn',flush=True)
            with tempfile.TemporaryDirectory(prefix='vizzi-source-storage-owned-') as temporary:
                root=pathlib.Path(temporary)/'source';shutil.copytree(profile['source'],root,symlinks=True)
                owner=str(uuid.uuid4());(root/'.vizzi-source-owned').write_text(owner)
                hashes={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in targets}
                config={'root':str(root),'policy':policy(profile['source_arguments']),'targets':targets,'owner':owner,'probe':str(uuid.uuid4()),'mode':mode}
                log=args.output.parent/f'{args.output.stem}-{name}-{mode}'
                config_path=log.with_suffix('.config.json')
                with config_path.open('x') as f:json.dump(config,f,indent=2)
                command=[sys.executable,str(wrapper),str(args.address_space_mib),str(binary),str(config_path)]
                print(f'{name} {mode}; four generations; child address-space limit {args.address_space_mib} MiB',flush=True)
                run=execute(command,900,log)
                report=run['events'][-1]['data'].pop('report');run['events'][-1]['data']['report_ref']='report'
                assert report['expired_from_publication'] and report['external_pin_bytes_match'] and report['restored_snapshot_matches'] and report['released_handles_dead']
                assert all(hashlib.sha256((root/p).read_bytes()).hexdigest()==h for p,h in hashes.items())
                manifests.add(report['original_source_manifest'])
                result={'profile':profile,'configuration':config,'original_target_sha256':hashes,'command':command,'report':report,'run':run}
                results.append(result)
                with log.with_suffix('.completed.json').open('x') as f:json.dump(result,f,indent=2)
                phase=report['phases'][5]
                print(f'  buffers: publication={phase["publication"]["buffer_bytes"]/1073741824:.3f} GiB; with external pin={phase["including_external"]["buffer_bytes"]/1073741824:.3f} GiB; sampled RSS peak={run["peaks_kib"].get("VmRSS",0)/1048576:.3f} GiB',flush=True)
        assert len(manifests)==1,'initial source manifest differs across modes'
    report={'schema':1,'base_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),
        'platform':platform.platform(),'address_space_limit_mib':args.address_space_mib,'results':results,
        'limits':['Source-only storage snapshots intentionally omit facts, parsers, caches, host recovery, watchers, HTTP and browser timing.',
                  'Warm OS caches; owned-copy construction and writes are excluded from capture duration. Capture timing includes hashing, indexing, validation and snapshot creation.',
                  'Buffer accounting uses actual byte/string and line-index capacities, excludes structs/documents/allocator overhead and deduplicates Arc allocation identity.',
                  'Publication current/history and an external query handle are separate ownership sets. Releasing references does not require RSS to return immediately.',
                  'Sampled process peaks include probe metadata, integrity verification, restoration and deallocation. A single run does not establish tails or a controlled allocator study.',
                  'Child-only RLIMIT_AS limits virtual address space, not RSS or physical RAM. No 16 GB laptop or production memory-cap qualification.']}
    with args.output.open('x') as f:json.dump(report,f,indent=2);f.write('\n')
    print(args.output,flush=True)


if __name__=='__main__':main()
