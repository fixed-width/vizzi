import{SearchSession,groupResults}from'/question-state.js';
export function installQuestionSearch({enabled,transport,inspect,keep,saveQuestion,openEditor,showError,startIndex}){
 const noop={receiveStatus(){},saved(){}};if(!enabled)return noop;
 const el=id=>document.getElementById(id),node=(tag,text)=>{const n=document.createElement(tag);n.textContent=text;return n;},session=new SearchSession();
 let controller,board,cards=[],saving=false;
 el('question-search-panel').hidden=false;el('discovery-panel').hidden=true;el('show-discovery').textContent='Question search';
 const saved=hit=>board?.evidence.find(e=>e.source.file===hit.file&&e.source.version===hit.version&&e.source.line===hit.line);
 function sync(){
  el('search-keep-question').disabled=!board||saving||!el('source-question').value.trim()||board?.question===el('source-question').value.trim();
  el('search-stale').hidden=!session.stale;el('search-results').setAttribute('aria-busy',String(session.pending));
  for(const card of cards){const existing=saved(card.hit);card.keep.textContent=existing?'Kept':'Keep this location';card.keep.disabled=!board||saving||!session.canKeep||Boolean(existing);card.editor.hidden=!existing;card.editor.disabled=saving||!session.canKeep;}
 }
 function render(){
  const data=session.data;if(!data)return;cards=[];const root=el('search-results');root.replaceChildren();root.dataset.selector=data.selector;
  el('search-scope').textContent=data.omitted_files?`${data.searched_files} of ${data.available_files} indexed files searched; some files are outside this trial’s search scope.`:`${data.searched_files} indexed files searched.`;
  el('search-summary').textContent=`${data.results.length} source starting points`;el('search-empty').hidden=data.results.length>0;
  for(const group of groupResults(data.results)){
   const section=node('section','');section.className='search-group';section.append(node('h2',group.label));
   for(const hit of group.items){
    const article=node('article','');article.className='search-result';article.dataset.searchResult=hit.id;article.append(node('h3',hit.title));
    const path=node('p',`${hit.path} · Lines ${hit.line}–${hit.end_line}`);path.className='search-location';const code=node('pre',hit.excerpt);article.append(path,code);
    const actions=node('div','');actions.className='search-actions';const preview=node('button','Inspect source'),keepButton=node('button','Keep this location'),editor=node('button','Open in VS Code');for(const b of[preview,keepButton,editor])b.type='button';
    preview.onclick=()=>inspect({label:hit.title,path:hit.path,snapshot:data.snapshot,location:{file:hit.file,version:hit.version,line:hit.line,kind:hit.scope?.kind??'source'}});
    keepButton.onclick=async()=>{if(!board||saving||!session.canKeep||saved(hit))return;saving=true;sync();try{await keep({snapshot:data.snapshot,file:hit.file,version:hit.version,line:hit.line,title:hit.title});}catch(e){showError(e);}finally{saving=false;sync();}};
    editor.onclick=()=>{const existing=saved(hit);if(existing&&session.canKeep)void openEditor(existing.id);};
    actions.append(preview,keepButton,editor);article.append(actions);section.append(article);cards.push({hit,keep:keepButton,editor});
   }
   root.append(section);
  }
  sync();
 }
 el('source-question').addEventListener('input',sync);
 el('source-question-form').onsubmit=async event=>{
  event.preventDefault();let request;
  try{request=session.begin(el('source-question').value.trim(),el('search-original').checked?'original':'focused');}catch(e){showError(e);return;}
  controller?.abort();controller=new AbortController();el('search-summary').textContent='Finding source…';sync();
  try{const data=await transport('/api/search',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(request.body),signal:controller.signal});if(session.complete(request.ticket,data))render();}
  catch(e){if(e.name!=='AbortError'&&request.ticket===session.ticket){session.fail(request.ticket);el('search-summary').textContent='Search did not complete. You can retry.';showError(e);}}
  finally{sync();}
 };
 el('search-keep-question').onclick=async()=>{if(!board||saving)return;saving=true;sync();try{await saveQuestion(el('source-question').value.trim());}catch(e){showError(e);}finally{saving=false;sync();}};
 el('search-retry-index').onclick=()=>startIndex();
 return{receiveStatus(status){const old=session.snapshot;session.setSnapshot(status.snapshot??null);if(old!==session.snapshot)controller?.abort();el('search-repository-title').textContent=status.name?`Search ${status.name}`:'Find a starting point in the code';el('search-retry-index').hidden=status.readiness!=='failed';sync();},saved(view){board=view.board;sync();}};
}
