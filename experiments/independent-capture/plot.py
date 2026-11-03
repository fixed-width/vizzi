#!/usr/bin/env python3
"""Plot every observed foreground publication sample for the two backends."""
import json
import pathlib
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

root = pathlib.Path(__file__).parent / 'results'
samples = json.loads((root / 'summary.json').read_text())['samples']
fig, axes = plt.subplots(1, 2, figsize=(11, 4.5))
groups = [('before', 'after_cleanup'), ('before', 'during_cleanup'),
          ('after', 'after_cleanup'), ('after', 'during_cleanup')]
labels = ['One file · after cleanup', 'One file · during cleanup',
          'Two files · after cleanup', 'Two files · during cleanup']
for ax, name in zip(axes, ['linux', 'pytorch']):
    for y, (implementation, condition) in enumerate(groups):
        values = [r['publication_ms'] for r in samples if r['profile'] == name
                  and r['implementation'] == implementation and r['condition'] == condition]
        ax.scatter(values, [y - .09, y, y + .09], s=38,
                   color='#417eab' if implementation == 'before' else '#48836b')
    ax.set_yticks(range(4), labels)
    ax.invert_yaxis()
    ax.set_title('Linux' if name == 'linux' else 'PyTorch')
    ax.set_xlabel('Write to observed source publication (ms)')
    ax.set_xlim(left=0)
    ax.grid(axis='x', alpha=.18)
    ax.set_axisbelow(True)
    for spine in ['top', 'right']:
        ax.spines[spine].set_visible(False)
fig.suptitle('Foreground publication around cleanup — every sample, same harness', fontsize=13)
fig.tight_layout(rect=[0, 0, 1, .92])
output = root / 'publication.svg'
fig.savefig(output)
output.write_text('\n'.join(line.rstrip() for line in output.read_text().splitlines()) + '\n')
