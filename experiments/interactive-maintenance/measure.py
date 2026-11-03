#!/usr/bin/env python3
"""Qualify native watching, periodic recovery and scheduled cleanup together."""
import argparse
import json
import pathlib
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cache', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('output must be new')
    raw = args.output.parent / f'{args.output.stem}-raw.json'
    subprocess.run([sys.executable, 'experiments/foreground-maintenance/measure.py',
                    '--binary', str(pathlib.Path('target/release/examples/idle_maintenance').resolve()),
                    '--watch', '--recovery-ms', '2000', '--recovery-samples', '3',
                    '--cache', str(args.cache.resolve()), '--output', str(raw)], check=True)
    data = json.loads(raw.read_text())
    for result in data['results']:
        report = result['report']
        assert report['watching'] and not report['explicit_hints'] and report['recovery_ms'] == 2000
        assert report['maintenance']['failed'] == 0
        assert report['maintenance']['attempts'] >= 6
        assert len(report['cases']) == 9
        assert sum(c['condition'] == 'recovery_observed' for c in report['cases']) == 3
        for case in report['cases']:
            assert not case['explicit_hint']
            assert case['maintenance_observed']['failed'] == 0
            for key in ['status_before', 'status_after']:
                assert case[key]['observation'] == 'native directory watches + periodic reconciliation'
            if case['condition'] == 'recovery_observed':
                assert case['status_before']['recovery']['running'] and not case['status_before']['pending']
            else:
                assert case['cleanup_start_ms'] >= case['retired_ms'] and case['admission_ms'] >= 0
                assert case['published_during_cleanup'] == (case['published_ms'] < case['cleanup_end_ms'])
    data['limits'] += [
        'Actual idle scheduler admits cleanup and waits for recovery before admission; no three-way overlap is forced.',
        'Only generated-tail cuts at least as large as the fixture signal cleanup entry; no artificial truncate delay.',
        'Recovery-observed cases record running status before writing, not an instrumented capture or parse interval.',
        'Native watcher callback release may outlast coordinator join; qualification waits for release before final storage checks.',
        'Quiet time can elapse during pressure setup; unblocked shutdown observations do not establish a deadline.'
    ]
    with args.output.open('x') as file:
        json.dump(data, file, indent=2)
        file.write('\n')


if __name__ == '__main__':
    main()
