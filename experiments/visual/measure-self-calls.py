import argparse,hashlib,json,pathlib,subprocess,time,statistics
parser=argparse.ArgumentParser(description='Compare cold one-shot indexing of synthetic wide Rust implementations.')
parser.add_argument('before');parser.add_argument('after');parser.add_argument('output')
args=parser.parse_args()
base=pathlib.Path(args.output).resolve();base.mkdir()
binaries={key:str(pathlib.Path(value).resolve()) for key,value in [('before',args.before),('after',args.after)]}
records=[]
for count in [512,2048,8192]:
 root=base/f'source-{count}';root.mkdir(exist_ok=True)
 text='struct Bench; impl Bench { fn render(&self) {}\n'+''.join(f'fn step_{i}(&self) {{ self.render(); }}\n' for i in range(count))+'}\n'
 (root/'lib.rs').write_text(text)
 for repeat in range(3):
  for mode,binary in binaries.items():
   cache=base/f'cache-{mode}-{count}-{repeat}'
   command=['nice','-n','10',binary,'--root',str(root),'--cache',str(cache),'--workers','1','--once']
   start=time.perf_counter();result=subprocess.run(command,text=True,capture_output=True,check=True);elapsed=time.perf_counter()-start
   status=json.loads(result.stdout);assert status['snapshot'],status
   records.append({'mode':mode,'methods':count,'bytes':len(text.encode()),'repeat':repeat,'elapsedMs':elapsed*1000,'snapshot':status['snapshot']})
   print(mode,count,repeat,round(elapsed*1000,1),flush=True)
out={'kind':'optimized native cold one-shot indexing; synthetic wide Rust impl','workers':1,'samples':records,'binarySha256':{k:hashlib.sha256(pathlib.Path(v).read_bytes()).hexdigest() for k,v in binaries.items()},'limitations':'Three sequential samples per size and binary; includes process start, capture, cache write, parsing/coloring/binding. Not isolated binder time or representative whole-repository qualification.'}
(base/'result.json').write_text(json.dumps(out,indent=2)+'\n')
for count in [512,2048,8192]:print(count,{mode:round(statistics.median(r['elapsedMs'] for r in records if r['methods']==count and r['mode']==mode),1) for mode in binaries})
