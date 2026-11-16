import {installAppHeader} from '/app-header.js';
import {fetchRead} from '/read-retry.js';
import {captureFor,originLabel} from '/source-origin.js';
import {modulePresentation} from '/module-identity.js';
import {installModuleOutlines} from '/module-outlines.js';
import {installLinkHighlights} from '/linked-hover.js';
import {BUBBLE_SIZE} from '/layout-growth.js';
import {worldPoint,frozenSource,frozenLabel} from '/visual-model.js';
import {TaskWorkspace as Workspace} from '/task-workspace.js';
import {installTaskControls,installTaskGroups} from '/workspace-tasks.js';
import {installReferenceStacks} from '/reference-stacks.js';
import {LiveUpdates} from '/live-updates.js';
import {DefinitionWarmup} from '/definition-warmup.js';
import {SourcePane} from '/source-pane.js';
import {functionIndent} from '/source-margin.js';
import {renderCallText,createCallMenu,retargetCalls} from '/call-actions.js';
import {installAppearance} from '/appearance.js';
import {createMessage} from '/messages.js';
import {followChanges} from '/updates.js';
import {installWorkspaceDelivery} from '/editor-delivery.js';
import {createCallPreparation,resolveCallChoice,directDestination} from '/call-delivery.js';
installAppHeader(document.querySelector('body > header'));
const el=id=>document.getElementById(id),node=(tag,text='')=>{const n=document.createElement(tag);n.textContent=text;return n;},svg=(tag,attrs={})=>{const n=document.createElementNS('http://www.w3.org/2000/svg',tag);for(const[k,v]of Object.entries(attrs))n.setAttribute(k,String(v));return n;};
const error=createMessage(el('visual-error'),{label:'Dismiss workspace error'}),notice=createMessage(el('visual-notice'),{label:'Dismiss workspace notice'});
let definitionProvider=false;const definitionCapabilities=transport('/api/structure').then(c=>{definitionProvider=c.definition_provider===true;}).catch(()=>{});
let model,currentSnapshot,taskControls,board,editorName='editor',busy=false,saveTimer,paletteTicket=0,jobs=Promise.resolve(),demoOpened=false,livePendingTimer;
const mounted=new Map();
const highlights=installLinkHighlights(el('workspace-canvas'),()=>({views:model?.views??[],links:model?.links??[]}));
const outlines=installModuleOutlines(el('module-outlines'),()=>model?model.visibleItems():[],{failed:()=>notice.show('Module outlines are unavailable for this layout.')});
let showModuleOutlines=false;try{showModuleOutlines=localStorage.getItem('vizzi.visual.module-outlines')==='true';}catch{}
el('module-outlines-setting').checked=showModuleOutlines;outlines.setEnabled(showModuleOutlines);el('module-outlines-setting').onchange=()=>{showModuleOutlines=el('module-outlines-setting').checked;outlines.setEnabled(showModuleOutlines);try{localStorage.setItem('vizzi.visual.module-outlines',String(showModuleOutlines));}catch{notice.show('The module outline preference could not be saved.');}};
const callMenu=createCallMenu(el('call-action-menu'));
function inlineAction(id,action,element,direct,keyboard){if(!model.navigable(id)||mounted.get(id)?.historyLoading)return;editorCalls.manual();callMenu.hide();const open=()=>{if(mounted.has(id)&&model.navigable(id))void chooseCall(id,{...action,lookup:true});};if(direct)open();else callMenu.show(element,action.name,[{label:frozenSource(model.view(id))?'Open in captured file':'Open definition',hint:'Ctrl/⌘-click',run:open}],keyboard);}
function inlineAvailability(code,id){for(const name of code.querySelectorAll('.code-call')){name.setAttribute('aria-disabled',String(!model.navigable(id)||Boolean(mounted.get(id)?.historyLoading)));name.tabIndex=model.navigable(id)&&!mounted.get(id)?.historyLoading?0:-1;if(frozenSource(model.view(id)))name.title='Captured-file call · Ctrl/⌘-click opens a same-file destination';}}
installAppearance(el('theme'));
let smartReflow=true;try{smartReflow=localStorage.getItem('vizzi.visual.code-layout')!=='original';}catch{}
el('code-layout').value=smartReflow?'smart':'original';el('code-layout').onchange=()=>{smartReflow=el('code-layout').value==='smart';for(const r of mounted.values())r.sourcePane.setReflow(smartReflow);try{localStorage.setItem('vizzi.visual.code-layout',smartReflow?'smart':'original');}catch{notice.show('This browser could not save the code layout preference.');}};
const fail=e=>error.show(e?.message??String(e));
async function transport(path,options){const response=await fetchRead(path,options);if(!response.ok){let text=await response.text();try{text=JSON.parse(text).message??text;}catch{}const error=new Error(text);error.status=response.status;throw error;}return response.json();}
const post=(path,body,signal)=>transport(path,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body),signal});
const definitionWarmup=new DefinitionWarmup((snapshot,call)=>post('/api/call-definition',{snapshot,call}));
function refreshCapture(){
 const views=model.views.filter(v=>v.mode==='live');const key=JSON.stringify([views.map(v=>[v.id,v.symbol.id,v.mode,mounted.get(v.id)?.sourceTicket,v.reading]),model.links.map(l=>[l.id,l.state])]);
 return {key,body:{views:views.map(v=>{const r=mounted.get(v.id),context=r?.sourcePane.readingContext(),reading=v.reading,a=reading.anchor;return {id:v.id,tracking:v.symbol.tracking??null,symbol:v.symbol.id,file:v.symbol.source.file,context:v.symbol.source.context,capture:v.symbol.origin?.capture??null,version:r?.lines?.length?v.symbol.source.version:'0'.repeat(64),area:v.area,section_offset:reading.section-v.symbol.source.line,anchor:a?{offset:a.line-v.symbol.source.line,byte:a.byte,text:context?.reading.anchor?.line===a.line?context.text:''}:null};}),links:model.links.map(l=>({id:l.id,from:l.from,to:l.to,spelling:l.spelling,kind:l.kind}))}};
}
function applyLive(data){
 if(data.snapshot!==currentSnapshot||!Array.isArray(data.views)||data.views.length>24||!Array.isArray(data.links)||data.links.length>96)throw new Error('Live observation differs');
 for(const update of data.views){const r=mounted.get(update.id);if(!r)throw new Error('Live view was closed');if(update.state!=='current')continue;
  const s=update.symbol?.source,w=update.window;
  if(!s||!Array.isArray(update.actions)||update.actions.length>4096||!Array.isArray(update.calls?.items)||update.calls.items.length>8||update.calls.snapshot!==data.snapshot)throw new Error('Invalid live call metadata');
  if(w&&(w.file!==s.file||w.version!==s.version||!Array.isArray(w.lines)||w.lines.length>96||update.origin?.number!==s.line||update.origin.left||update.origin.column!==0))throw new Error('Invalid live source window');
  if(!w&&(!r.lines?.length||model.view(update.id).symbol.source.version!==s.version))throw new Error('Live source window is missing');
 }
 const publicationChanged=model.snapshot!==data.snapshot;clearTimeout(livePendingTimer);model.refresh(data.snapshot,data.views,data.links);callMenu.hide();hideDestinations();
 for(const update of data.views){const r=mounted.get(update.id),v=model.view(update.id);r.navigation=(r.navigation??0)+1;r.resolving=false;r.warming=false;r.awaitingLive=false;r.sourceTicket=(r.sourceTicket??0)+1;
  if(update.state!=='current'){r.popup.hidden=true;updateFragmentStatus(r);continue;}
  const preserve=!update.window&&retargetCalls(r.code,r.actions,update.actions);r.actions=update.actions;
  if(update.window){r.lines=update.window.lines.filter(l=>l.number<=v.symbol.source.end_line);r.margin=functionIndent(update.origin);}
  if(!preserve)r.sourcePane.set(r.lines,{...v.reading},r.margin??0);
  const p=update.position,last=r.lines.at(-1)?.number??p.section;
  r.sectionLabel.textContent=`Lines ${p.section}–${last} · ${v.symbol.source.end_line-v.symbol.source.line+1}-line function`;r.prev.hidden=p.section<=v.symbol.source.line;r.next.hidden=last>=v.symbol.source.end_line;
  const callsChanged=JSON.stringify(r.sites)!==JSON.stringify(update.calls.items);r.sites=update.calls.items;r.nextCalls=update.calls.next;r.matching=update.calls.matching;r.callsReady=true;if(publicationChanged||update.window)r.popup.hidden=true;if(callsChanged)renderCalls(v.id);
  r.element.dataset.name=v.symbol.name;r.element.setAttribute('aria-label',`${v.symbol.name} bubble`);r.element.querySelector('.fragment-heading strong').textContent=[...v.symbol.parents,v.symbol.name].join('::');r.element.querySelector('.fragment-heading small').textContent=`${v.symbol.origin?originLabel(v.symbol.origin)+' · ':''}${v.symbol.source.path}:${v.symbol.source.line}`;
  sourceStatus(r,p.reset?'Live · reading reset':'Live');model.read(v.id,{section:v.reading.section,...r.sourcePane.reading()});void prepareDefinitions(v.id);
 }
 geometry();save();void editorCalls.resume();
}
const live=new LiveUpdates({capture:refreshCapture,request:(body,signal)=>post('/api/visual/refresh',body,signal),apply:applyLive,blocked:()=>busy||[...mounted.values()].some(r=>r.loading||r.sourcePane.selected()),pending:snapshot=>{if(model){clearTimeout(livePendingTimer);model.markSnapshot(snapshot);geometry();if(model.stale)livePendingTimer=setTimeout(()=>{for(const r of mounted.values()){r.awaitingLive=model.stale;updateFragmentStatus(r);}},150);}},failed:e=>{if(e.status!==409)notice.show('Live refresh is waiting. Earlier source is retained.');}});
function refreshLive(){if(model&&currentSnapshot)live.changed(currentSnapshot,true);}
const prepareEditorCall=createCallPreparation({read:()=>model,
 bind:(action,signal)=>post('/api/editor/call',{snapshot:action.snapshot,query:{kind:'invocation',call:action.call}},signal),
 resolve:async(site,signal,snapshot)=>{await definitionCapabilities;return resolveCallChoice(site,{snapshot,provider:definitionProvider,warmup:()=>definitionWarmup.wait(),request:(snapshot,call,signal)=>post('/api/call-definition',{snapshot,call},signal),signal});},
 source:prepareFragmentSource,
 apply:({caller,target,choice,sources})=>{
  geometry();mount(caller,sources.get(model.view(caller).symbol.id));
  if(target)mount(target,sources.get(model.view(target).symbol.id));
  if(target){hideDestinations();highlightCall(caller,choice.site);focus([caller,target]);}
  else {focus([caller]);showDestinations(caller,choice);}
  save();
 }});
