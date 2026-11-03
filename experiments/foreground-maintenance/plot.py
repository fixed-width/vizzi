#!/usr/bin/env python3
"""Plot the measured foreground latency components from the recorded summary."""
import json
import pathlib
import matplotlib

matplotlib.use('Agg')
import matplotlib.pyplot as plt

root = pathlib.Path(__file__).parent / 'results'
rows = json.loads((root / 'summary.json').read_text())['samples']
fig, axes = plt.subplots(1, 2, figsize=(12, 5.1))
keys = ['write_ms', 'hint_to_entry_ms', 'gate_ms', 'ready_to_published_ms']
labels = ['Write', 'Hint to capture entry', 'Writer / maintenance gate', 'Ready to observed publication']
colors = ['#777777', '#417eab', '#dd8a29', '#48836b']
for ax, name in zip(axes, ['linux', 'pytorch']):
    selected = [r for r in rows if r['profile'] == name]
    left = [0.0] * len(selected)
    for key, label, color in zip(keys, labels, colors):
        values = [r[key] for r in selected]
        ax.barh(range(len(selected)), values, left=left, label=label, color=color, height=.65)
        left = [a + b for a, b in zip(left, values)]
    ax.set_yticks(range(len(selected)), [('After' if r['condition'] == 'after_cleanup' else 'During')
                                       + f' · sample {r["sample"]}' for r in selected])
    ax.invert_yaxis()
    ax.set_title('Linux' if name == 'linux' else 'PyTorch')
    ax.set_xlabel('Write to observed source publication (ms)')
    ax.grid(axis='x', alpha=.18)
    ax.set_axisbelow(True)
    for spine in ['top', 'right']:
        ax.spines[spine].set_visible(False)
fig.suptitle('Foreground edits around real cleanup — three samples per condition', fontsize=13)
handles, legend = axes[0].get_legend_handles_labels()
fig.legend(handles, legend, loc='lower center', ncol=2, frameon=False)
fig.tight_layout(rect=[0, .15, 1, .93])
output = root / 'edit-latency.svg'
fig.savefig(output)
output.write_text('\n'.join(line.rstrip() for line in output.read_text().splitlines()) + '\n')
