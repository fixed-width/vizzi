#!/usr/bin/env python3
"""Archive raw matrix evidence and summarise simultaneous memory samples."""
import argparse
import collections
import csv
import gzip
import hashlib
import json
import pathlib
import shutil
import statistics


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    matrix = json.loads((args.input / 'matrix.json').read_text())
    shutil.copyfile(args.input / 'matrix.json', args.output / 'matrix.json')
    summary = {'schema': 1, 'mode': matrix['mode'], 'conditions': [], 'raw_sha256': {}}
    rows = []
    for condition in matrix['results']:
        stem = condition['profile'] + '-' + condition['backing']
        browser_bytes = (args.input / condition['browser']).read_bytes()
        memory_bytes = (args.input / condition['memory']).read_bytes()
        browser, memory = json.loads(browser_bytes), json.loads(memory_bytes)
        result = browser['results'][0]
        assert memory['success']
        assert memory['cap_verified'] == (matrix['mode'] == 'cgroup')
        assert result['cleanSnapshotMatches']
        for label, data in [('browser', browser_bytes), ('memory', memory_bytes)]:
            name = stem + '.' + label + '.json.gz'
            (args.output / name).write_bytes(gzip.compress(data, mtime=0))
            summary['raw_sha256'][name] = hashlib.sha256(data).hexdigest()
        for sample in memory['samples']:
            role = collections.Counter()
            for entry in sample['processes']:
                name = entry['process']['name']
                category = 'engine' if name == 'vizzi' else 'chromium' if name.startswith('chrome') else 'harness'
                role[category] += entry['usage']['pss_kib']
            assert sum(role.values()) == sample['usage']['pss_kib']
            rows.append({'condition': stem, 'elapsed_ms': sample['elapsed_ms'],
                         'sampling_ms': sample['sampling_ms'], **sample['usage'],
                         **{key + '_pss_kib': role[key] for key in ['engine', 'chromium', 'harness']},
                         'processes': len(sample['processes']),
                         'disappeared': len(sample['disappeared']), 'errors': len(sample['errors'])})
        peak = max(memory['samples'], key=lambda sample: sample['usage']['pss_kib'])
        refresh = [run for run in result['runs'] if run['condition'] == 'refresh']
        screenshot = pathlib.Path(result['screenshot'])
        shutil.copyfile(screenshot, args.output / screenshot.name)
        summary['conditions'].append({
            'profile': condition['profile'], 'backing': condition['backing'],
            'cap_verified': memory['cap_verified'], 'files': result['capturedFiles'],
            'source_manifest_sha256': result['originalSourceManifestSha256'],
            'pss_peak_gib': memory['peak_pss_kib'] / 1048576,
            'rss_peak_gib': memory['peak_rss_kib'] / 1048576,
            'peak_sample': peak, 'sample_count': len(memory['samples']),
            'nominal_interval_ms': memory['interval_ms'],
            'sampling_ms': {'median': statistics.median(s['sampling_ms'] for s in memory['samples']),
                            'max': max(s['sampling_ms'] for s in memory['samples'])},
            'wall_s': memory['wall_ms'] / 1000,
            'disappeared_observations': sum(len(s['disappeared']) for s in memory['samples']),
            'sample_errors': sum(len(s['errors']) for s in memory['samples']),
            'observer_errors': memory['errors'],
            'ordinary_dom_ms': result['editLatency']['summary'],
            'large_edit_dom_ms': result['largeSource']['editToDomAndTwoRafMs'],
            'large_file': result['largeSource']['path'], 'large_bytes': result['largeSource']['bytes'],
            'refresh_frame_p99_ms': [run['frameSummary']['p99'] for run in refresh],
            'max_observed_frame_ms': max(run['frameSummary']['max'] for run in result['runs']),
            'native_observations': [run['publishedRefresh']['observation'] if run['publishedRefresh'] else None for run in refresh],
            'clean_matches': result['cleanSnapshotMatches'], 'screenshot': screenshot.name,
            'group_before': memory['group_before'], 'group_after': memory['group_after'],
        })
    summary['machine'] = browser['machine']
    summary['browser'] = browser['browser']
    summary['node'] = browser['node']
    summary['playwright'] = browser['playwright']
    summary['gpu'] = browser['gpu']
    (args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    with (args.output / 'samples.csv').open('w') as file:
        writer = csv.DictWriter(file, fieldnames=list(rows[0]), lineterminator='\n')
        writer.writeheader()
        writer.writerows(rows)


if __name__ == '__main__':
    main()
