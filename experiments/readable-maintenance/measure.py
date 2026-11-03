#!/usr/bin/env python3
"""Compare pinned source reads during maintenance with preceding and current spool locks."""
import argparse
import json
import pathlib
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=pathlib.Path, required=True)
    parser.add_argument('--after', type=pathlib.Path, default=pathlib.Path('target/release/examples/source_storage'))
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('output must be new')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    observations = {}
    for label, binary in [('before', args.before), ('after', args.after)]:
        target = args.output.parent / f'{args.output.stem}-{label}.json'
        print(f'Implementation: {label}', flush=True)
        subprocess.run([sys.executable, 'experiments/reclamation/measure.py', '--binary', str(binary.resolve()),
                        '--maintenance-reads', '--output', str(target)], check=True)
        observations[label] = json.loads(target.read_text())
    for before, after in zip(observations['before']['results'], observations['after']['results'], strict=True):
        assert before['profile']['revision'] == after['profile']['revision']
        assert before['report']['original_source_manifest'] == after['report']['original_source_manifest']
        assert before['report']['spool']['written_bytes'] == after['report']['spool']['written_bytes']
    with args.output.open('x') as file:
        json.dump({'schema': 1, 'observations': observations, 'limits': [
            'Same read-sampling harness on both spool implementations; old first, new second, one child per profile/implementation.',
            'A separate 64 KiB prefix pin stays alive while all corpus snapshots retire. It is released afterward; final backing length is zero.',
            'Fresh verified 64 KiB backing reads sample about every 2 ms, at most 4096 per phase; ready handshake precedes maintenance.',
            'Maintenance and read intervals share a monotonic clock. Scheduler/filesystem delays remain; finite observations do not prove latency tails or hard deadlines.',
            'Source-only histories omit parsing, facts, watcher scheduling, HTTP and UI. Child AS limits are not RSS/physical-RAM limits.'
        ]}, file, indent=2)
        file.write('\n')


if __name__ == '__main__':
    main()