const editorCalls=installWorkspaceDelivery({mode:'call',
 ready:action=>Boolean(size().w>=1&&size().h>=1&&(action.kind==='reveal'||model&&!model.stale)),
 prepare:(action,context)=>action.kind==='reveal'?Promise.resolve({commit(){}}):prepareEditorCall(action,context),failed:fail,
 changed:pending=>{if(pending){callMenu.hide();hideDestinations();window.getSelection()?.removeAllRanges();for(const r of mounted.values()){r.navigation=(r.navigation??0)+1;r.historyTicket=(r.historyTicket??0)+1;r.historyController?.abort();if(r.historyLoading)r.comparing=r.historyLoading=r.loading=false;r.resolving=false;updateFragmentStatus(r);}geometry();live.resume();}}
});
document.addEventListener('selectionchange',()=>live.resume());
function updateFragmentStatus(r){const v=model.view(r.id);r.state.textContent=r.historyLoading?(r.comparing?'Loading comparison…':'Loading function history…'):r.resolving&&r.resolvingCaptured?'Finding captured destination…':v.captured?(v.captured.returning?(r.awaitingLive?'Returning to live…':`Function unavailable — captured ${shortRevision(v.captured.label)}`):`Captured · ${shortRevision(v.captured.label)}`):v.comparison?(v.comparison.returning?(r.awaitingLive?'Returning to live…':`Function unavailable — ${comparisonTitle(v.comparison)}`):comparisonTitle(v.comparison)):v.history?.returning?(r.awaitingLive?'Returning to live…':`Function unavailable — revision ${v.history.commit.slice(0,8)}`):v.history?`Revision ${v.history.commit.slice(0,8)}`:r.resolving?'Finding definition…':r.warming?'Preparing type navigation…':r.awaitingLive&&v.mode==='live'?'Update pending':(v.state==='unresolved'?'Function unavailable — earlier version':v.mode==='pinned'?'Pinned version':r.sourceStatus??'Live');}
async function changeHistory(id,direction){
 const r=mounted.get(id),workspace=model,v=model.view(id);if(!r||r.loading||v.history?.returning&&r.awaitingLive||r.sourcePane.selected())return;
 if(!v.history&&(!model.usable(id)||v.symbol.origin))return;
 editorCalls.manual();callMenu.hide();hideDestinations();r.popup.hidden=true;r.navigation=(r.navigation??0)+1;r.resolving=false;r.warming=false;
 const binding=JSON.stringify([v.snapshot,v.symbol.id,v.history]),ticket=r.historyTicket=(r.historyTicket??0)+1,controller=new AbortController();r.historyController=controller;r.historyLoading=true;r.loading=true;geometry();
 const verify=()=>{if(model!==workspace||mounted.get(id)!==r||r.historyTicket!==ticket||JSON.stringify([model.view(id).snapshot,model.view(id).symbol.id,model.view(id).history])!==binding)throw new DOMException('Function history superseded','AbortError');if(r.sourcePane.selected())throw new Error('Clear the source selection before changing revision.');};
 try{
  const revision=v.history?await post('/api/function-history/step',{token:v.history.token,index:v.history.index+direction},controller.signal):await post('/api/function-history',{snapshot:v.snapshot,file:v.symbol.source.file,version:v.symbol.source.version,tracking:v.symbol.tracking},controller.signal);
  verify();const planned=Workspace.restore(model.serialize(),model.snapshot);planned.showHistory(id,revision);const next=planned.view(id),s=next.symbol.source,start=next.reading.section;
  const query=new URLSearchParams({token:revision.token,snapshot:revision.snapshot,file:s.file,version:s.version,line_start:String(start),line_count:String(Math.min(96,s.end_line-start+1)),outline_count:'1',reflow:'true'});
  const [data,actions]=await Promise.all([start===s.line?revision.window:transport('/api/function-history/window?'+query,{signal:controller.signal}),capturedActions(next,start,Math.min(96,s.end_line-start+1),controller.signal)]);verify();
  if(data?.snapshot!==revision.snapshot||data.file!==s.file||data.version!==s.version||!Array.isArray(data.lines)||!data.lines.length||data.lines.length>96||data.lines[0].number!==start||revision.origin?.number!==s.line||revision.origin.left||revision.origin.column!==0)throw new Error('Historical source differs from the requested function');
  r.sourceTicket=(r.sourceTicket??0)+1;model.showHistory(id,revision);r.historyLoading=false;r.loading=false;r.sites=[];r.callsReady=false;r.actions=[];
  r.element.querySelector('.fragment-heading small').textContent=`${s.path}:${s.line}`;
  applyPreparedSource(id,{data,actions,start,margin:functionIndent(revision.origin)});save();
 }catch(e){if(e.name!=='AbortError'&&mounted.get(id)===r&&model===workspace&&r.historyTicket===ticket)fail(e);}
 finally{if(mounted.get(id)===r&&r.historyTicket===ticket){r.historyLoading=false;r.loading=false;geometry();live.resume();}}
}
function returnHistoryLive(id){
 const r=mounted.get(id);editorCalls.manual();callMenu.hide();hideDestinations();r.popup.hidden=true;r.historyController?.abort();r.historyTicket=(r.historyTicket??0)+1;r.sourceTicket=(r.sourceTicket??0)+1;r.navigation=(r.navigation??0)+1;
 model.returnLive(id);r.historyLoading=false;r.loading=false;r.resolving=false;r.actions=[];r.callsReady=false;r.awaitingLive=true;geometry();save();refreshLive();
}
function shortRevision(label){return /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/.test(label)?label.slice(0,8):label;}
function comparisonTitle(c){return `${c.side==='older'?'Before':'After'} · ${shortRevision(c.label)}`;}
function sourceBinding(v){const owner=frozenSource(v)?.owner;return owner?.kind==='comparison'?{token:owner.token,side:owner.side}:owner?{token:owner.token}:{};}
function sourceEndpointForView(v){const owner=frozenSource(v)?.owner;return owner?.kind==='comparison'?'/api/function-comparisons/window':owner?'/api/function-history/window':v.symbol.origin?'/api/dependency/window':'/api/window';}
async function capturedActions(v,start,count,signal){
 const actions=[];let offset=0;
 try{for(let page=0;page<16;page++){const data=await post('/api/captured-functions',{source:frozenSource(v),query:{kind:'actions',line_start:start,line_count:count,offset}},signal);if(data.snapshot!==v.snapshot||!Array.isArray(data.items)||data.items.length>256||!Number.isSafeInteger(data.matching)||data.matching>4096)throw new Error('Invalid captured call actions');actions.push(...data.items);if(data.next===null)return actions;if(!Number.isSafeInteger(data.next)||data.next<=offset)throw new Error('Invalid captured call continuation');offset=data.next;}throw new Error('Captured call-action allowance exceeded');}catch(error){if(signal?.aborted)throw error;return [];}
}
async function chooseCapturedCall(id,action){
 const r=mounted.get(id),workspace=model,source=frozenSource(model.view(id));if(!r||!source||!model.navigable(id)||r.historyLoading)return;
 editorCalls.manual();callMenu.hide();hideDestinations();r.popup.hidden=true;const epoch=r.navigation=(r.navigation??0)+1,signature=JSON.stringify(source),controller=new AbortController();r.historyController=controller;r.resolving=r.resolvingCaptured=true;updateFragmentStatus(r);
 const verify=()=>{if(model!==workspace||mounted.get(id)!==r||r.navigation!==epoch||!model.navigable(id)||JSON.stringify(frozenSource(model.view(id)))!==signature)throw new DOMException('Captured navigation superseded','AbortError');};
 const open=async(site,target)=>{
  verify();if(r.sourcePane.selected())return;r.resolving=r.resolvingCaptured=true;updateFragmentStatus(r);
  try{
   const result=await post('/api/captured-functions',{source,query:{kind:'open',call:site.id,target:target.id}},controller.signal);verify();
   const planned=Workspace.restore(model.serialize(),model.snapshot),next=planned.followCaptured(id,result.invocation,result.frame),view=planned.view(next),s=view.symbol.source,start=view.reading.section,query=new URLSearchParams({...sourceBinding(view),snapshot:view.snapshot,file:s.file,version:s.version,line_start:String(start),line_count:String(Math.min(96,s.end_line-start+1)),outline_count:'1',reflow:'true'});
   const [data,actions]=await Promise.all([start===result.frame.window.line_start?result.frame.window:transport(sourceEndpointForView(view)+'?'+query,{signal:controller.signal}),capturedActions(view,start,Math.min(96,s.end_line-start+1),controller.signal)]);verify();
   if(r.sourcePane.selected())throw new DOMException('Captured navigation deferred for selection','AbortError');
   const prepared=preparedComparison(view,{window:data,origin:result.frame.origin});prepared.actions=actions;
   const destination=model.followCaptured(id,result.invocation,result.frame);hideDestinations();mount(destination,prepared);focus([id,destination]);save();
  }catch(error){if(error.name!=='AbortError'&&model===workspace&&mounted.get(id)===r&&r.navigation===epoch)fail(error);}
  finally{if(mounted.get(id)===r&&r.navigation===epoch){r.resolving=false;updateFragmentStatus(r);live.resume();}}
 };
 try{
  const result=await post('/api/captured-functions',{source,query:{kind:'definition',call:action.id}},controller.signal);verify();
  const site=result.invocation;if(result.snapshot!==source.snapshot||site?.owner!==source.symbol||site.id!==action.id)throw new Error('Captured definition differs from the visible caller');
  const choice={site,attempted:!result.complete&&!site.binding,semantic:false,note:result.detail,captured:{open:target=>void open(site,target)}};const target=directDestination(choice);
  if(target)await open(site,target);else showDestinations(id,choice);
 }catch(error){if(error.name!=='AbortError'&&model===workspace&&mounted.get(id)===r&&r.navigation===epoch)fail(error);}
 finally{if(mounted.get(id)===r&&r.navigation===epoch){r.resolving=false;updateFragmentStatus(r);live.resume();}}
}
function comparisonControls(r,v){
 if(v.captured)r.element.dataset.capturedFile='true';else delete r.element.dataset.capturedFile;
 const c=v.comparison;r.compareButton.hidden=Boolean(v.symbol.origin)||Boolean(c&&!v.history);r.compareButton.textContent=c?'History':'Compare';r.compareButton.setAttribute('aria-label',c?'Continue function history':'Compare with previous version');r.compareButton.disabled=r.loading||model.views.length>=24&&!c||Boolean(c?.returning&&r.awaitingLive)||!v.history&&!model.usable(v.id);
 r.editPrevious.hidden=r.editNext.hidden=!c;r.editPrevious.disabled=r.loading||!c||c.returning||c.cursor===0;r.editNext.disabled=r.loading||!c||c.returning||c.cursor>=c.changes.length-1;
 r.historyPrevious.hidden=Boolean(c||v.captured);if(v.captured){r.historyNext.hidden=true;r.compareButton.hidden=true;r.historyLabel.textContent=shortRevision(v.captured.label);r.historyLabel.title=`Captured file · ${v.captured.label}`;}if(c){r.historyNext.hidden=true;r.historyLabel.textContent=`${comparisonTitle(c)} · ${c.changes.length?c.cursor+1:0}/${c.changes.length}`;r.historyLabel.title=`${c.label}; compared with ${c.other_label}`;r.element.dataset.comparisonSide=c.side;r.element.dataset.comparisonToken=c.token;}else{delete r.element.dataset.comparisonSide;delete r.element.dataset.comparisonToken;}
 const key=c?`${c.token}:${c.side}`:'';if(r.comparisonKey===key&&r.comparisonFirst===r.code.firstElementChild)return;
 for(const row of r.code.querySelectorAll('.code-line')){const number=Number(row.dataset.line),changed=Boolean(c&&c.changes.some(([start,end])=>number>=start&&number<end));row.classList.toggle('comparison-removed',changed&&c.side==='older');row.classList.toggle('comparison-added',changed&&c.side==='newer');if(changed)row.title=`${c.side==='older'?'Removed or replaced':'Added or replaced'} line ${number} · ${shortRevision(c.label)}`;else row.removeAttribute('title');}
 r.comparisonKey=key;r.comparisonFirst=r.code.firstElementChild;
}
function preparedComparison(v,frame){
 const s=v.symbol.source,data=frame.window,start=v.reading.section,origin=frame.origin;
 if(data?.snapshot!==v.snapshot||data.file!==s.file||data.version!==s.version||!Array.isArray(data.lines)||!data.lines.length||data.lines.length>96||data.lines[0].number!==start||origin?.number!==s.line||origin.left||origin.column!==0)throw new Error('Comparison source differs from the requested declaration');
 return {data,actions:[],start,margin:functionIndent(origin)};
}
async function compareFunction(id){
 const r=mounted.get(id),workspace=model,v=model.view(id);if(!r||r.loading||r.sourcePane.selected()||v.comparison)return;
 if(model.views.length>=24){notice.show('Close a bubble before opening a comparison.');return;}
 if(!v.history&&!model.usable(id)||v.symbol.origin)return;
 editorCalls.manual();callMenu.hide();hideDestinations();r.popup.hidden=true;r.navigation=(r.navigation??0)+1;r.resolving=r.warming=false;
 const binding=JSON.stringify([v.snapshot,v.symbol.id,v.history]),ticket=r.historyTicket=(r.historyTicket??0)+1,controller=new AbortController();r.historyController=controller;r.comparing=r.historyLoading=r.loading=true;geometry();
 try{
  const s=v.symbol.source,source=v.history?{token:v.history.token,index:v.history.index,snapshot:v.snapshot,file:s.file,version:s.version}:{snapshot:v.snapshot,file:s.file,version:s.version,tracking:v.symbol.tracking};
  const pair=await post('/api/function-comparisons',{kind:v.history?'historical':'current',source},controller.signal);
  if(model!==workspace||mounted.get(id)!==r||r.historyTicket!==ticket||JSON.stringify([model.view(id).snapshot,model.view(id).symbol.id,model.view(id).history])!==binding)throw new DOMException('Comparison superseded','AbortError');
  if(r.sourcePane.selected())throw new Error('Clear the source selection before comparing versions.');
  const planned=Workspace.restore(model.serialize(),model.snapshot),preview=planned.openComparison(id,pair),newer=preparedComparison(planned.view(id),pair.newer),older=preparedComparison(planned.view(preview.older),pair.older);
  const actionSets=await Promise.all([id,preview.older].map(viewId=>{const view=planned.view(viewId);return capturedActions(view,view.reading.section,Math.min(96,view.symbol.source.end_line-view.reading.section+1),controller.signal);}));
  if(model!==workspace||mounted.get(id)!==r||r.historyTicket!==ticket||JSON.stringify([model.view(id).snapshot,model.view(id).symbol.id,model.view(id).history])!==binding)throw new DOMException('Comparison superseded','AbortError');
  if(r.sourcePane.selected())throw new DOMException('Comparison deferred for selection','AbortError');newer.actions=actionSets[0];older.actions=actionSets[1];
  const opened=model.openComparison(id,pair);r.historyLoading=r.loading=r.comparing=false;r.sites=[];r.actions=[];r.callsReady=false;r.element.querySelector('.fragment-heading small').textContent=`${model.view(id).symbol.source.path}:${model.view(id).symbol.source.line}`;
  applyPreparedSource(id,newer);mount(opened.older,older);focus([id,opened.older]);save();
 }catch(error){if(error.name!=='AbortError'&&model===workspace&&mounted.get(id)===r&&r.historyTicket===ticket)fail(error);}
 finally{if(mounted.get(id)===r&&r.historyTicket===ticket){r.comparing=r.historyLoading=r.loading=false;geometry();live.resume();}}
}
async function comparisonEdit(id,direction){
 const workspace=model,origin=model.view(id),targets=model.views.filter(v=>v.comparison?.token===origin.comparison?.token&&!v.comparison?.returning),records=targets.map(v=>mounted.get(v.id));
 if(!origin.comparison||records.some(r=>!r||r.loading||r.sourcePane.selected()))return;
 const planned=Workspace.restore(model.serialize(),model.snapshot),ids=planned.comparisonEdit(id,direction),controller=new AbortController();
 const intent=JSON.stringify(targets.map(v=>[v.id,v.snapshot,v.comparison]));
 const tickets=records.map(r=>{r.historyController=controller;r.comparing=r.historyLoading=r.loading=true;return r.historyTicket=(r.historyTicket??0)+1;});geometry();
 try{
  const prepared=await Promise.all(ids.map(async viewId=>{const v=planned.view(viewId),s=v.symbol.source,start=v.reading.section,r=mounted.get(viewId),query=new URLSearchParams({...sourceBinding(v),snapshot:v.snapshot,file:s.file,version:s.version,line_start:String(start),line_count:String(Math.min(96,s.end_line-start+1)),outline_count:'1',reflow:'true'});const [data,actions]=await Promise.all([transport(sourceEndpointForView(v)+'?'+query,{signal:controller.signal}),capturedActions(v,start,Math.min(96,s.end_line-start+1),controller.signal)]);if(data.snapshot!==v.snapshot||data.file!==s.file||data.version!==s.version||!Array.isArray(data.lines)||!data.lines.length||data.lines.length>96||data.lines[0].number!==start)throw new Error('Comparison reading source differs');return {data,actions,start,margin:r.margin};}));
  if(model!==workspace||records.some((r,i)=>mounted.get(r.id)!==r||r.historyTicket!==tickets[i]||r.sourcePane.selected())||JSON.stringify(model.views.filter(v=>targets.some(t=>t.id===v.id)).map(v=>[v.id,v.snapshot,v.comparison]))!==intent)throw new DOMException('Comparison reading superseded','AbortError');
  model.comparisonEdit(id,direction);for(const [i,viewId]of ids.entries())applyPreparedSource(viewId,prepared[i]);save();
 }catch(error){if(error.name!=='AbortError'&&model===workspace&&records.every((r,i)=>mounted.get(r.id)===r&&r.historyTicket===tickets[i]))fail(error);}
 finally{for(const [i,r]of records.entries())if(mounted.get(r.id)===r&&r.historyTicket===tickets[i])r.comparing=r.historyLoading=r.loading=false;geometry();live.resume();}
}
function sourceStatus(r,text){r.sourceStatus=model?.view(r.id).symbol.origin&&text.startsWith('Live')?text.replace('Live','Captured dependency'):text;updateFragmentStatus(r);}
async function prepareDefinitions(id){await definitionCapabilities;const r=mounted.get(id);if(!definitionProvider||!r||!model.usable(id))return;if(definitionWarmup.start(model.snapshot,r.sites)){const pending=setTimeout(()=>{if(mounted.get(id)===r){r.warming=true;updateFragmentStatus(r);}},150);void definitionWarmup.wait().finally(()=>{clearTimeout(pending);if(mounted.get(id)===r){r.warming=false;updateFragmentStatus(r);}});}}
function flushSave(){clearTimeout(saveTimer);if(model&&taskControls)taskControls.save();}
function save(){clearTimeout(saveTimer);saveTimer=setTimeout(flushSave,120);}
window.addEventListener('pagehide',flushSave);
function size(){const r=el('workspace-canvas').getBoundingClientRect();return{w:r.width,h:r.height};}
function focus(ids){const s=size();model.reveal(ids,s.w,s.h);geometry();save();}
function overview(){
 const root=el('overview-map');root.replaceChildren();if(!model)return;
 const items=model.visibleItems();if(!items.length)return;
 const x=Math.min(...items.map(v=>v.x))-40,y=Math.min(...items.map(v=>v.y))-40,right=Math.max(...items.map(v=>v.x+v.w))+40,bottom=Math.max(...items.map(v=>v.y+(v.collapsed?54:v.h)))+40;
 root.setAttribute('viewBox',`${x} ${y} ${right-x} ${bottom-y}`);
 for(const v of items){
  const description=v.kind==='source'?`${[...v.symbol.parents,v.symbol.name].join('::')} · ${v.symbol.source.path}:${v.symbol.source.line}`:v.kind==='group'?v.title:`${v.query} of ${v.target.name}`;
  const r=svg('rect',{x:v.x,y:v.y,width:v.w,height:v.collapsed?54:v.h,rx:6,tabindex:0,'aria-label':`Return to ${description}`}),title=svg('title');
  const m=v.kind==='source'?modulePresentation(v,model.views):null;
  title.textContent=description+(m?' · '+m.label:'')+(v.symbol?.origin?' · '+originLabel(v.symbol.origin):'');
  if(m){r.dataset.moduleId=m.id;r.style.setProperty('--module-accent',`var(--module-${m.colour})`);r.style.setProperty('--module-tint',`var(--module-tint-${m.colour})`);}
  r.append(title);r.classList.add('overview-fragment');if(v.id===model.selected)r.classList.add('selected');
  const reveal=()=>{if(v.kind==='group')focus(model.group(v.id).members);else{if(v.kind==='source')model.selected=v.id;focus([v.id]);}};
  r.addEventListener('click',reveal);r.addEventListener('keydown',e=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();reveal();}});root.append(r);
 }
 const viewport=size(),c=model.camera;root.append(svg('rect',{x:c.x,y:c.y,width:viewport.w/c.scale,height:viewport.h/c.scale,class:'overview-camera'}));
}
function links(){
 const root=el('workspace-links');root.replaceChildren();if(!model)return;const defs=svg('defs'),marker=svg('marker',{id:'candidate-arrow',viewBox:'0 0 10 10',refX:9,refY:5,markerWidth:6,markerHeight:6,orient:'auto-start-reverse'});marker.append(svg('polygon',{points:'0,0 10,5 0,10',fill:'var(--link)'}));defs.append(marker);const hits=svg('g',{'aria-hidden':'true'}),strokes=svg('g');root.style.setProperty('--link-hit-width',20/model.camera.scale+'px');root.append(defs,hits,strokes);
 for(const l of model.links){if(!model.isVisible(l.from)||!model.isVisible(l.to))continue;const a=model.view(l.from),b=model.view(l.to),reverse=b.x+b.w<=a.x,direction=reverse?-1:1,x=reverse?a.x:a.x+a.w,y=a.y+28,tx=reverse?b.x+b.w:b.x,ty=b.y+28;let d;
  if(a.id===b.id)d=`M ${x} ${y} C ${x+90} ${y-100} ${a.x-90} ${y-100} ${a.x} ${y}`;
  else{const bend=Math.max(45,Math.abs(tx-x)/2);d=`M ${x} ${y} C ${x+direction*bend} ${y} ${tx-direction*bend} ${ty} ${tx} ${ty}`;}
  const path=svg('path',{d,'marker-end':'url(#candidate-arrow)',tabindex:0,role:'img'}),title=svg('title');path.classList.add('link-stroke');path.dataset.linkId=l.id;path.dataset.state=l.state??'current';path.dataset.kind=l.kind;path.classList.toggle('historical-call',l.state!=null&&l.state!=='current');path.classList.toggle('self-call',l.kind==='self');path.classList.toggle('definition-call',l.kind==='definition');
  const destination=[...b.symbol.parents,b.symbol.name].join('::'),detail=l.state==='captured'?`Captured file call (${l.kind==='self'?'source-local binding':'name candidate'}) · ${shortRevision(l.captured_label)}`:l.state==='removed'?'Call no longer present in current code':l.state==='unverified'?'Earlier call; current destination is unconfirmed':l.kind==='candidate'?'Destination chosen by matching the call name':'Resolved call';title.textContent=`${detail}: ${l.spelling} → ${destination} · ${l.source.path}:${l.source.line}`;path.setAttribute('aria-label',title.textContent);path.append(title);const hit=svg('path',{d,class:'link-hit','aria-hidden':'true'}),hitTitle=svg('title');hit.dataset.linkId=l.id;hitTitle.textContent=title.textContent;hit.append(hitTitle);hits.append(hit);strokes.append(path);
 }
}
function geometry(){
 if(!model)return;for(const[id,r]of mounted)if(!model.views.some(v=>v.id===id)){r.controller.abort();r.historyController?.abort();r.sourcePane.dispose();r.element.remove();mounted.delete(id);}const c=model.camera;el('workspace-world').style.transform=`translate(${-c.x*c.scale}px,${-c.y*c.scale}px) scale(${c.scale})`;el('visual-zoom').textContent=Math.round(c.scale*100)+'%';el('visual-empty').hidden=model.items().length>0;el('visual-stale').hidden=!model.views.some(v=>v.state==='unresolved');
 for(const v of model.views){const r=mounted.get(v.id);if(!r)continue;r.element.hidden=!model.isVisible(v.id);Object.assign(r.element.style,{left:v.x+'px',top:v.y+'px',width:v.w+'px',height:(v.collapsed?54:v.h)+'px'});const membership=modulePresentation(v,model.views);r.element.classList.toggle('has-module',Boolean(membership));const moduleKey=JSON.stringify(membership);if(moduleKey!==r.moduleKey){r.moduleKey=moduleKey;r.moduleTag.hidden=!membership;if(membership){r.element.dataset.moduleId=membership.id;r.moduleTag.dataset.moduleId=membership.id;r.moduleTag.textContent=membership.label;r.moduleTag.title=membership.title;r.element.style.setProperty('--module-accent',`var(--module-${membership.colour})`);r.element.style.setProperty('--module-tint',`var(--module-tint-${membership.colour})`);}else{delete r.element.dataset.moduleId;delete r.moduleTag.dataset.moduleId;r.moduleTag.textContent='';r.element.style.removeProperty('--module-accent');r.element.style.removeProperty('--module-tint');}}r.body.hidden=v.collapsed;r.collapse.textContent=v.collapsed?'▾':'▴';r.collapse.setAttribute('aria-label',v.collapsed?'Expand bubble':'Collapse bubble');r.element.classList.toggle('is-selected',v.id===model.selected);r.element.classList.toggle('is-stale',!model.usable(v.id));r.prev.disabled=r.next.disabled=Boolean(r.historyLoading);r.callsButton.disabled=!model.navigable(v.id)||r.historyLoading;r.referenceActions.disabled=!model.usable(v.id)||r.historyLoading;r.historyPrevious.disabled=r.loading||v.history?.returning&&r.awaitingLive||(!v.history&&!model.usable(v.id));r.historyNext.hidden=!v.history;r.historyNext.disabled=r.loading||v.history?.returning&&r.awaitingLive||!v.history||v.history.index===0;r.historyRow.hidden=Boolean(v.symbol.origin);r.historyLabel.textContent=v.history?v.history.commit.slice(0,8):'';r.historyLabel.title=v.history?`Commit ${v.history.commit}; first-parent history at ${v.history.tip}`:'';r.fitHeight.disabled=r.fitWidth.disabled=r.loading||!r.lines?.length||v.collapsed;inlineAvailability(r.code,v.id);r.keep.disabled=Boolean(v.symbol.origin)||!model.usable(v.id)||r.historyLoading||!board||busy;r.keep.title=v.symbol.origin?'Dependency locations stay in this workspace.':'Save location';r.editor.disabled=!model.usable(v.id)||r.historyLoading||busy;r.pin.disabled=r.historyLoading&&!v.history&&!v.comparison&&!v.captured;r.pin.textContent=v.history||v.comparison||v.captured?'Return to live':v.mode==='pinned'?'Follow live':'Pin version';r.pin.setAttribute('aria-label',v.history||v.comparison||v.captured?'Return to live':v.mode==='pinned'?'Follow live changes':'Pin version');comparisonControls(r,v);updateFragmentStatus(r);}
 if(model.stale)callMenu.hide();links();overview();outlines.refresh();highlights.refresh();taskGroups.refresh();referenceStacks.refresh();
}
function button(text,label,handler){const b=node('button',text);b.type='button';if(label)b.setAttribute('aria-label',label);b.onclick=()=>{try{handler();}catch(e){fail(e);}};return b;}
function bindDrag(handle,id,resizing=false){handle.addEventListener('pointerdown',event=>{if(event.button!==0||(!resizing&&event.target.closest('button')))return;event.preventDefault();model.selected=id;const v=model.view(id),start={x:event.clientX,y:event.clientY,vx:v.x,vy:v.y,w:v.w,h:v.h,scale:model.camera.scale};handle.setPointerCapture(event.pointerId);
 const move=e=>{const dx=(e.clientX-start.x)/start.scale,dy=(e.clientY-start.y)/start.scale;try{if(resizing)model.resize(id,Math.max(320,Math.min(BUBBLE_SIZE.maxWidth,start.w+dx)),Math.max(200,Math.min(BUBBLE_SIZE.maxHeight,start.h+dy)));else model.move(id,start.vx+dx,start.vy+dy);geometry();}catch(e){fail(e);}};
 const end=()=>{handle.removeEventListener('pointermove',move);handle.removeEventListener('pointerup',end);handle.removeEventListener('pointercancel',end);save();};handle.addEventListener('pointermove',move);handle.addEventListener('pointerup',end);handle.addEventListener('pointercancel',end);});}
