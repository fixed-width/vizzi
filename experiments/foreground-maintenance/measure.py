#!/usr/bin/env python3
"""Measure real-host foreground publication after cleanup and during observed truncation."""
import argparse
import hashlib
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import uuid

sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from experiments.qualification.native import execute, policy


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cache', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    parser.add_argument('--binary', type=pathlib.Path, default=pathlib.Path('target/release/examples/foreground_maintenance'))
    parser.add_argument('--prepare-only', action='store_true')
    parser.add_argument('--watch', action='store_true')
    parser.add_argument('--recovery-ms', type=int, default=3600000)
    parser.add_argument('--recovery-samples', type=int, default=0)
    args = parser.parse_args()
    combined = args.watch or args.recovery_ms != 3600000 or args.recovery_samples != 0
    if args.recovery_ms <= 0 or not 0 <= args.recovery_samples <= 8:
        parser.error('recovery interval must be positive; samples must be between zero and eight')
    if args.output.exists():
        parser.error('output must be new')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    host = pathlib.Path('target/release/vizzi').resolve()
    binary = args.binary.resolve()
    profiles = json.loads(pathlib.Path('experiments/qualification/profiles.json').read_text())
    if args.prepare_only:
        prepared = []
        for profile in profiles:
            cache = args.cache.resolve() / profile['name']
            print(f'Preparing {profile["name"]} compatible facts outside timing', flush=True)
            process = subprocess.run([str(host), '--root', profile['source'], '--cache', str(cache),
                                      '--once', *profile['source_arguments']], capture_output=True, text=True, check=True)
            prepared.append({'profile': profile['name'], 'status': json.loads(process.stdout)})
        with args.output.open('x') as file:
            json.dump({'host_sha256': hashlib.sha256(host.read_bytes()).hexdigest(), 'preparation': prepared}, file, indent=2)
        return
    results = []
    for profile in profiles:
        name = profile['name']
        target = 'kernel/locking/mutex.c' if name == 'linux' else 'c10/core/Device.cpp'
        with tempfile.TemporaryDirectory(prefix='vizzi-foreground-owned-') as temporary:
            root = pathlib.Path(temporary) / 'source'
            shutil.copytree(profile['source'], root, symlinks=True)
            owner = str(uuid.uuid4())
            (root / '.vizzi-foreground-owned').write_text(owner)
            original = hashlib.sha256((root / target).read_bytes()).hexdigest()
            config = {'root': str(root), 'cache': str(args.cache.resolve() / name),
                      'policy': policy(profile['source_arguments']), 'target': target,
                      'owner': owner, 'probe': str(uuid.uuid4()), 'pairs': 3,
                      'spool_mib': 8192 if name == 'linux' else 1024,
                      'tail_mib': 4096 if name == 'linux' else 768,
                      'watch': args.watch, 'recovery_ms': args.recovery_ms,
                      'recovery_samples': args.recovery_samples}
            stem = args.output.parent / f'{args.output.stem}-{name}'
            path = stem.with_suffix('.config.json')
            with path.open('x') as file:
                json.dump(config, file, indent=2)
            print(f'{name}: three alternating pairs, {config["tail_mib"]} MiB generated tail; 12 GiB AS limit', flush=True)
            run = execute([sys.executable, str(pathlib.Path('experiments/admission/limit.py').resolve()),
                           '12288', str(binary), str(path)], 1200, stem)
            report = run['events'][-1]['data'].pop('report')
            run['events'][-1]['data']['report_ref'] = 'report'
            assert report['restored_matches'] and report['final_clean_matches']
            assert report['spool']['file_bytes'] == report['spool']['allocated_bytes'] == 0
            assert hashlib.sha256((root / target).read_bytes()).hexdigest() == original
            if combined:
                assert report['watching'] == args.watch and report['explicit_hints'] == (not args.watch)
                assert report['recovery_ms'] == args.recovery_ms
                assert len(report['cases']) == 6 + args.recovery_samples
            for case in report['cases']:
                assert all(case[key] for key in ['source_matches', 'fresh_facts_match', 'expected_declaration', 'old_pin_matches'])
                assert case['write_start_ms'] <= case['published_ms']
                if combined:
                    assert case['explicit_hint'] == (not args.watch)
                if case['condition'] == 'recovery_observed':
                    assert case['status_before']['recovery']['running']
                    assert case['write_start_ms'] <= case['write_finished_ms'] <= case['published_ms']
                    continue
                assert case['write_overlaps_cleanup'] == (case['cleanup_start_ms'] <= case['write_start_ms'] < case['cleanup_end_ms'])
                if case['condition'] == 'after_cleanup':
                    assert not case['write_overlaps_cleanup']
                if combined:
                    assert case['capture_marks'] == [] and case['capture_overlaps_cleanup'] is None
                else:
                    assert case['measurements']['read_files'] == 1 and len(case['capture_marks']) == 1
                    mark = case['capture_marks'][0]
                    assert case['write_start_ms'] <= case['hint_ms'] <= mark['entered_ms'] <= mark['ready_ms'] <= case['published_ms']
                    assert case['capture_overlaps_cleanup'] == (case['cleanup_start_ms'] <= mark['entered_ms'] < case['cleanup_end_ms'])
                    if case['condition'] == 'after_cleanup':
                        assert not case['capture_overlaps_cleanup']
            result = {'profile': profile, 'configuration': config, 'original_target_sha256': original,
                      'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'run': run, 'report': report}
            results.append(result)
            with stem.with_suffix('.completed.json').open('x') as file:
                json.dump(result, file, indent=2)
            print(f'  source/fact/pin/restoration checks pass; actual write overlaps: {sum(c.get("write_overlaps_cleanup", False) for c in report["cases"])}', flush=True)
    with args.output.open('x') as file:
        json.dump({'schema': 1, 'results': results, 'limits': [
            ('Watch/recovery options recorded in configuration; exact notification attribution and recovery overlap are not isolated.' if combined else
             'Explicit host path hints, watcher disabled; initial recovery completes before an hour-long recovery interval isolates cleanup.'),
            'Generated streamed tail is controlled allocation pressure outside project source; it is not representative repository code.',
            'Three samples per condition/profile; finite observations, not browser latency tails or randomised speedups.',
            ('Unlabelled capture tracing disabled; latest operation measurements may describe recovery rather than the foreground edit.' if combined else
             'Writer plus initial maintenance gate is traced; engine capture/total durations and observed publication include distinct work.'),
            'Compatible cache preparation, pressure setup and independent verification are outside write-to-publication timing, but affect process peaks.',
            'Child 12 GiB virtual-address-space limit is not RSS/physical-RAM enforcement or baseline-laptop qualification.'
        ]}, file, indent=2)
        file.write('\n')


if __name__ == '__main__':
    main()
