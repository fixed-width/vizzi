#!/usr/bin/env python3
"""Run real-host foreground measurements with automatically admitted cleanup."""
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
                    '--cache', str(args.cache.resolve()), '--output', str(raw)], check=True)
    data = json.loads(raw.read_text())
    for result in data['results']:
        report = result['report']
        assert report['maintenance']['failed'] == 0
        assert report['maintenance']['attempts'] >= 6
        for case in report['cases']:
            assert case['maintenance_observed']['failed'] == 0
            assert case['cleanup_start_ms'] >= case['retired_ms']
            assert case['admission_ms'] >= 0
            assert case['published_during_cleanup'] == (case['published_ms'] < case['cleanup_end_ms'])
    data['limits'] += [
        'Actual host idle scheduler admits all cleanup; an observed provider delegates the same metadata hint and physical operation.',
        'Only generated-tail cuts at least as large as the fixture signal overlap; unrelated small maintenance is not mistaken for the target cut.',
        'Quiet interval may already have elapsed during pressure setup; retirement-to-entry includes up to the host poll cadence.',
        'Shutdown observations with no blocked syscall do not establish a hard shutdown deadline.'
    ]
    with args.output.open('x') as file:
        json.dump(data, file, indent=2)
        file.write('\n')


if __name__ == '__main__':
    main()
