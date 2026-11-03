#!/usr/bin/env python3
"""Measure source-history reclamation below the preceding cumulative append requirement."""
import argparse
import hashlib
import json
import pathlib
import shutil
import sys
import tempfile
import uuid

sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from experiments.qualification.native import execute, policy


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('output must be new')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    binary = pathlib.Path('target/release/examples/source_storage').resolve()
    previous = json.loads(pathlib.Path('experiments/verified-reuse/results/after.json').read_text())
    results = []
    for profile in json.loads(pathlib.Path('experiments/qualification/profiles.json').read_text()):
        name = profile['name']
        old = next(r for r in previous['results'] if r['kind'] == 'corpus' and r['profile']['name'] == name)
        quota = 8192 if name == 'linux' else 1024
        targets = old['configuration']['targets']
        with tempfile.TemporaryDirectory(prefix='vizzi-reclaim-corpus-') as temporary:
            root = pathlib.Path(temporary) / 'source'
            shutil.copytree(profile['source'], root, symlinks=True)
            owner = str(uuid.uuid4())
            (root / '.vizzi-source-owned').write_text(owner)
            hashes = {p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in targets}
            configuration = {'root': str(root), 'policy': policy(profile['source_arguments']),
                             'targets': targets, 'owner': owner, 'probe': str(uuid.uuid4()),
                             'mode': 'workspace', 'spool_mib': quota}
            stem = args.output.parent / f'{args.output.stem}-{name}'
            config = stem.with_suffix('.config.json')
            with config.open('x') as file:
                json.dump(configuration, file, indent=2)
            print(f'{name}: {quota} MiB backing, separate staging allowance, 12 GiB child AS limit', flush=True)
            run = execute([sys.executable, str(pathlib.Path('experiments/admission/limit.py').resolve()),
                           '12288', str(binary), str(config)], 1200, stem)
            report = run['events'][-1]['data'].pop('report')
            run['events'][-1]['data']['report_ref'] = 'report'
            assert report['restored_snapshot_matches'] and report['released_handles_dead'] and report['external_pin_bytes_match']
            assert report['original_source_manifest'] == old['report']['original_source_manifest']
            assert report['spool']['written_bytes'] == old['report']['spool']['written_bytes']
            assert report['spool']['written_bytes'] > report['spool']['capacity_bytes']
            for phase in report['phases']:
                stats = phase['spool']
                assert stats['file_bytes'] <= stats['capacity_bytes']
                assert stats['extent_bytes'] <= stats['capacity_bytes']
                assert stats['allocated_bytes'] + stats['free_bytes'] == stats['extent_bytes']
            assert all(report['spool'][key] == 0 for key in ['allocated_bytes', 'extent_bytes', 'file_bytes', 'pending_retirements', 'staging_bytes'])
            assert all(hashlib.sha256((root / p).read_bytes()).hexdigest() == h for p, h in hashes.items())
            result = {'profile': profile, 'configuration': configuration, 'original_target_sha256': hashes,
                      'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'run': run, 'report': report}
            results.append(result)
            with stem.with_suffix('.completed.json').open('x') as file:
                json.dump(result, file, indent=2)
            print(f'  checks pass; peak span {report["spool"]["peak_extent_bytes"] / 2**30:.3f} GiB; final file 0 bytes', flush=True)
    with args.output.open('x') as file:
        json.dump({'schema': 1, 'results': results, 'limits': [
            'Source-only history; no facts/parsers/watchers/UI. Single sequential condition per profile, not timing tails or randomised speedups.',
            'Maintenance before each phase inventory is recorded separately from the preceding capture/validation duration.',
            'File lengths and reserved address spans are not filesystem block allocation or process-memory caps.',
            'Interior gaps are reused, free tails truncated; fragmentation and external pins can still refuse allocations.',
            '12 GiB RLIMIT_AS constrains virtual address space, not RSS/physical RAM or the baseline laptop envelope.'
        ]}, file, indent=2)
        file.write('\n')


if __name__ == '__main__':
    main()
