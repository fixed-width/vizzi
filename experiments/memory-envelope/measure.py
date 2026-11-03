#!/usr/bin/env python3
"""Compare resident and spooled source with engine/browser memory accounting."""
import argparse
import hashlib
import json
import pathlib
import subprocess
import uuid


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cache', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    parser.add_argument('--observe-only', action='store_true')
    args = parser.parse_args()
    if args.output.exists():
        parser.error('output directory must be new')
    profiles = json.loads(pathlib.Path('experiments/qualification/profiles.json').read_text())
    for profile in profiles:
        if not (args.cache / profile['name']).is_dir():
            parser.error('prepare compatible fact caches before measurement')
    binary_paths = ['target/release/vizzi', 'target/release/vizzi-envelope']
    if not all(pathlib.Path(path).is_file() for path in binary_paths):
        parser.error('build release host and observer before measurement')
    args.output.mkdir(parents=True)
    hashes = {path: hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest() for path in binary_paths}
    results = []
    # Reverse policy order for the second corpus; this is not a randomised speedup trial.
    for name, backing in [('linux', 'resident'), ('linux', 'spooled'),
                          ('pytorch', 'spooled'), ('pytorch', 'resident')]:
        profile = next(profile.copy() for profile in profiles if profile['name'] == name)
        profile['cache'] = str((args.cache / name).resolve())
        profile['source_arguments'] = list(profile['source_arguments'])
        if backing == 'spooled':
            profile['source_arguments'] += ['--source-spool-mib', '8192' if name == 'linux' else '1024']
        stem = args.output.resolve() / f'{name}-{backing}'
        configuration = stem.with_suffix('.profile.json')
        configuration.write_text(json.dumps(profile, indent=2) + '\n')
        output = stem.with_suffix('.json')
        command = ['npm', 'test', '--', '--envelope', '--reference-only', '--during-recovery',
                   '--profile', str(configuration), '--output', str(output)]
        if args.observe_only:
            command += ['--observe-only']
        else:
            command = ['systemd-run', '--user', '--scope',
                       '--property=MemoryMax=12G', '--property=MemorySwapMax=0',
                       '--property=MemoryAccounting=yes',
                       '--unit=vizzi-envelope-' + str(uuid.uuid4()), *command]
        print(f'{name} {backing}: {"observe only" if args.observe_only else "12 GiB cgroup, swap disabled"}', flush=True)
        with stem.with_suffix('.log').open('x') as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=1200)
        browser = json.loads(output.read_text())
        memory = json.loads(pathlib.Path(str(output) + '.memory.json').read_text())
        assert memory['success'] and memory['cap_verified'] == (not args.observe_only)
        assert browser['options']['noBuild']
        assert len(browser['results']) == 1
        result = browser['results'][0]
        assert result['capturedFiles'] == (66066 if name == 'linux' else 10578)
        assert result['cleanSnapshotMatches'] and result['errors'] == []
        assert len(result['editLatency']['samples']) == 10
        completed = [sample['recoveryBeforeWrite']['completed'] for sample in result['editLatency']['samples']]
        assert all(sample['recoveryBeforeWrite']['running'] for sample in result['editLatency']['samples'])
        assert all(left < right for left, right in zip(completed, completed[1:]))
        results.append({'profile': name, 'backing': backing, 'command': command,
                        'browser': output.name, 'memory': output.name + '.memory.json',
                        'cap_verified': memory['cap_verified'], 'peak_pss_kib': memory['peak_pss_kib'],
                        'peak_rss_kib': memory['peak_rss_kib']})
        assert all(hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest() == digest for path, digest in hashes.items())
        print(f'  concurrent sampled PSS {memory["peak_pss_kib"]/1048576:.3f} GiB; browser/clean-capture checks pass', flush=True)
    with (args.output / 'matrix.json').open('x') as file:
        json.dump({'schema': 1, 'mode': 'observe_only' if args.observe_only else 'cgroup',
                   'binary_sha256': hashes, 'results': results}, file, indent=2)
        file.write('\n')


if __name__ == '__main__':
    main()
