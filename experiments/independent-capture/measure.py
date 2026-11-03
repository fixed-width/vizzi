#!/usr/bin/env python3
"""Compare foreground capture during cleanup with one-file and two-file allocation."""
import argparse
import json
import pathlib
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=pathlib.Path, required=True)
    parser.add_argument('--after', type=pathlib.Path, default=pathlib.Path('target/release/examples/foreground_maintenance'))
    parser.add_argument('--cache', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('output must be new')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    observations = {}
    for label, binary in [('before', args.before), ('after', args.after)]:
        target = args.output.parent / f'{args.output.stem}-{label}.json'
        print(f'Implementation: {label}', flush=True)
        subprocess.run([sys.executable, 'experiments/foreground-maintenance/measure.py',
                        '--binary', str(binary.resolve()), '--cache', str(args.cache.resolve()),
                        '--output', str(target)], check=True)
        observations[label] = json.loads(target.read_text())
    for before, after in zip(observations['before']['results'], observations['after']['results'], strict=True):
        assert before['profile']['revision'] == after['profile']['revision']
        assert before['original_target_sha256'] == after['original_target_sha256']
        assert before['report']['files'] == after['report']['files']
        assert before['report']['spool']['written_bytes'] == after['report']['spool']['written_bytes']
        assert before['binary_sha256'] != after['binary_sha256']
    with args.output.open('x') as file:
        json.dump({'schema': 1, 'observations': observations, 'limits': [
            'Identical updated foreground harness on preceding/current backends, separate build outputs and verified binary hashes.',
            'Old first, new second; three alternating condition pairs per repository/implementation, finite warm native observations.',
            'Controlled generated tails and explicit hints isolate cleanup; source/fact/pin/restoration checks remain mandatory.',
            'Aggregate backing quotas include cleaning space until IO succeeds; independent staging bound is additional.',
            'No browser/watcher/recovery-contention or baseline-laptop latency qualification; child AS limit is not physical RAM.'
        ]}, file, indent=2)
        file.write('\n')


if __name__ == '__main__':
    main()
