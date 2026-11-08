// Explicit structural operations over captured metadata; no task/relationship inference.
export function installStructure({enabled,transport,inspect,showError}) {
 const noop={receiveStatus(){},saved(){}};if(!enabled)return noop;
 const el=id=>document.getElementById(id),node=(tag,text)=>{const n=document.createElement(tag);n.textContent=text;return n;};
 let snapshot=null,ticket=0,controller,pending=false,valid=false,data,area='',trail=[],current;
 el('structure-panel').hidden=false;el('show-discovery').textContent='Structure';
 function sync(){el('structure-results').setAttribute('aria-busy',String(pending));for(const button of el('structure-results').querySelectorAll('button'))button.disabled=!valid||pending;for(const button of el('structure-trail').querySelectorAll('button'))button.disabled=!valid||pending;}
 const action=(label,fn)=>{const b=node('button',label);b.type='button';b.onclick=()=>{if(valid&&!pending)fn();};return b;};
 const itemSource=(source,label,kind)=>({label,path:source.path,snapshot,location:{file:source.file,version:source.version,line:source.line,kind}});
 function location(source){const p=node('p',`${source.path} · Line ${source.line}`);p.className='search-location';return p;}
 function renderTrail(){
  const root=el('structure-trail');root.replaceChildren();
  trail.forEach((symbol,i)=>{const b=action(symbol.name,()=>{trail=trail.slice(0,i+1);void send({kind:'calls',symbol:symbol.id,area,offset:0});});b.dataset.trailIndex=String(i);b.title=symbol.source.path;root.append(b);});
 }
 function symbols(rows){
  const root=el('structure-results');
  for(const entry of rows){const s=entry.symbol,row=node('article','');row.className='search-result';row.dataset.structureSymbol=s.id;row.dataset.name=s.name;
   row.append(node('h2',`${s.kind} ${s.parents.length?s.parents.join(' › ')+' › ':''}${s.name}`),location(s.source));
   row.append(node('p',`${entry.candidate_uses} recorded same-name call sites · ${entry.same_name_definitions} declarations with that name`));
   const actions=node('div','');actions.className='search-actions';actions.append(action('Inspect declaration',()=>inspect(itemSource(s.source,s.name,s.kind))),action('Show candidate uses',()=>{trail=[s];void send({kind:'uses',symbol:s.id,area,offset:0});}),action('Follow written calls',()=>{trail=[s];void send({kind:'calls',symbol:s.id,area,offset:0});}));row.append(actions);root.append(row);
  }
 }
 function calls(rows){
  const root=el('structure-results');
  for(const site of rows){const row=node('article','');row.className='search-result';row.dataset.callSite=site.source.file+':'+site.source.line;row.append(node('h2',site.spelling),location(site.source));
   row.append(action('Inspect call',()=>inspect(itemSource(site.source,site.name??'Dynamic call','call'))));
   row.append(node('p',site.name?`${site.candidate_count} candidate declarations; terminal-name matching only.`:'Dynamic callee; no destination inferred.'));
   for(const target of site.candidates){const b=action(`Follow ${target.name}`,()=>{trail=[...trail.slice(-7),target];void send({kind:'calls',symbol:target.id,area,offset:0});});b.title=`${target.source.path}:${target.source.line}`;row.append(b,node('span',` ${target.source.path}${target.source.path===area||target.source.path.startsWith(area+'/')?'':' (outside area)'} `));}
   root.append(row);
  }
 }
 function render(){
  const root=el('structure-results');root.replaceChildren();renderTrail();
  const c=data.coverage;el('structure-scope').textContent=`${area||'Repository'} · ${c.indexed_files}/${c.available_files} indexed files · ${c.unavailable_call_files} files without call analysis · ${c.partial_files} files with partial syntax · ${c.omitted_files} files, ${c.omitted_symbols} declarations, ${c.omitted_calls} calls and ${c.omitted_areas} areas omitted.`;
  el('structure-summary').textContent=`${data.matching} matching ${current.kind==='areas'?'areas':current.kind==='inventory'?'declarations':'written call sites'}`;
  if(current.kind==='areas')for(const a of data.items){const b=action(`${a.path||'Repository'} · ${a.files} files`,()=>{area=a.path;trail=[];void send({kind:'inventory',area,search:el('structure-search').value.trim(),offset:0});});b.dataset.area=a.path;root.append(b);}
  else if(current.kind==='inventory')symbols(data.items);else calls(data.items);
  if(!data.items.length)root.append(node('p','No matches in this admitted source scope. Missing call analysis is not evidence of no calls.'));
  if(data.next!==null&&data.next!==undefined)root.append(action('Next results',()=>void send({...current,offset:data.next})));
  if(current.offset>0)root.append(action('First results',()=>void send({...current,offset:0})));
  sync();
 }
 async function send(query){
  if(!snapshot){showError(new Error('Wait for source indexing before exploring.'));return;}
  const id=++ticket,expected=snapshot;controller?.abort();controller=new AbortController();pending=true;valid=false;sync();el('structure-summary').textContent='Reading captured structure…';
  try {const result=await transport('/api/structure',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({snapshot:expected,query}),signal:controller.signal});if(id!==ticket||snapshot!==expected)return;
   const limit=query.kind==='areas'||query.kind==='inventory'?24:8;if(result.snapshot!==expected||!Array.isArray(result.items)||result.items.length>limit||!Number.isSafeInteger(result.matching)||result.matching<result.items.length)throw new Error('Invalid bounded structure response');
   data=result;current=query;pending=false;valid=true;el('structure-stale').hidden=true;render();
  }catch(error){if(id===ticket&&error.name!=='AbortError'){pending=false;showError(error);el('structure-summary').textContent='Exploration did not complete. Retry finding an area.';sync();}}
 }
 el('structure-form').onsubmit=event=>{event.preventDefault();trail=[];area='';void send({kind:'areas',search:el('structure-search').value.trim(),offset:0});};
 el('structure-areas').onclick=()=>{trail=[];area='';void send({kind:'areas',search:el('structure-search').value.trim(),offset:0});};
 el('structure-inventory').onclick=()=>{trail=[];void send({kind:'inventory',area,search:'',offset:0});};
 return{receiveStatus(status){const next=status.snapshot??null;if(next===snapshot)return;const had=snapshot;snapshot=next;ticket++;controller?.abort();pending=false;valid=false;if(data)el('structure-stale').hidden=false;sync();if(!had&&snapshot&&!data)void send({kind:'areas',search:'',offset:0});},saved(){}};
}
