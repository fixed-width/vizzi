#!/usr/bin/env python3
"""Plot captured-buffer ownership separately from observed RSS."""
import argparse
import json
import pathlib

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('observations',type=pathlib.Path)
parser.add_argument('output',type=pathlib.Path)
parser.add_argument('--preview',type=pathlib.Path)
args=parser.parse_args()
if args.output.exists() or (args.preview and args.preview.exists()):parser.error('refusing to overwrite plot output')
data=json.loads(args.observations.read_text())
labels=['Initial','Verified','Edit 1','Edit 2','Edit 3','Edit 4','Pin released','Restored','Released']
series=[('current','Current buffers','#2467a8','-','o'),('publication','Publication buffers','#27836a','--','s'),('including_external','Including external pin','#c47720','-.','^')]
fig,axes=plt.subplots(1,2,figsize=(12,4.8))
for ax,profile in zip(axes,['linux','pytorch']):
    result=next(r for r in data['results'] if r['profile']['name']==profile and r['configuration']['mode']=='workspace')
    phases=result['report']['phases']
    for key,label,color,style,marker in series:
        ax.plot(range(len(phases)),[p[key]['buffer_bytes']/2**30 for p in phases],label=label,color=color,linestyle=style,marker=marker,markersize=4)
    ax.plot(range(len(phases)),[p['rss_kib']/2**20 for p in phases],label='Point process RSS',color='#676977',linestyle=':',marker='.',markersize=5)
    ax.set_title(f'{"Linux" if profile=="linux" else "PyTorch"}: {result["report"]["files"]:,} files')
    ax.set_xticks(range(len(labels)),labels,rotation=35,ha='right',fontsize=8)
    ax.set_ylabel('GiB');ax.set_ylim(bottom=0);ax.grid(alpha=.2)
axes[0].legend(fontsize=8,loc='upper left')
fig.suptitle('Whole-source version churn: buffer ownership and process RSS')
fig.text(.5,.015,'Different panel scales. Buffer capacities omit object/allocator costs; dead handles do not require RSS to fall.',ha='center',fontsize=8)
fig.tight_layout(rect=(0,.055,1,.94));fig.savefig(args.output)
args.output.write_text('\n'.join(line.rstrip() for line in args.output.read_text().splitlines())+'\n')
if args.preview:fig.savefig(args.preview,dpi=130)
print(args.output)