function bindLayoutKeys(handle,id,resizing=false){handle.tabIndex=0;handle.addEventListener('keydown',e=>{if(e.target!==handle)return;const delta={ArrowLeft:[-16,0],ArrowRight:[16,0],ArrowUp:[0,-16],ArrowDown:[0,16]}[e.key];if(!delta)return;e.preventDefault();try{const v=model.view(id);if(resizing)model.resize(id,Math.max(320,Math.min(BUBBLE_SIZE.maxWidth,v.w+delta[0])),Math.max(200,Math.min(BUBBLE_SIZE.maxHeight,v.h+delta[1])));else model.move(id,v.x+delta[0],v.y+delta[1]);model.selected=id;geometry();save();}catch(e){fail(e);}});}
function fitCode(id,axis){const r=mounted.get(id),v=model.view(id),content=r.sourcePane.contentSize();if(!content)return;const extent=Math.ceil(axis==='width'?v.w-r.code.clientWidth+content.width:v.h-r.code.clientHeight+content.height);model.fitSize(id,axis,extent);geometry();save();if(extent>(axis==='width'?BUBBLE_SIZE.maxWidth:BUBBLE_SIZE.maxHeight))notice.show('Some code still needs scrolling at this size.');}
function mount(id,prepared){
 if(mounted.has(id)){if(prepared&&!mounted.get(id).code.querySelector('.code-line'))applyPreparedSource(id,prepared);return;}const v=model.view(id),element=node('section');element.className='fragment';element.dataset.view=id;element.dataset.name=v.symbol.name;element.setAttribute('aria-label',`${v.symbol.name} bubble`);
 const header=node('div');header.className='fragment-header';header.setAttribute('aria-label','Move bubble with arrow keys');bindLayoutKeys(header,id);const heading=node('div');heading.className='fragment-heading';const context=node('div');context.className='fragment-context';const moduleTag=node('span');moduleTag.className='module-tag';moduleTag.tabIndex=0;moduleTag.hidden=true;context.append(moduleTag,node('small',`${v.symbol.origin?originLabel(v.symbol.origin)+' · ':''}${v.symbol.source.path}:${v.symbol.source.line}`));heading.append(node('strong',[...v.symbol.parents,v.symbol.name].join('::')),context);
 const collapse=button('▴','Collapse bubble',()=>{callMenu.hide();model.collapse(id);geometry();save();}),close=button('×','Close bubble',()=>{editorCalls.manual();callMenu.hide();const r=mounted.get(id);r.controller.abort();r.historyController?.abort();r.sourcePane.dispose();element.remove();mounted.delete(id);model.close(id);hideDestinations();geometry();save();});const pin=button('Pin version','Pin version',()=>{if(model.view(id).history||model.view(id).comparison||model.view(id).captured){returnHistoryLive(id);return;}model.pin(id,model.view(id).mode!=='pinned');geometry();save();refreshLive();});const fitHeight=button('↕','Fit height',()=>fitCode(id,'height')),fitWidth=button('↔','Fit width',()=>fitCode(id,'width'));for(const b of [fitHeight,fitWidth]){b.className='bubble-fit';b.title=b.getAttribute('aria-label')+' to this code section';}heading.title=[...v.symbol.parents,v.symbol.name].join('::');const referenceActions=button('⋯',`Actions for ${v.symbol.name} bubble`,()=>callMenu.show(referenceActions,v.symbol.name,[{label:'Find callers',run:()=>openReferences(id,'callers')},{label:'Find references',run:()=>openReferences(id,'references')},{label:'Task group',run:()=>taskControls.showGroups(model.groupFor(id)?.id??'')}],false));header.append(heading,referenceActions,fitHeight,fitWidth,collapse,close);element.append(header);bindDrag(header,id);
 const body=node('div');body.className='fragment-body';const historyRow=node('div');historyRow.className='fragment-history';const historyPrevious=button('Previous change','Previous function change',()=>void changeHistory(id,1)),historyNext=button('Next change','Next function change',()=>void changeHistory(id,-1)),historyLabel=node('span');historyLabel.setAttribute('aria-live','polite');const compareButton=button('Compare','Compare with previous version',()=>model.view(id).comparison?void changeHistory(id,1):void compareFunction(id)),editPrevious=button('◂ edit','Previous comparison edit',()=>void comparisonEdit(id,-1)),editNext=button('edit ▸','Next comparison edit',()=>void comparisonEdit(id,1));historyRow.append(historyPrevious,editPrevious,historyLabel,historyNext,editNext,compareButton);const section=node('div');section.className='source-section';const sectionLabel=node('span','Loading captured source…');const prev=button('Previous section',null,()=>{const v=model.view(id);void loadSource(id,Math.max(v.symbol.source.line,v.reading.section-96));}),next=button('Next section',null,()=>{const v=model.view(id);void loadSource(id,Math.min(v.symbol.source.end_line,v.reading.section+96));});prev.hidden=true;next.hidden=true;section.append(sectionLabel,prev,next);
 const code=node('div');code.className='code-pane';code.tabIndex=0;code.setAttribute('aria-label',`${v.symbol.name} captured code`);code.append(node('p','Loading source…'));const footer=node('div');footer.className='fragment-footer';const callsButton=button('Calls','Calls',()=>toggleCalls(id)),state=node('span','Captured source');state.setAttribute('role','status');const keep=button('Save','Save location',()=>void keepLocation(id)),editor=button('Editor',`Open in ${editorName}`,()=>void openEditor(id));editor.disabled=true;footer.append(callsButton,state,pin,keep,editor);
 const popup=node('div');popup.className='calls-popover';popup.hidden=true;body.append(historyRow,section,code,popup,footer);const resize=button('','Resize bubble',()=>{});resize.title='Drag or use arrow keys to resize';resize.className='resize-handle';body.append(resize);bindDrag(resize,id,true);bindLayoutKeys(resize,id,true);element.append(body);el('fragment-layer').append(element);
 const kept=board?.evidence.find(e=>e.source.file===v.symbol.source.file&&e.source.version===v.symbol.source.version&&e.source.line===v.symbol.source.line)?.id??null;
 const record={id,compareButton,editPrevious,editNext,historyRow,historyPrevious,historyNext,historyLabel,referenceActions,moduleTag,pin,fitHeight,fitWidth,loading:true,element,body,code,collapse,sectionLabel,prev,next,callsButton,popup,state,keep,editor,sites:[],nextCalls:0,matching:0,callsReady:false,controller:new AbortController(),kept};record.actions=[];record.sourcePane=new SourcePane(code,(text,line,plan,margin)=>{renderCallText(text,line,record.actions,(action,button,direct,keyboard)=>inlineAction(id,action,button,direct,keyboard),plan,margin);inlineAvailability(text,id);});record.sourcePane.setReflow(smartReflow);mounted.set(id,record);
 element.addEventListener('pointerdown',()=>{model.selected=id;geometry();});code.addEventListener('scroll',()=>{callMenu.hide();if(mounted.get(id)!==record)return;const v=model.view(id);model.read(id,{section:v.reading.section,...record.sourcePane.reading()});save();});
 jobs=jobs.then(async()=>{if(mounted.get(id)!==record)return;if(!record.code.querySelector('.code-line'))await loadSource(id,v.reading.section);if(model.navigable(id))await loadCalls(id,0);}).catch(fail);geometry();if(prepared)applyPreparedSource(id,prepared);
}
async function prepareFragmentSource(symbol,reading,signal,snapshot) {
 const s=symbol.source,start=reading.section,expected=captureFor(symbol,snapshot),endpoint=symbol.origin?'/api/dependency/window':'/api/window';
 const count=Math.min(96,s.end_line-start+1),query=new URLSearchParams({snapshot:expected,file:s.file,version:s.version,line_start:String(start),line_count:String(count),outline_count:'1',reflow:'true'});
 const header=start===s.line?Promise.resolve(null):transport(endpoint+'?'+new URLSearchParams({snapshot:expected,file:s.file,version:s.version,line_start:String(s.line),line_count:'1',outline_count:'1'}),{signal});
 const [data,origin,actions]=await Promise.all([transport(endpoint+'?'+query,{signal}),header,requestActions(symbol,start,count,snapshot,signal)]);
 if(data.snapshot!==expected||data.file!==s.file||data.version!==s.version||!Array.isArray(data.lines)||!data.lines.length||data.lines.length>96)throw new Error('Captured source for the requested function is unavailable');
 if(origin&&(origin.snapshot!==expected||origin.file!==s.file||origin.version!==s.version||origin.lines?.length!==1||origin.lines[0].number!==s.line||origin.lines[0].left||origin.lines[0].column!==0))throw new Error('Fragment indentation source differs');
 return {data,actions,start,margin:functionIndent(origin?.lines[0]??data.lines.find(line=>line.number===s.line))};
}
function applyPreparedSource(id,prepared) {
 const r=mounted.get(id),v=model.view(id),s=v.symbol.source,{data,actions,start,margin}=prepared;
 if(data.file!==s.file||data.version!==s.version)throw new Error('Prepared source differs from the requested fragment');
 r.sourceTicket=(r.sourceTicket??0)+1;r.loading=false;r.margin=margin;r.lines=data.lines.filter(line=>line.number<=s.end_line);r.actions=actions;
 r.sourcePane.set(r.lines,{...v.reading},margin);inlineAvailability(r.code,id);
 const last=Math.min(s.end_line,data.lines.at(-1)?.number??start);r.sectionLabel.textContent=`Lines ${start}–${last} · ${s.end_line-s.line+1}-line function`;r.prev.hidden=start<=s.line;r.next.hidden=last>=s.end_line;
 sourceStatus(r,data.coloring==='ready'?'Live':'Styling '+data.coloring);model.read(id,{section:start,...r.sourcePane.reading()});geometry();
}
async function requestActions(symbol,start,count,snapshot,signal) {
 const actions=[];let offset=0;
 try{for(let page=0;page<16;page++){
  const data=await post('/api/structure',{snapshot,query:{kind:'actions',symbol:symbol.id,line_start:start,line_count:count,offset}},signal);
  if(data.snapshot!==snapshot||!Array.isArray(data.items)||data.items.length>256||!Number.isSafeInteger(data.matching)||data.matching>4096)throw new Error('Invalid captured call actions');
  actions.push(...data.items);if(data.next===null)return actions;if(!Number.isSafeInteger(data.next)||data.next<=offset)throw new Error('Invalid call-action continuation');offset=data.next;
 }throw new Error('Call-action allowance exceeded');}catch(error){if(signal?.aborted)throw error;return [];}
}
async function loadSource(id,start){
 callMenu.hide();const r=mounted.get(id);if(!r)return;const ticket=r.sourceTicket=(r.sourceTicket??0)+1,v=model.view(id),s=v.symbol.source;const expected=captureFor(v.symbol,v.snapshot),sourceEndpoint=sourceEndpointForView(v);const saved={...v.reading};r.loading=true;
 try{const query=new URLSearchParams({...sourceBinding(v),snapshot:expected,file:s.file,version:s.version,line_start:String(start),line_count:String(Math.min(96,s.end_line-start+1)),outline_count:'1',reflow:'true'});const header=r.margin===undefined&&start!==s.line?transport(sourceEndpoint+'?'+new URLSearchParams({...sourceBinding(v),snapshot:expected,file:s.file,version:s.version,line_start:String(s.line),line_count:'1',outline_count:'1'}),{signal:r.controller.signal}):Promise.resolve(null);const [data,actions,origin]=await Promise.all([transport(sourceEndpoint+'?'+query,{signal:r.controller.signal}),frozenSource(v)?capturedActions(v,start,Math.min(96,s.end_line-start+1),r.controller.signal):!model.usable(id)?Promise.resolve([]):loadActions(id,start,Math.min(96,s.end_line-start+1),model.snapshot,r),header]);if(mounted.get(id)!==r||r.sourceTicket!==ticket)return;
  if(data.snapshot!==expected||data.file!==s.file||data.version!==s.version||!Array.isArray(data.lines)||data.lines.length>96)throw new Error('Captured source version differs');r.code.replaceChildren();
  if(origin&&(origin.snapshot!==expected||origin.file!==s.file||origin.version!==s.version||origin.lines?.length!==1||origin.lines[0].number!==s.line||origin.lines[0].left||origin.lines[0].column!==0))throw new Error('Fragment indentation source differs');
  if(r.margin===undefined)r.margin=functionIndent(origin?.lines[0]??data.lines.find(line=>line.number===s.line));
  r.lines=data.lines.filter(line=>line.number<=s.end_line);r.actions=actions;r.sourcePane.set(data.lines.filter(line=>line.number<=s.end_line),{top:start===saved.section?saved.top:0,left:saved.left,anchor:start===saved.section?saved.anchor:null},r.margin);
  inlineAvailability(r.code,v.id);const last=Math.min(s.end_line,data.lines.at(-1)?.number??start);r.sectionLabel.textContent=`Lines ${start}–${last} · ${s.end_line-s.line+1}-line function`;r.prev.hidden=start<=s.line;r.next.hidden=last>=s.end_line;sourceStatus(r,data.coloring==='ready'?'Live':'Styling '+data.coloring);
  model.read(id,{section:start,...r.sourcePane.reading()});save();
 }catch(e){if(e.name==='AbortError')return;if(mounted.get(id)===r&&r.sourceTicket===ticket){if(v.history||v.comparison||v.captured){notice.show(e.message);if(r.lines?.length)return;}r.sourcePane.set([]);r.code.replaceChildren(node('p','Source for this bubble is unavailable.'));r.sectionLabel.textContent='Source unavailable';sourceStatus(r,e.message);}}finally{r.loading=false;geometry();live.resume();}
}
async function loadActions(id,start,count,snapshot,record){
 const actions=[];let offset=0;
 try{for(let page=0;page<16;page++){
  const data=await post('/api/structure',{snapshot,query:{kind:'actions',symbol:model.view(id).symbol.id,line_start:start,line_count:count,offset}},record.controller.signal);
  if(data.snapshot!==snapshot||!Array.isArray(data.items)||data.items.length>256||!Number.isSafeInteger(data.matching)||data.matching>4096)throw new Error('Invalid captured call actions');
  actions.push(...data.items);if(data.next===null)return actions;if(!Number.isSafeInteger(data.next)||data.next<=offset)throw new Error('Invalid call-action continuation');offset=data.next;
 }throw new Error('Call-action allowance exceeded');}catch(e){if(e.name!=='AbortError'&&e.status!==409&&mounted.get(id)===record)notice.show('Inline call actions are unavailable for this section. Calls remains available.');return [];}
}
async function loadCalls(id,offset){
 const r=mounted.get(id);if(!r||!model.navigable(id))return;const v=model.view(id),source=frozenSource(v),expected=source?.snapshot??model.snapshot,binding=JSON.stringify([v.snapshot,v.symbol.id,source]);
 try{const data=source?await post('/api/captured-functions',{source,query:{kind:'calls',offset}},r.controller.signal):await post('/api/structure',{snapshot:expected,query:{kind:'calls',symbol:v.symbol.id,area:v.area||v.symbol.source.path,offset}},r.controller.signal);if(mounted.get(id)!==r||!model.navigable(id)||JSON.stringify([model.view(id).snapshot,model.view(id).symbol.id,frozenSource(model.view(id))])!==binding)return;if(data.snapshot!==expected||!Array.isArray(data.items)||data.items.length>8)throw new Error('Call response differs');r.sites=offset===0?data.items:[...r.sites,...data.items];r.nextCalls=data.next;r.matching=data.matching;r.callsReady=true;renderCalls(id);if(!source)void prepareDefinitions(id);}catch(error){if(error.name!=='AbortError'){r.callsReady=true;r.popup.replaceChildren(node('p',error.message));}}
}
function renderCalls(id){const r=mounted.get(id);if(!r)return;r.popup.replaceChildren(node('p',`${r.sites.length} of ${r.matching} written calls.`));for(const site of r.sites)r.popup.append(button(`${site.name??'Dynamic call'} · line ${site.source.line}`,null,()=>chooseCall(id,site)));if(r.nextCalls!==null&&r.nextCalls!==undefined&&r.sites.length<64)r.popup.append(button('More written calls',null,()=>void loadCalls(id,r.nextCalls)));if(!r.matching)r.popup.append(node('p','No attributed calls recorded. This does not prove no runtime calls.'));}
function toggleCalls(id){editorCalls.manual();const r=mounted.get(id);if(!r||!model.navigable(id))return;hideDestinations();for(const[other,x]of mounted)if(other!==id)x.popup.hidden=true;r.popup.hidden=!r.popup.hidden;if(!r.callsReady){r.popup.replaceChildren(node('p','Loading written calls…'));void loadCalls(id,0);}else renderCalls(id);}
function hideDestinations(){el('destination-stack').hidden=true;}
async function chooseCall(id,site){
 editorCalls.manual();const record=mounted.get(id);if(!record||!model.navigable(id))return;if(frozenSource(model.view(id))){void chooseCapturedCall(id,site);return;}record.resolvingCaptured=false;
 site={...site,owner:model.view(id).symbol.id};
 const ticket=record.navigation=(record.navigation??0)+1,expected=model.snapshot;
 const verify=()=>{if(mounted.get(id)!==record||record.navigation!==ticket||model.snapshot!==expected||!model.usable(id))throw new DOMException('Call navigation superseded','AbortError');};
 await definitionCapabilities;
 let choice={site,attempted:definitionProvider&&!site.binding,note:'',semantic:false};
 record.popup.hidden=true;const pending=setTimeout(()=>{if(mounted.get(id)===record&&record.navigation===ticket){record.resolving=true;updateFragmentStatus(record);}},150);
 try {choice=await resolveCallChoice(site,{snapshot:expected,provider:definitionProvider,warmup:()=>definitionWarmup.wait(),request:(snapshot,call,signal)=>post('/api/call-definition',{snapshot,call},signal),signal:record.controller.signal,verify});}
 catch(error){if(error.name==='AbortError')return;if(error.status===409||site.lookup){fail(error);return;}choice.note=error.message;}
 finally{clearTimeout(pending);if(mounted.get(id)===record&&record.navigation===ticket){record.resolving=false;updateFragmentStatus(record);}}
 try{verify();}catch{return;}
 highlightCall(id,choice.site);const target=directDestination(choice);if(target){follow(id,choice.site,target);return;}showDestinations(id,choice);
}
function highlightCall(id,site){const r=mounted.get(id);r.popup.hidden=true;for(const line of r.code.querySelectorAll('.is-call'))line.classList.remove('is-call');r.code.querySelector(`[data-line="${site.source.line}"]`)?.classList.add('is-call');}
function showDestinations(id,{site,note,semantic,captured}) {
 highlightCall(id,site);const candidates=site.candidates,stack=el('destination-stack'),v=model.view(id);
 stack.replaceChildren(node('h2',site.spelling),node('p',candidates.length?`Choose among ${site.candidate_count} ${semantic?'definition locations':'terminal-name matches'}. ${candidates.length} captured destinations are shown.${note?' '+note:''}`:note||'No indexed destination. Dynamic or external calls remain unresolved.'));stack.append(button('Close choices',null,hideDestinations));
 for(const candidate of candidates){const b=button([...candidate.parents,candidate.name].join('::'),null,()=>captured?captured.open(candidate):follow(id,site,candidate));b.dataset.candidate=candidate.id;b.dataset.path=candidate.source.path;b.append(node('small',`${candidate.origin?originLabel(candidate.origin)+' · ':''}${candidate.source.path}:${candidate.source.line}`));stack.append(b);}Object.assign(stack.style,{left:v.x+v.w+24+'px',top:v.y+'px'});stack.hidden=false;
}
function follow(parent,site,target){editorCalls.manual();try{const id=model.follow(parent,site,target);hideDestinations();const record=mounted.get(id);mount(id);if(target.origin&&record&&(record.element.classList.contains('is-stale')||!record.lines?.length)){void loadSource(id,model.view(id).reading.section);record.callsReady=false;void loadCalls(id,0);}focus([parent,id]);save();}catch(e){fail(e);}}
async function keepLocation(id){
 const r=mounted.get(id),v=model.view(id);if(!r||!board||busy||!model.usable(id))return null;const existing=board.evidence.find(e=>e.source.file===v.symbol.source.file&&e.source.version===v.symbol.source.version&&e.source.line===v.symbol.source.line);if(existing){r.kept=existing.id;geometry();return existing.id;}
 busy=true;geometry();try{const s=v.symbol.source,result=await post('/api/search/evidence',{revision:board.revision,snapshot:model.snapshot,file:s.file,version:s.version,line:s.line,title:v.symbol.name});board=result.view.board;r.kept=result.id;notice.show('Location saved.');return r.kept;}catch(e){fail(e);return null;}finally{busy=false;geometry();live.resume();}
}
async function openEditor(id){const r=mounted.get(id);if(!r||busy||!model.usable(id))return;const s=model.view(id).symbol.source;busy=true;geometry();try{await post(model.view(id).symbol.origin?'/api/dependency/editor':'/api/editor/open',{snapshot:model.view(id).symbol.origin?.capture??model.snapshot,file:s.file,version:s.version,line:s.line});}catch(e){fail(e);}finally{busy=false;geometry();live.resume();}}
function openReferences(id,query){editorCalls.manual();try{if(!model.usable(id))throw new Error('Open a current function before finding references.');const stack=model.addReferences(model.view(id).symbol,query,id);geometry();focus([id,stack]);save();}catch(e){fail(e);}}
function replaceTask(next){
 editorCalls.manual();callMenu.hide();hideDestinations();clearTimeout(livePendingTimer);
 for(const r of mounted.values()){r.controller.abort();r.historyController?.abort();r.sourcePane.dispose();r.element.remove();}
 mounted.clear();referenceStacks.reset();jobs=Promise.resolve();model=next;
 for(const v of model.views)mount(v.id);
 geometry();live.changed(currentSnapshot,true);save();
}
function receive(status){
 el('visual-source-status').textContent=status.snapshot?'Source ready':status.error?'Source unavailable':'Mapping source…';
 if(!status.snapshot)return;currentSnapshot=status.snapshot;
 if(!model){
  try{model=taskControls.load(status.name??'source',currentSnapshot);}catch(e){model=new Workspace(currentSnapshot);fail(e);}
  for(const v of model.views)mount(v.id);geometry();save();
  if(new URLSearchParams(location.search).get('demo')==='minitosis'&&!model.items().length&&!demoOpened){demoOpened=true;void openMinitosis();}
 }else{model.markSnapshot(currentSnapshot);geometry();}
 live.changed(currentSnapshot,model.views.some(v=>v.mode==='live'&&(!v.symbol.tracking||!Object.hasOwn(v.symbol,'module'))),!model.stale&&model.snapshot===currentSnapshot&&model.views.every(v=>v.mode==='pinned'||v.snapshot===currentSnapshot&&v.state==='current'&&v.symbol.tracking&&Object.hasOwn(v.symbol,'module')));
 void editorCalls.resume();
}
async function findFragments(offset=0){const id=++paletteTicket;try{if(!model||model.stale)throw new Error('Wait for current source or start a new workspace.');const data=await post('/api/structure',{snapshot:model.snapshot,query:{kind:'inventory',area:'',search:el('fragment-search').value.trim(),offset,functions_only:true}});if(id!==paletteTicket)return;el('fragment-results').replaceChildren();el('fragment-search-status').textContent=`${data.matching} matching functions; choose a function.`;
 for(const entry of data.items){const s=entry.symbol;if(s.kind!=='function')continue;const b=button([...s.parents,s.name].join('::'),null,()=>{editorCalls.manual();const id=model.openRoot(s,s.source.path.split('/').slice(0,-1).join('/')||s.source.path);mount(id);focus([id]);save();el('fragment-dialog').close();});b.dataset.addSymbol=s.id;b.dataset.name=s.name;b.append(node('small',`${s.source.path}:${s.source.line}`));el('fragment-results').append(b);}el('more-fragment-results').hidden=data.next===null;el('more-fragment-results').dataset.offset=String(data.next??0);
 }catch(e){fail(e);}}
