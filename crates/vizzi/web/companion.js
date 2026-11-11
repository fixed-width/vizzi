import {installAppHeader} from '/app-header.js';
import {installAppearance} from '/appearance.js';
import {createMessage,textRow} from '/messages.js';
import {followChanges} from '/updates.js';
import {installDiscovery} from '/discovery.js';
import {installStructure} from '/structure.js';
installAppHeader(document.querySelector('body > header'));
const element=id=>document.getElementById(id),node=(tag,text)=>{const n=document.createElement(tag);n.textContent=text;return n;};
installAppearance(element('theme'));
const error=createMessage(element('investigation-error'),{label:'Dismiss investigation error'}),notice=createMessage(element('investigation-notice'),{label:'Dismiss investigation notice'}),sourceLabel=textRow(element('source-state'));
let view,sourceStatus,selected,request=0,busy=false,noteDirty=false,wantIndex=false,updates;
const trialStructure=new URLSearchParams(location.search).get('trial')==='structure';
const keepLocation=location=>mutate(trialStructure?'/api/search/evidence':'/api/investigation/evidence',{revision:view.board.revision,snapshot:location.snapshot,file:location.file,version:location.version,line:location.line,title:location.title});
const discovery=installDiscovery({transport,openEditor,showError,keep:keepLocation,browseEnabled:!trialStructure});
const structure=installStructure({enabled:trialStructure,transport,inspect:discovery.inspect,showError});
function showMode(saved){
 if(noteDirty && !saved){notice.show('Save the current note before changing views.');return;}
 element('saved-workspace').hidden=!saved;element('discovery-panel').hidden=saved||trialStructure;element('structure-panel').hidden=saved||!trialStructure;
 element('show-saved').setAttribute('aria-pressed',String(saved));element('show-discovery').setAttribute('aria-pressed',String(!saved));
 discovery.closePreview();if(!saved)element('evidence-detail').hidden=true;
 history.replaceState(null,'',saved?'#saved':location.pathname+location.search);
}
element('show-saved').onclick=()=>showMode(true);element('show-discovery').onclick=()=>showMode(false);
const fresh=()=>new Map(view?.freshness ?? []),stateText=state=>({current:'Saved source unchanged in the current index',changed:'Source changed · saved location retained',missing:'Source absent from current index · saved location retained',unchecked:'Source not checked · start the index'}[state] ?? 'Source not checked');
async function transport(uri,options){const response=await fetch(uri,options);if(!response.ok){let message=await response.text();try{message=JSON.parse(message).message ?? message;}catch{}throw Object.assign(new Error(message),{status:response.status});}return response.json();}
function setBusy(value){busy=value;for(const b of document.querySelectorAll('button[type="submit"],#add-source,#remove-evidence,#detail-editor,form input,form textarea,form select'))b.disabled=value;if(view && view.board.evidence.length<2)element('connection-form').querySelector('button').disabled=true;updateSourceControls();}
function showError(e){error.show(e.message ?? String(e));}
async function load(){const token=++request;try{const next=await transport('/api/investigation');if(token===request)apply(next);}catch(e){showError(e);}}
async function mutate(uri,body,report=showError){if(busy)return null;setBusy(true);++request;error.show(null);try{const result=await transport(uri,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)});apply(result.view ?? result);return result;}catch(e){report(e);if(e.status===409)await load();return null;}finally{setBusy(false);}}
function apply(next){if(!next?.board || next.board.schema!==1 || !Array.isArray(next.board.evidence) || next.board.evidence.length>32 || !Array.isArray(next.board.connections) || next.board.connections.length>64)throw new Error('Invalid bounded investigation');if(view && next.board.revision<view.board.revision)return;const changed=!view || view.board.revision!==next.board.revision;view=next;if(changed){render();}else renderFreshness();discovery.saved(view);}
function render(){
 const board=view.board;
 element('save-mode').textContent=view.persistent?'Saved locally.':'Session investigation · export to keep a copy after the host stops.';
 element('investigation-counts').textContent=`${board.evidence.length} locations · ${board.connections.length} reader connections`;
 element('show-saved').textContent=`Saved locations (${board.evidence.length})`;element('connection-tools').hidden=board.evidence.length<2;element('connection-list-panel').hidden=board.connections.length===0;
 const cards=element('evidence-cards');cards.replaceChildren();element('empty-investigation').hidden=board.evidence.length>0;
 for(const evidence of board.evidence){const card=node('article','');card.className='evidence-card';card.dataset.evidence=evidence.id;card.append(node('h2',evidence.title));const path=node('p',`${evidence.source.path}:${evidence.source.line}`);path.className='evidence-path';const state=node('span','');state.className='source-freshness';const summary=node('p',evidence.note);summary.className='evidence-summary';const actions=node('div','');actions.className='card-actions';const inspect=node('button','Inspect location');inspect.type='button';inspect.onclick=()=>openDetail(evidence.id);const editor=node('button',`Open in ${view.editor}`);editor.type='button';editor.onclick=()=>openEditor(evidence.id);actions.append(inspect,editor);card.append(path,state,summary,actions);cards.append(card);}
 for(const id of ['connection-from','connection-to']){const select=element(id),before=select.value;select.replaceChildren();for(const e of board.evidence){const option=node('option',e.title);option.value=e.id;select.append(option);}if(board.evidence.some(e=>String(e.id)===before))select.value=before;}
 if(board.evidence.length>1 && element('connection-from').value===element('connection-to').value)element('connection-to').selectedIndex=1;
 element('connection-form').querySelector('button').disabled=board.evidence.length<2;
 const list=element('connection-list');list.replaceChildren();for(const link of board.connections){const from=board.evidence.find(e=>e.id===link.from),to=board.evidence.find(e=>e.id===link.to);const item=node('li',`${from.title} → ${to.title}: ${link.explanation}`);const marker=node('span','Your assertion · locations remain separately inspectable');marker.className='reader-assertion';item.append(marker);const remove=node('button','Remove connection');remove.type='button';remove.onclick=()=>mutate('/api/investigation',{action:'disconnect',revision:view.board.revision,from:link.from,to:link.to});item.append(remove);list.append(item);}
 if(selected && !board.evidence.some(e=>e.id===selected)){selected=undefined;element('evidence-detail').hidden=true;}if(selected)renderDetail();renderFreshness();requestAnimationFrame(drawConnections);
}
function renderFreshness(){const states=fresh();for(const card of element('evidence-cards').children){const state=states.get(Number(card.dataset.evidence));card.dataset.sourceState=state;card.querySelector('.source-freshness').textContent=stateText(state);}if(selected)element('detail-freshness').textContent=stateText(states.get(selected));}
function openDetail(id){if(noteDirty && selected!==id){notice.show('Save the current evidence note before switching cards.');return;}selected=id;element('evidence-detail').hidden=false;renderDetail();}
function renderDetail(){const evidence=view.board.evidence.find(e=>e.id===selected);if(!evidence)return;element('detail-title').textContent=evidence.title;element('detail-location').textContent=`${evidence.source.path}:${evidence.source.line}–${evidence.source.end_line}`;if(!noteDirty)element('evidence-note').value=evidence.note;element('detail-editor').textContent=`Open in ${view.editor}`;element('detail-version').textContent=`Saved source ${evidence.source.content.slice(0,8)} · document ${evidence.source.version.slice(0,8)}${evidence.source.truncated?' · excerpt shortened':''}`;element('detail-excerpt').textContent=evidence.source.excerpt;element('detail-freshness').textContent=stateText(fresh().get(selected));}
function drawConnections(){if(!view)return;const svg=element('evidence-links'),bounds=element('evidence-map').getBoundingClientRect();svg.replaceChildren();for(const connection of view.board.connections){const from=element('evidence-cards').querySelector(`[data-evidence="${connection.from}"]`),to=element('evidence-cards').querySelector(`[data-evidence="${connection.to}"]`);if(!from || !to)continue;const a=from.getBoundingClientRect(),b=to.getBoundingClientRect(),x=a.left+a.width/2-bounds.left,y=a.bottom-bounds.top,tx=b.left+b.width/2-bounds.left,ty=b.top-bounds.top,mid=(y+ty)/2;const path=document.createElementNS('http://www.w3.org/2000/svg','path');path.setAttribute('d',`M ${x} ${y} C ${x} ${mid} ${tx} ${mid} ${tx} ${ty}`);svg.append(path);}}
async function startIndex(){wantIndex=true;try{await transport('/api/workspace/start',{method:'POST',headers:{'Content-Type':'application/json'},body:'{}'});}catch(e){showError(e);}}
function updateSourceControls(){const ready=Boolean(sourceStatus?.snapshot);element('pin-source').disabled=busy || !ready;element('index-source').disabled=Boolean(sourceStatus?.pending);element('index-source').hidden=ready || !sourceStatus?.error;element('source-dialog-state').textContent=ready?'The saved excerpt will use the current captured version.':sourceStatus?.error?sourceStatus.error:wantIndex?'Preparing the source index…':'Start the source index to capture evidence.';if(sourceStatus)sourceLabel.show(ready?'Source ready':sourceStatus.error?'Source indexing failed':'Mapping source…');}
function showSourceError(e){element('source-dialog-error').textContent=e?.message ?? '';element('source-dialog-error').hidden=!e;}
function addSource(){element('locations-actions').open=false;showSourceError(null);element('source-dialog').showModal();if(!sourceStatus?.snapshot)void startIndex();updateSourceControls();element('source-path').focus();}
async function openEditor(id){if(busy)return;setBusy(true);error.show(null);try{await transport(`/api/investigation/evidence/${id}/open`,{method:'POST',headers:{'Content-Type':'application/json'},body:'{}'});notice.show(`${view.editor} handoff requested for the verified working file. The editor opens a live file.`);}catch(e){showError(e);await load();}finally{setBusy(false);}}
element('evidence-note').addEventListener('input',()=>{noteDirty=true;});
element('source-form').addEventListener('submit',async event=>{
 event.preventDefault();if(!view || busy)return;showSourceError(null);
 const path=element('source-path').value,line=Number(element('source-line').value),title=element('source-title').value;
 let target;setBusy(true);
 try{target=await transport(`/api/investigation/lookup?${new URLSearchParams({path})}`);}
 catch(e){showSourceError(e);return;}finally{setBusy(false);}
 const result=await mutate('/api/investigation/evidence',{revision:view.board.revision,snapshot:target.snapshot,file:target.file,version:target.version,line,title},showSourceError);
 if(result){element('source-dialog').close();element('source-title').value='';}
});
element('note-form').addEventListener('submit',async event=>{event.preventDefault();if(!selected)return;const result=await mutate('/api/investigation',{action:'annotate',revision:view.board.revision,id:selected,note:element('evidence-note').value});if(result){noteDirty=false;renderDetail();}});
element('connection-form').addEventListener('submit',async event=>{event.preventDefault();const result=await mutate('/api/investigation',{action:'connect',revision:view.board.revision,from:Number(element('connection-from').value),to:Number(element('connection-to').value),explanation:element('connection-text').value});if(result)element('connection-text').value='';});
element('remove-evidence').addEventListener('click',async()=>{if(!selected)return;const result=await mutate('/api/investigation',{action:'remove',revision:view.board.revision,id:selected});if(result)noteDirty=false;});
element('close-detail').addEventListener('click',()=>{if(noteDirty){notice.show('Save the current note before closing its detail.');return;}selected=undefined;element('evidence-detail').hidden=true;});element('detail-editor').addEventListener('click',()=>openEditor(selected));
element('add-source').addEventListener('click',addSource);element('empty-add-source').addEventListener('click',()=>showMode(false));element('index-source').addEventListener('click',startIndex);element('cancel-source').addEventListener('click',()=>element('source-dialog').close());
element('export').addEventListener('click',()=>{if(!view)return;const url=URL.createObjectURL(new Blob([JSON.stringify(view.board,null,2)+'\n'],{type:'application/json'}));const link=node('a','');link.href=url;link.download='investigation.json';link.click();setTimeout(()=>URL.revokeObjectURL(url),0);});
new ResizeObserver(()=>requestAnimationFrame(drawConnections)).observe(element('evidence-map'));
updates=followChanges({session:()=>({receive(change){const old=sourceStatus?.snapshot;sourceStatus=change.status;updateSourceControls();discovery.receiveStatus(sourceStatus);structure.receiveStatus(sourceStatus);if(old!==sourceStatus.snapshot && view)void load();return true;},close(e){if(e)sourceLabel.show('Source updates disconnected; saved investigation is kept');}})});
showMode(location.hash==='#saved');
void load();void startIndex();
