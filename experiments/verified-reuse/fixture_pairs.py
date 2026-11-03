#!/usr/bin/env python3
"""Repeat large-file capture in opposite implementation orders on one owned fixture."""
import argparse
import hashlib
import json
import pathlib
import sys
import tempfile
import uuid

sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from experiments.qualification.native import execute


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', type=pathlib.Path, required=True)
    parser.add_argument('--after', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('output must be new')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    binaries = {name: path.resolve() for name, path in [('before', args.before), ('after', args.after)]}
    results = []
    with tempfile.TemporaryDirectory(prefix='vizzi-reuse-pairs-') as temporary:
        root = pathlib.Path(temporary) / 'source'
        root.mkdir()
        owner = str(uuid.uuid4())
        (root / '.vizzi-stream-owned').write_text(owner)
        with (root / 'large.rs').open('wb') as fixture:
            for _ in range(8192):
                fixture.write(b' ' * 65536)
        for pair, order in enumerate([['before', 'after'], ['after', 'before']]):
            for position, name in enumerate(order):
                stem = args.output.parent / f'{args.output.stem}-{pair}-{position}-{name}'
                config = stem.with_suffix('.config.json')
                with config.open('x') as file:
                    json.dump({'root': str(root), 'owner': owner, 'mode': 'stream'}, file)
                print(f'Pair {pair}, position {position}: {name}', flush=True)
                run = execute([sys.executable, str(pathlib.Path('experiments/admission/limit.py').resolve()),
                               '2048', str(binaries[name]), str(config)], 600, stem)
                report = run['events'][-1]['data'].pop('report')
                run['events'][-1]['data']['report_ref'] = 'report'
                assert report['bounded_bytes_match'] and report['unchanged_shared']
                assert report['bytes'] == report['final_spool']['written_bytes'] == 512 * 1048576
                results.append({'pair': pair, 'position': position, 'implementation': name,
                                'binary_sha256': hashlib.sha256(binaries[name].read_bytes()).hexdigest(),
                                'run': run, 'report': report})
    with args.output.open('x') as file:
        json.dump({'schema': 1, 'fixture_bytes': 512 * 1048576, 'results': results,
                   'limits': ['Two opposite-order pairs, one capture/unchanged sample per child; finite observations, not timing tails.',
                              'Same owned 512 MiB input; independent bounded verification; 2 GiB child virtual-address-space limit.']}, file, indent=2)
        file.write('\n')


if __name__ == '__main__':
    main()