async function openMinitosis(){try{if(!model||model.stale)throw new Error('Wait for current source.');const data=await post('/api/structure',{snapshot:model.snapshot,query:{kind:'inventory',area:'minitosis/src/lib.rs',search:'process',offset:0}});const s=data.items.map(i=>i.symbol).find(s=>s.name==='process'&&s.kind==='function');if(!s)throw new Error('Minitosis example is not in this source workspace. Use Add bubble.');const id=model.openRoot(s,'minitosis/src');mount(id);focus([id]);save();}catch(e){fail(e);}}
const openDialog=()=>{editorCalls.manual();el('fragment-dialog').showModal();el('fragment-search').focus();};el('add-fragment').onclick=openDialog;el('empty-add').onclick=openDialog;el('close-fragment-dialog').onclick=()=>el('fragment-dialog').close();el('fragment-form').onsubmit=e=>{e.preventDefault();void findFragments();};el('more-fragment-results').onclick=()=>void findFragments(Number(el('more-fragment-results').dataset.offset));
el('fit-workspace').onclick=()=>{if(!model)return;const s=size();model.fit(s.w,s.h);geometry();save();};for(const[id,factor]of[['zoom-in',1.2],['zoom-out',1/1.2]])el(id).onclick=()=>{if(!model)return;const s=size();model.zoomAt(model.camera.scale*factor,{x:s.w/2,y:s.h/2});geometry();save();};
const canvas=el('workspace-canvas');canvas.addEventListener('pointerdown',e=>{if(e.button!==0||e.target.closest('.fragment,.reference-stack,.task-group-header,#destination-stack,#workspace-overview,#visual-empty,[data-link-id]'))return;if(!model)return;e.preventDefault();canvas.setPointerCapture(e.pointerId);let last={x:e.clientX,y:e.clientY};const move=event=>{model.pan(event.clientX-last.x,event.clientY-last.y);last={x:event.clientX,y:event.clientY};geometry();};const end=()=>{canvas.removeEventListener('pointermove',move);canvas.removeEventListener('pointerup',end);canvas.removeEventListener('pointercancel',end);save();};canvas.addEventListener('pointermove',move);canvas.addEventListener('pointerup',end);canvas.addEventListener('pointercancel',end);});
canvas.addEventListener('wheel',e=>{callMenu.hide();if(!model||e.target.closest('.code-pane,.reference-body,.calls-popover,#destination-stack'))return;e.preventDefault();if(e.ctrlKey||e.metaKey){const rect=canvas.getBoundingClientRect();model.zoomAt(model.camera.scale*Math.exp(-e.deltaY/500),{x:e.clientX-rect.left,y:e.clientY-rect.top});}else model.pan(-e.deltaX,-e.deltaY);geometry();save();},{passive:false});canvas.addEventListener('keydown',e=>{if(e.target!==canvas||!model)return;const moves={ArrowLeft:[70,0],ArrowRight:[-70,0],ArrowUp:[0,70],ArrowDown:[0,-70]};if(moves[e.key]){e.preventDefault();model.pan(...moves[e.key]);geometry();save();}if(e.key==='Escape')hideDestinations();});new ResizeObserver(()=>{geometry();void editorCalls.resume();}).observe(canvas);
const referenceStacks=installReferenceStacks(el('reference-layer'),{read:()=>model,post,transport,changed:()=>{geometry();save();},failed:fail,open:(id,row,data)=>{editorCalls.manual();try{if(data.snapshot!==model.snapshot||model.stale)throw new Error('Source changed. Refresh references.');const opened=model.openReference(id,row,data.target);for(const v of model.views)mount(v.id);focus([...(opened.destination?[opened.destination]:[]),opened.caller]);save();}catch(e){fail(e);}}});
const taskGroups=installTaskGroups(el('task-group-layer'),{read:()=>model,changed:()=>{geometry();save();},edit:id=>taskControls.showGroups(id),failed:fail});
taskControls=installTaskControls({read:()=>model,replace:replaceTask,snapshot:()=>currentSnapshot,changed:()=>{geometry();save();},failed:fail});
if (location.pathname === '/' && location.hash === '#saved') {
 location.replace('/investigate#saved');
} else {
 void transport('/api/investigation').then(view=>{board=view.board;editorName=view.editor;for(const[id,r]of mounted){r.editor.setAttribute('aria-label',`Open in ${editorName}`);r.editor.title=`Open in ${editorName}`;const v=model.view(id),kept=board.evidence.find(e=>e.source.file===v.symbol.source.file&&e.source.version===v.symbol.source.version&&e.source.line===v.symbol.source.line);r.kept=kept?.id??null;}geometry();}).catch(fail);
 followChanges({session:()=>({receive(change){receive(change.status);return true;},close(e){if(e)notice.show('Source updates disconnected; the visual layout is retained.');}})});
 void post('/api/workspace/start',{}).catch(fail);
}
