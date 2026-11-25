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
import {renderCallText,createCallMenu,retargetCalls,requestSourceActions} from '/call-actions.js';
import {installAppearance} from '/appearance.js';
import {createMessage} from '/messages.js';
import {followChanges} from '/updates.js';
import {installWorkspaceDelivery} from '/editor-delivery.js';
import {createCallPreparation,resolveCallChoice,directDestination} from '/call-delivery.js';
installAppHeader(document.querySelector('body > header'));
const el=id=>document.getElementById(id),node=(tag,text='')=>{const n=document.createElement(tag);n.textContent=text;return n;},svg=(tag,attrs={})=>{const n=document.createElementNS('http://www.w3.org/2000/svg',tag);for(const[k,v]of Object.entries(attrs))n.setAttribute(k,String(v));return n;};
const error=createMessage(el('visual-error'),{label:'Dismiss workspace error'}),notice=createMessage(el('visual-notice'),{label:'Dismiss workspace notice'});
let definitionProvider=false,javaImportAvailable=false;
const definitionCapabilities=transport('/api/structure').then(c=>{
 definitionProvider=c.definition_provider===true;
 javaImportAvailable=c.java_project_import===true;
}).catch(()=>{});
let model,currentSnapshot,taskControls,board,editorName='editor',editorJavaArchives=false,busy=false,saveTimer,paletteTicket=0,jobs=Promise.resolve(),demoOpened=false,livePendingTimer;
let revisionPicker,lineagePicker,crossFilePicker;
const mounted=new Map();
const returnPending=r=>r.liveReturn==='pending';
const SECTION_ROWS=96,SECTION_LIMIT=4,SECTION_EDGE=240;
const highlights=installLinkHighlights(el('workspace-canvas'),()=>({views:model?.views??[],links:model?.links??[]}));
const outlines=installModuleOutlines(el('module-outlines'),()=>model?model.visibleItems():[],{failed:()=>notice.show('Module outlines are unavailable for this layout.')});
let showModuleOutlines=false;try{showModuleOutlines=localStorage.getItem('vizzi.visual.module-outlines')==='true';}catch{}
el('module-outlines-setting').checked=showModuleOutlines;outlines.setEnabled(showModuleOutlines);el('module-outlines-setting').onchange=()=>{showModuleOutlines=el('module-outlines-setting').checked;outlines.setEnabled(showModuleOutlines);try{localStorage.setItem('vizzi.visual.module-outlines',String(showModuleOutlines));}catch{notice.show('The module outline preference could not be saved.');}};
const callMenu=createCallMenu(el('call-action-menu'));
function inlineAction(id,action,element,direct,keyboard){
 if(!model.navigable(id)||mounted.get(id)?.historyLoading)return;
 editorCalls.manual();callMenu.hide();
 const view=model.view(id),open=()=>{if(mounted.has(id)&&model.navigable(id))void chooseCall(id,{...action,lookup:true},element);};
 if(direct){open();return;}
 const actions=[{label:frozenSource(view)?'Open in captured file':'Open definition',hint:'Ctrl/⌘-click',run:open}];
 if(canLookInFile(view,action))actions.push({label:'Look in another file…',run:()=>void chooseCapturedCall(id,action,{anotherFile:true,opener:element})});
 callMenu.show(element,action.name,actions,keyboard);
}
function inlineAvailability(code,id){for(const name of code.querySelectorAll('.code-call')){name.setAttribute('aria-disabled',String(!model.navigable(id)||Boolean(mounted.get(id)?.historyLoading)));name.tabIndex=model.navigable(id)&&!mounted.get(id)?.historyLoading?0:-1;if(frozenSource(model.view(id)))name.title='Captured-file call · Ctrl/⌘-click opens a same-file destination';}}
installAppearance(el('theme'));
let smartReflow=true;try{smartReflow=localStorage.getItem('vizzi.visual.code-layout')!=='original';}catch{}
el('code-layout').value=smartReflow?'smart':'original';el('code-layout').onchange=()=>{smartReflow=el('code-layout').value==='smart';for(const r of mounted.values())r.sourcePane.setReflow(smartReflow);try{localStorage.setItem('vizzi.visual.code-layout',smartReflow?'smart':'original');}catch{notice.show('This browser could not save the code layout preference.');}};
const fail=e=>error.show(e?.message??String(e));
async function transport(path,options){const response=await fetchRead(path,options);if(!response.ok){let text=await response.text();try{text=JSON.parse(text).message??text;}catch{}const error=new Error(text);error.status=response.status;throw error;}return response.json();}
const post=(path,body,signal)=>transport(path,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body),signal});
const javaImportButton=el('import-java-project');
let javaProject;
async function updateJavaProject(snapshot){
 await definitionCapabilities;
 if(!javaImportAvailable||currentSnapshot!==snapshot)return;
 try{
  const project=await transport('/api/java-project');
  if(currentSnapshot!==snapshot)return;
  javaProject=project?.snapshot===snapshot&&['Maven','Gradle'].includes(project.kind)?project:null;
  javaImportButton.hidden=!javaProject;
  if(javaProject)javaImportButton.textContent=`Import ${javaProject.kind} classpath`;
 }catch{}
}
javaImportButton.onclick=async()=>{
 const project=javaProject;
 if(!project||project.snapshot!==currentSnapshot||javaImportButton.disabled)return;
 javaImportButton.disabled=true;
 el('workspace-actions').open=false;
 notice.show(`Importing Java classpath with ${project.kind}…`);
 try{
  await post('/api/java-project',{snapshot:project.snapshot});
  notice.show('Java project classpath imported.');
 }catch(e){notice.show(null);fail(e);}
 finally{javaImportButton.disabled=false;}
};
const definitionWarmup=new DefinitionWarmup((snapshot,call,path)=>post('/api/call-definition',{snapshot,call,path}));
function refreshCapture(){
 const views=model.views.filter(v=>v.mode==='live');const key=JSON.stringify([views.map(v=>[v.id,v.symbol.id,v.mode,mounted.get(v.id)?.sourceTicket,v.reading]),model.links.map(l=>[l.id,l.state])]);
 return {key,body:{views:views.map(v=>{const r=mounted.get(v.id),context=r?.sourcePane.readingContext(),reading=v.reading,a=reading.anchor;
  const tracked=v.history?.lineage?.length?v.history.base.symbol:v.symbol;
  return {id:v.id,tracking:tracked.tracking??null,symbol:tracked.id,file:tracked.source.file,context:tracked.source.context,capture:tracked.origin?.capture??null,version:r?.lines?.length?v.symbol.source.version:'0'.repeat(64),area:v.area,section_offset:reading.section-v.symbol.source.line,anchor:a?{offset:a.line-v.symbol.source.line,byte:a.byte,text:context?.reading.anchor?.line===a.line?context.text:''}:null};}),links:model.links.map(l=>({id:l.id,from:l.from,to:l.to,spelling:l.spelling,kind:l.kind}))}};
}
async function requestLiveRefresh(body,signal){
 const data=await post('/api/visual/refresh',body,signal);
 if(!Array.isArray(data.views)||data.views.length>24)throw new Error('Invalid live source updates');
 const neighbouringActions=new Map();
 for(const update of data.views??[]){
  if(update.state!=='current'||update.window)continue;
  const sections=mounted.get(update.id)?.sections??[];
  const actions=new Map([[update.position.section,update.actions]]);
  for(const section of sections){
   if(!actions.has(section.start))actions.set(section.start,await requestActions(update.symbol,section.start,section.lines.length,data.snapshot,signal));
  }
  neighbouringActions.set(update.id,actions);
 }
 return {...data,neighbouringActions};
}
function applyLive(data){
 if(data.snapshot!==currentSnapshot||!Array.isArray(data.views)||data.views.length>24||!Array.isArray(data.links)||data.links.length>96)throw new Error('Live observation differs');
 let focusAfterReturn;
 for(const update of data.views){const r=mounted.get(update.id);if(!r)throw new Error('Live view was closed');if(update.state!=='current')continue;
  const previous=model.view(update.id),s=update.symbol?.source,w=update.window;
  if(document.activeElement===r.historyNext&&newestHistory(previous))focusAfterReturn=r;
  if(!s||!Array.isArray(update.actions)||update.actions.length>4096||!Array.isArray(update.calls?.items)||update.calls.items.length>8||update.calls.snapshot!==data.snapshot)throw new Error('Invalid live call metadata');
  if(w&&(w.file!==s.file||w.version!==s.version||!Array.isArray(w.lines)||w.lines.length>96||update.origin?.number!==s.line||update.origin.left||update.origin.column!==0))throw new Error('Invalid live source window');
  if(!w&&(!r.lines?.length||previous.symbol.source.version!==s.version))throw new Error('Live source window is missing');
 }
 const publicationChanged=model.snapshot!==data.snapshot;clearTimeout(livePendingTimer);model.refresh(data.snapshot,data.views,data.links);callMenu.hide();hideDestinations();
 for(const update of data.views){const r=mounted.get(update.id),v=model.view(update.id);
  r.navigation=(r.navigation??0)+1;r.resolving=false;r.warming=false;r.awaitingLive=false;r.liveReturn=null;r.sourceTicket=(r.sourceTicket??0)+1;
  if(update.state!=='current'){r.popup.hidden=true;updateFragmentStatus(r);continue;}
  r.adjacent?.abort();r.adjacent=null;
  const p=update.position;
  if(update.window){
   resetSource(r,v,update.window.lines,update.actions,v.reading,functionIndent(update.origin));
  }else{
   const before=r.actions,fresh=data.neighbouringActions.get(update.id);
   setSections(r,r.sections.map(section=>({...section,actions:fresh.get(section.start)??[]})));
   if(!retargetCalls(r.code,before,r.actions))r.sourcePane.rerender();
  }
  const callsChanged=JSON.stringify(r.sites)!==JSON.stringify(update.calls.items);r.sites=update.calls.items;r.nextCalls=update.calls.next;r.matching=update.calls.matching;r.callsReady=true;if(publicationChanged||update.window)r.popup.hidden=true;if(callsChanged)renderCalls(v.id);
  updateFragmentHeading(r,v);
  sourceStatus(r,p.reset?'Live · reading reset':'Live');recordReading(r);void prepareDefinitions(v.id);
  if(r.restoreAnchor)void extendSource(r,false);
 }
 geometry();
 if(focusAfterReturn){
  const {compareButton,historyPrevious}=focusAfterReturn;
  const destination=compareButton.disabled?historyPrevious:compareButton;
  destination.focus({preventScroll:true});
 }
 save();void editorCalls.resume();
}
function liveRefreshFailed(error){
 let changed=false;
 for(const r of mounted.values()){
  if(!returnPending(r))continue;
  r.liveReturn='waiting';
  changed=true;
 }
 if(changed)geometry();
 if(error.status!==409)notice.show('Live refresh is waiting. Earlier source is retained.');
}
const live=new LiveUpdates({capture:refreshCapture,request:requestLiveRefresh,apply:applyLive,blocked:()=>busy||[...mounted.values()].some(r=>r.loading||r.sourcePane.selected()),pending:snapshot=>{if(model){clearTimeout(livePendingTimer);model.markSnapshot(snapshot);geometry();if(model.stale)livePendingTimer=setTimeout(()=>{for(const r of mounted.values()){r.awaitingLive=model.stale;updateFragmentStatus(r);}},150);}},failed:liveRefreshFailed});
function refreshLive(){if(model&&currentSnapshot)live.changed(currentSnapshot,true);}
const prepareCall=createCallPreparation({read:()=>model,
 bind:(action,signal)=>post('/api/editor/call',{snapshot:action.snapshot,query:{kind:'invocation',call:action.call,file:action.file}},signal),
 resolve:async(site,signal,snapshot,path)=>{await definitionCapabilities;return resolveCallChoice(site,{snapshot,provider:definitionProvider,warmup:()=>definitionWarmup.wait(),request:(snapshot,call,signal)=>post('/api/call-definition',{snapshot,call,path},signal),signal});},
 source:prepareFragmentSource,
 apply:({caller,target,choice,sources})=>{
  geometry();mount(caller,sources.get(model.view(caller).symbol.id));
  if(target)mount(target,sources.get(model.view(target).symbol.id));
  if(target){hideDestinations();highlightCall(caller,choice.site);focus([caller,target]);}
  else {focus([caller]);showDestinations(caller,choice);}
  save();
 }});
async function prepareBubble(action,{signal}){
 const workspace=model;
 const saved=action.kind==='bubble';
 const verify=()=>{
  signal.throwIfAborted();
  if(model!==workspace||model.stale||model.snapshot!==action.snapshot)throw new Error('Source changed before opening this bubble. Select the location again.');
 };
 verify();
 const query=saved?
  {kind:'saved_declaration',file:action.file,version:action.version,line:action.line,declaration:action.declaration}:
  {kind:'function',file:action.file,symbol:action.symbol};
 const {snapshot,symbol}=await post('/api/structure',{snapshot:action.snapshot,query},signal);
 const matches=saved?
  symbol?.tracking===action.declaration&&symbol.source?.version===action.version&&symbol.source.line===action.line:
  symbol?.id===action.symbol&&symbol.kind==='function';
 if(snapshot!==action.snapshot||!matches||symbol.source?.file!==action.file)throw new Error('The selected declaration differs from current captured source');
 verify();
 const savedView=()=>saved ? workspace.views.find(view=>
  view.state==='current'&&!frozenSource(view)&&!view.symbol.origin&&
  view.symbol.tracking===action.declaration&&view.symbol.source.file===action.file&&
  view.symbol.source.version===action.version&&view.symbol.source.line===action.line) : undefined;
 const existing=savedView()??workspace.views.find(view=>view.symbol.id===symbol.id);
 const reading=existing?.reading??{section:symbol.source.line};
 const prepared=await prepareFragmentSource(symbol,reading,signal,action.snapshot);
 verify();
 return {commit(){
  verify();
  const target=savedView();
  if(!target){openFunction(symbol,prepared);return;}
  const source=workspace.usable(target.id)?prepared:{...prepared,actions:[]};
  revealBubble(target.id,source);
 }};
}
const editorCalls=installWorkspaceDelivery({mode:'call',
 ready:action=>Boolean(size().w>=1&&size().h>=1&&(action.kind==='reveal'||model&&!model.stale)),
 async prepare(action,context){
  if(action.kind==='reveal')return {commit(){}};
  return action.kind==='call'?prepareCall(action,context):prepareBubble(action,context);
 },failed:fail,
 changed:pending=>{if(pending){closeRevisionPicker();closeLineagePicker();closeCrossFilePicker();callMenu.hide();hideDestinations();window.getSelection()?.removeAllRanges();for(const r of mounted.values()){r.navigation=(r.navigation??0)+1;r.historyTicket=(r.historyTicket??0)+1;r.historyController?.abort();if(r.historyLoading)r.comparing=r.historyLoading=r.loading=false;r.resolving=false;updateFragmentStatus(r);}geometry();live.resume();}}
});
document.addEventListener('selectionchange',()=>live.resume());
function qualifiedName(symbol){return [...symbol.parents,symbol.name].join('::');}
function updateFragmentHeading(record,view){
 const {element,referenceActions,code}=record,{symbol}=view;
 element.dataset.name=symbol.name;
 element.setAttribute('aria-label',`${symbol.name} bubble`);
 referenceActions.setAttribute('aria-label',`Actions for ${symbol.name} bubble`);
 code.setAttribute('aria-label',`${symbol.name} captured code`);
 const heading=element.querySelector('.fragment-heading');
 heading.title=qualifiedName(symbol);
 heading.querySelector('strong').textContent=heading.title;
 heading.querySelector('small').textContent=`${symbol.origin?originLabel(symbol.origin)+' · ':''}${symbol.source.path}:${symbol.source.line}`;
}
function updateFragmentStatus(r){
 const v=model.view(r.id);
 if(r.historyLoading)r.state.textContent=r.comparing?'Loading comparison…':'Loading function history…';
 else if(r.liveReturn)r.state.textContent=returnPending(r)?'Returning to live…':'Waiting for live source…';
 else if(r.resolving&&r.resolvingCaptured)r.state.textContent='Finding captured destination…';
 else if(v.captured){const label=shortRevision(v.captured.label);r.state.textContent=v.captured.returning?`Function unavailable — captured ${label}`:`Captured · ${label}`;}
 else if(v.comparison){
  const label=comparisonTitle(v.comparison);
  r.state.textContent=v.comparison.returning?`Function unavailable — ${label}`:r.openedAtFirstBlock?'At first change':'';
 }
 else if(v.history){const label=v.history.commit.slice(0,8);r.state.textContent=v.history.returning?`Function unavailable — revision ${label}`:`Revision ${label}`;}
 else if(r.resolving)r.state.textContent='Finding definition…';
 else if(r.warming)r.state.textContent='Preparing type navigation…';
 else if(r.awaitingLive&&v.mode==='live')r.state.textContent='Update pending';
 else r.state.textContent=v.state==='unresolved'?'Function unavailable — earlier version':v.mode==='pinned'?'Pinned version':r.sourceStatus??'Live';
}
function currentHistorySource(v){
 const {file,version}=v.symbol.source;
 return {snapshot:v.snapshot,file,version,tracking:v.symbol.tracking};
}

function historySource(v){
 const {file,version}=v.symbol.source;
 return {token:v.history.token,index:v.history.index,snapshot:v.snapshot,file,version};
}

function canFindEarlier(v){return Boolean(v.history&&!v.comparison&&v.mode==='pinned');}

function closeLineagePicker(restoreFocus=false){
 const picker=lineagePicker;
 lineagePicker=undefined;
 picker?.controller.abort();
 const dialog=el('lineage-dialog');
 if(dialog.open)dialog.close();
 if(restoreFocus&&picker?.trigger.isConnected)picker.trigger.focus({preventScroll:true});
}

function lineageStatus(message){
 const status=el('lineage-status');status.textContent=message;status.hidden=!message;
}

function lineageError(error){
 const message=el('lineage-error');message.textContent=error.message;message.hidden=false;
}

async function openLineagePicker(id,trigger){
 const view=model.view(id);
 if(!canFindEarlier(view)||mounted.get(id)?.loading)return;
 editorCalls.manual();callMenu.hide();hideDestinations();
 const controller=new AbortController(),source=historySource(view),picker={id,trigger,controller,source};
 lineagePicker=picker;
 const current=el('lineage-current'),candidate=el('lineage-candidate'),accept=el('accept-lineage');
 current.textContent=`${qualifiedName(view.symbol)} · ${view.symbol.source.path}:${view.symbol.source.line} · ${view.history.commit}`;
 candidate.hidden=true;accept.disabled=true;el('lineage-error').hidden=true;
 lineageStatus('Checking the immediate parent…');
 el('lineage-dialog').showModal();
 try{
  const result=await post('/api/function-history/lineage',{kind:'preview',source},controller.signal);
  if(lineagePicker!==picker)return;
  candidate.textContent=`${qualifiedName(result.symbol)} · ${result.symbol.source.path}:${result.symbol.source.line} · ${result.commit}`;
  candidate.hidden=false;
  picker.selection={kind:'accept',source,snapshot:result.snapshot,target:result.symbol.id};
  accept.disabled=false;
  lineageStatus('The declaration text matches apart from its name, and the enclosing scope matches.');
 }catch(error){
  if(lineagePicker===picker&&error.name!=='AbortError'){lineageStatus('');lineageError(error);}
 }
}
el('cancel-lineage').onclick=()=>closeLineagePicker(true);
el('lineage-dialog').addEventListener('cancel',event=>{event.preventDefault();closeLineagePicker(true);});
el('lineage-form').onsubmit=async event=>{
 event.preventDefault();
 const picker=lineagePicker,accept=el('accept-lineage');
 if(!picker?.selection||accept.disabled)return;
 accept.disabled=true;el('lineage-error').hidden=true;
 lineageStatus('Preparing the earlier declaration…');
 const accepted=await changeHistory(picker.id,0,{lineage:picker.selection,signal:picker.controller.signal,failed:lineageError});
 if(lineagePicker!==picker)return;
 if(accepted){closeLineagePicker(true);return;}
 lineageStatus('');accept.disabled=false;
};

function canChooseRevision(v){
 return !v.symbol.origin&&v.symbol.kind==='function'&&v.symbol.tracking&&
  (model.usable(v.id)||v.mode==='pinned'&&v.state==='current'||frozenSource(v));
}

function openRevisionPicker(id,trigger){
 const v=model.view(id),r=mounted.get(id);
 if(!r||r.loading||returnPending(r)||!canChooseRevision(v))return;
 editorCalls.manual();callMenu.hide();hideDestinations();r.popup.hidden=true;
 revisionPicker={id,trigger,controller:new AbortController()};
 el('revision-input').value='';
 el('revision-error').hidden=true;
 el('revision-status').textContent='';
 el('open-revision').disabled=false;
 el('revision-dialog').showModal();
}

function closeRevisionPicker(restoreFocus=false){
 const picker=revisionPicker;
 picker?.controller.abort();
 revisionPicker=undefined;
 const dialog=el('revision-dialog');
 if(dialog.open)dialog.close();
 if(restoreFocus&&picker){
  const button=picker.trigger.disabled||picker.trigger.hidden?mounted.get(picker.id)?.chooseRevision:picker.trigger;
  button?.focus({preventScroll:true});
 }
}

el('cancel-revision').onclick=()=>closeRevisionPicker(true);
el('revision-dialog').addEventListener('cancel',event=>{event.preventDefault();closeRevisionPicker(true);});
el('revision-form').onsubmit=async event=>{
 event.preventDefault();
 const picker=revisionPicker,submit=el('open-revision'),status=el('revision-status'),error=el('revision-error');
 if(!picker||submit.disabled)return;
 submit.disabled=true;
 error.hidden=true;
 status.textContent='Reading revision…';
 const selected=await changeHistory(picker.id,0,{
  revision:el('revision-input').value,
  signal:picker.controller.signal,
  failed:failure=>{error.hidden=false;error.textContent=failure.message;},
 });
 if(revisionPicker!==picker)return;
 if(selected){closeRevisionPicker(true);return;}
 status.textContent='';
 submit.disabled=false;
 el('revision-input').focus();
};

async function changeHistory(id,direction,selection){
 const r=mounted.get(id),workspace=model;
 if(!r||r.loading||returnPending(r))return;
 const v=model.view(id);
 if(r.sourcePane.selected()){
  selection?.failed(new Error('Clear the source selection before changing revision.'));
  return;
 }
 if(selection&&!(selection.lineage?canFindEarlier(v):canChooseRevision(v))){
  selection.failed(new Error('Wait for the function source before choosing a revision.'));
  return;
 }
 if(!selection&&!v.history&&(!model.usable(id)||v.symbol.origin))return;
 editorCalls.manual();callMenu.hide();hideDestinations();r.popup.hidden=true;r.navigation=(r.navigation??0)+1;r.resolving=false;r.warming=false;
 const binding=JSON.stringify([v.snapshot,v.symbol.id,v.history]),ticket=r.historyTicket=(r.historyTicket??0)+1,controller=new AbortController();r.historyController=controller;r.historyLoading=true;r.loading=true;geometry();
 const signal=selection?AbortSignal.any([controller.signal,selection.signal]):controller.signal;
 const verify=()=>{signal.throwIfAborted();if(model!==workspace||mounted.get(id)!==r||r.historyTicket!==ticket||JSON.stringify([model.view(id).snapshot,model.view(id).symbol.id,model.view(id).history])!==binding)throw new DOMException('Function history superseded','AbortError');if(r.sourcePane.selected())throw new Error('Clear the source selection before changing revision.');};
 let finished=false;
 const finish=()=>{
  if(finished)return;
  finished=true;
  if(mounted.get(id)===r&&r.historyTicket===ticket){r.historyLoading=false;r.loading=false;geometry();live.resume();}
 };
 signal.addEventListener('abort',finish,{once:true});
 try{
  let revision;
  if(selection?.lineage){
   if(JSON.stringify(historySource(v))!==JSON.stringify(selection.lineage.source))throw new Error('The selected revision changed. Open the earlier declaration finder again.');
   revision=await post('/api/function-history/lineage',selection.lineage,signal);
  }else if(selection){
   const captured=frozenSource(v);
   const origin=captured?{kind:'captured',source:captured}:{kind:v.mode==='pinned'?'pinned':'current',source:currentHistorySource(v)};
   revision=await post('/api/function-history/select',{revision:selection.revision,origin},signal);
  }else if(v.history){
   revision=await post('/api/function-history/step',{token:v.history.token,index:v.history.index+direction},signal);
  }else{
   revision=await post('/api/function-history',currentHistorySource(v),signal);
  }
  verify();const planned=Workspace.restore(model.serialize(),model.snapshot);planned.showHistory(id,revision);const next=planned.view(id),s=next.symbol.source,start=next.reading.section;
  const query=new URLSearchParams({token:revision.token,snapshot:revision.snapshot,file:s.file,version:s.version,line_start:String(start),line_count:String(Math.min(96,s.end_line-start+1)),outline_count:'1',reflow:'true'});
  const [data,actions]=await Promise.all([start===s.line?revision.window:transport('/api/function-history/window?'+query,{signal}),capturedActions(next,start,Math.min(96,s.end_line-start+1),signal)]);verify();
  if(data?.snapshot!==revision.snapshot||data.file!==s.file||data.version!==s.version||!Array.isArray(data.lines)||!data.lines.length||data.lines.length>96||data.lines[0].number!==start||revision.origin?.number!==s.line||revision.origin.left||revision.origin.column!==0)throw new Error('Historical source differs from the requested function');
  r.sourceTicket=(r.sourceTicket??0)+1;model.showHistory(id,revision);r.liveReturn=null;r.historyLoading=false;r.loading=false;r.sites=[];r.callsReady=false;r.actions=[];
  updateFragmentHeading(r,model.view(id));
  applyPreparedSource(id,{data,actions,start,margin:functionIndent(revision.origin)});save();
  if(selection&&!selection.lineage){
   void post('/api/function-history/confirm',{token:revision.token}).catch(error=>{
    if(model===workspace&&mounted.get(id)===r&&model.view(id).history?.token===revision.token)notice.show(error.message);
   });
  }
  return true;
 }catch(e){if(!signal.aborted&&e.name!=='AbortError'&&mounted.get(id)===r&&model===workspace&&r.historyTicket===ticket)(selection?.failed??fail)(e);}
 finally{signal.removeEventListener('abort',finish);finish();}
}
function returnHistoryLive(id){
 const r=mounted.get(id);if(returnPending(r))return;
 editorCalls.manual();callMenu.hide();hideDestinations();r.popup.hidden=true;r.historyController?.abort();r.historyTicket=(r.historyTicket??0)+1;r.sourceTicket=(r.sourceTicket??0)+1;r.navigation=(r.navigation??0)+1;
 model.returnLive(id);r.historyLoading=false;r.loading=false;r.resolving=false;r.actions=[];r.callsReady=false;r.liveReturn='pending';geometry();save();refreshLive();
}
const newestHistory=v=>v.history?.index===0&&!v.comparison;
function shortRevision(label){return /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/.test(label)?label.slice(0,8):label;}
function comparisonTitle(c){return `${c.side==='older'?'Before':'After'} · ${shortRevision(c.label)}`;}
function sourceBinding(v){const owner=frozenSource(v)?.owner;return owner?.kind==='comparison'?{token:owner.token,side:owner.side}:owner?{token:owner.token}:{};}
function sourceEndpointForView(v){const owner=frozenSource(v)?.owner;return owner?.kind==='comparison'?'/api/function-comparisons/window':owner?'/api/function-history/window':v.symbol.origin?'/api/dependency/window':'/api/window';}
async function capturedActions(v,start,count,signal){
 const actions=[];let offset=0;
 try{for(let page=0;page<16;page++){const data=await post('/api/captured-functions',{source:frozenSource(v),query:{kind:'actions',line_start:start,line_count:count,offset}},signal);if(data.snapshot!==v.snapshot||!Array.isArray(data.items)||data.items.length>256||!Number.isSafeInteger(data.matching)||data.matching>4096)throw new Error('Invalid captured call actions');actions.push(...data.items);if(data.next===null)return actions;if(!Number.isSafeInteger(data.next)||data.next<=offset)throw new Error('Invalid captured call continuation');offset=data.next;}throw new Error('Captured call-action allowance exceeded');}catch(error){if(signal?.aborted)throw error;return [];}
}
function canLookInFile(view,site){
 return Boolean(site.name&&/^(?:[0-9a-f]{40}|[0-9a-f]{64})$/.test(frozenLabel(view)));
}

function closeCrossFilePicker(restoreFocus=false){
 const picker=crossFilePicker;
 crossFilePicker=undefined;
 picker?.request?.abort();
 const dialog=el('cross-file-dialog');
 if(dialog.open)dialog.close();
 if(restoreFocus)picker?.restoreFocus();
}

function openCrossFilePicker({commit,site,path,signal,request,open,restoreFocus}){
 closeCrossFilePicker();
 const picker={commit,path,signal,request:null,send:request,open,restoreFocus};
 crossFilePicker=picker;
 el('cross-file-path').value='';
 el('cross-file-commit').textContent=`${site.spelling??site.name} · ${commit}`;
 el('cross-file-results').replaceChildren();
 el('cross-file-error').hidden=true;
 el('cross-file-dialog').showModal();
 signal.addEventListener('abort',()=>{if(crossFilePicker===picker)closeCrossFilePicker();},{once:true});
 void browseCapturedDirectory(path.split('/').slice(0,-1).join('/'));
}

async function runCrossFileOperation(message,operation,render){
 const picker=crossFilePicker;
 if(!picker)return;
 picker.request?.abort();
 const task=new AbortController();picker.request=task;
 const signal=AbortSignal.any([picker.signal,task.signal]);
 const error=el('cross-file-error'),status=el('cross-file-status');
 error.hidden=true;status.textContent=message;
 try{
  const result=await operation(picker,signal);
  signal.throwIfAborted();
  render(result,picker);
 }catch(failure){
  if(!signal.aborted&&crossFilePicker===picker&&failure.name!=='AbortError'){
   error.textContent=failure.message;error.hidden=false;
  }
 }finally{
  if(crossFilePicker===picker&&picker.request===task&&status.textContent===message)status.textContent='';
 }
}

function browseCapturedDirectory(directory,offset=0){
 const results=el('cross-file-results');
 const restoreFocus=results.contains(document.activeElement);
 results.replaceChildren();
 const up=button('Up one directory',null,()=>void browseCapturedDirectory(directory.split('/').slice(0,-1).join('/')));
 up.hidden=!directory;results.append(up);
 const root=button('Repository root',null,()=>void browseCapturedDirectory(''));
 root.hidden=!directory;results.append(root);
 return runCrossFileOperation('Reading files at this commit…',
  (picker,signal)=>picker.send({kind:'files',directory,offset},signal),
  (data,picker)=>{
   if(data.commit!==picker.commit||data.directory!==directory||!Array.isArray(data.items)||data.items.length>24)throw new Error('Directory listing differs from the captured commit');
   el('cross-file-status').textContent=directory?`Files in ${directory}`:'Files at repository root';
   for(const item of data.items){
    if(item.path===picker.path)continue;
    const name=item.path.split('/').at(-1)+(item.directory?'/':'');
    const label=item.directory?`Browse ${item.path}`:`Find candidates in ${item.path}`;
    results.append(button(name,label,()=>{
     if(item.directory)void browseCapturedDirectory(item.path);
     else{el('cross-file-path').value=item.path;void findCapturedCandidates(item.path);}
    }));
   }
   if(Number.isSafeInteger(data.next)&&data.next>offset)results.append(button('More files',null,()=>void browseCapturedDirectory(directory,data.next)));
   if(restoreFocus)(results.querySelector('button:not([hidden])')??el('cross-file-path')).focus();
  });
}

function findCapturedCandidates(path,offset=0){
 const results=el('cross-file-results'),restoreFocus=results.contains(document.activeElement);
 results.replaceChildren(button('Browse repository files',null,()=>void browseCapturedDirectory('')));
 return runCrossFileOperation('Reading declarations in the selected file…',
  (picker,signal)=>picker.send({kind:'candidates',path,offset},signal),
  (data,picker)=>{
   if(data.commit!==picker.commit||data.path!==path||!Array.isArray(data.items)||data.items.length>8)throw new Error('File candidates differ from the captured commit');
   const partial=data.coverage?.partial_files||data.coverage?.omitted_symbols;
   el('cross-file-status').textContent=(data.matching?`${data.matching} name ${data.matching===1?'candidate':'candidates'} in ${path}.`:`No name candidates in ${path}.`)+(partial?' Declaration coverage is partial.':'');
   let firstChoice;
   for(const target of data.items){
    if(target.source?.path!==path)throw new Error('Candidate is outside the selected file');
    const label=[...target.parents,target.name].join('::');
    const choice=button(label,`Open candidate ${label}`,()=>{
     void runCrossFileOperation('Preparing captured source…',
      (pending,signal)=>pending.open({kind:'open_file',path,snapshot:data.snapshot,target:target.id},signal),
      opened=>{if(opened)closeCrossFilePicker();});
    });
    choice.append(node('small',`${path}:${target.source.line} · Name candidate`));results.append(choice);
    firstChoice??=choice;
   }
   if(Number.isSafeInteger(data.next)&&data.next>offset)results.append(button('More candidates',null,()=>void findCapturedCandidates(path,data.next)));
   if(restoreFocus)(firstChoice??el('cross-file-path')).focus();
  });
}
el('close-cross-file').onclick=()=>closeCrossFilePicker(true);
el('cross-file-dialog').addEventListener('cancel',event=>{event.preventDefault();closeCrossFilePicker(true);});
el('cross-file-form').onsubmit=event=>{event.preventDefault();void findCapturedCandidates(el('cross-file-path').value);};

async function chooseCapturedCall(id,action,{anotherFile=false,opener}={}){
 const r=mounted.get(id),workspace=model,view=model.view(id),source=frozenSource(view);
 if(!r||!source||!model.navigable(id)||r.historyLoading)return;
 editorCalls.manual();callMenu.hide();hideDestinations();r.popup.hidden=true;
 r.historyController?.abort();
 const epoch=r.navigation=(r.navigation??0)+1,signature=JSON.stringify(source),controller=new AbortController();
 r.historyController=controller;
 const current=()=>model===workspace&&mounted.get(id)===r&&r.navigation===epoch;
 const verify=()=>{
  controller.signal.throwIfAborted();
  if(!current()||!model.navigable(id)||JSON.stringify(frozenSource(model.view(id)))!==signature)throw new DOMException('Captured navigation superseded','AbortError');
 };
 const send=async(query,signal)=>{
  verify();signal.throwIfAborted();
  const result=await post('/api/captured-functions',{source,query:{...query,call:action.id}},signal);
  verify();signal.throwIfAborted();return result;
 };
 const open=async(query,signal)=>{
  if(r.sourcePane.selected())throw new Error('Clear the source selection before following the call.');
  r.resolving=r.resolvingCaptured=true;updateFragmentStatus(r);
  try{
   const result=await send(query,signal),frame=result.frame;
   if(result.invocation?.id!==action.id||frame?.symbol?.id!==query.target||
    (query.kind==='open_file'&&(frame.cross_file!==true||typeof result.prepared!=='boolean'||frame.snapshot!==query.snapshot||frame.symbol.source.path!==query.path)))throw new Error('Captured destination differs from the selected candidate');
   const planned=Workspace.restore(model.serialize(),model.snapshot),next=planned.followCaptured(id,result.invocation,frame),view=planned.view(next),s=view.symbol.source,start=view.reading.section;
   const count=Math.min(96,s.end_line-start+1),params=new URLSearchParams({...sourceBinding(view),snapshot:view.snapshot,file:s.file,version:s.version,line_start:String(start),line_count:String(count),outline_count:'1',reflow:'true'});
   const [data,actions]=await Promise.all([start===frame.window.line_start?frame.window:transport(sourceEndpointForView(view)+'?'+params,{signal}),capturedActions(view,start,count,signal)]);
   verify();signal.throwIfAborted();
   const prepared=preparedComparison(view,{window:data,origin:frame.origin});prepared.actions=actions;
   if(result.prepared===true){
    await post('/api/captured-functions',{source:frozenSource(view),query:{kind:'confirm_file'}},signal);
    verify();signal.throwIfAborted();
   }
   if(r.sourcePane.selected())throw new DOMException('Captured navigation deferred for selection','AbortError');
   const destination=model.followCaptured(id,result.invocation,frame);
   hideDestinations();mount(destination,prepared);focus([id,destination]);save();
   return true;
  }finally{if(current()){r.resolving=false;updateFragmentStatus(r);live.resume();}}
 };
 const restoreFocus=()=>{
  if(mounted.get(id)!==r)return;
  const control=opener?.isConnected?opener:r.callsButton;
  control.focus({preventScroll:true});
 };
 const browse=site=>openCrossFilePicker({commit:frozenLabel(view),site,path:view.symbol.source.path,signal:controller.signal,request:send,open,restoreFocus});
 const report=error=>{if(error.name!=='AbortError'&&current())fail(error);};
 if(anotherFile){
  r.resolving=false;updateFragmentStatus(r);
  if(canLookInFile(view,action))browse(action);
  return;
 }
 r.resolving=r.resolvingCaptured=true;updateFragmentStatus(r);
 try{
  const result=await send({kind:'definition'},controller.signal);
  const site=result.invocation;
  if(result.snapshot!==source.snapshot||site?.owner!==source.symbol||site.id!==action.id)throw new Error('Captured definition differs from the visible caller');
  const openLocal=target=>open({kind:'open',target:target.id},controller.signal);
  const choice={site,attempted:!result.complete&&!site.binding,semantic:false,note:result.detail,
   captured:{open:target=>void openLocal(target).catch(report),browse:canLookInFile(view,site)?()=>browse(site):null}};
  const target=directDestination(choice);
  if(target)await openLocal(target);else showDestinations(id,choice);
 }catch(error){report(error);}
 finally{if(current()){r.resolving=false;updateFragmentStatus(r);live.resume();}}
}
function historyControls(r,v){
 const atNewest=newestHistory(v),pending=returnPending(r),hasCapture=v.history||v.comparison||v.captured;
 r.historyRow.hidden=Boolean(v.symbol.origin);
 r.historyPrevious.disabled=r.loading||pending||(!v.history&&!model.usable(v.id));
 r.historyNext.hidden=!v.history;
 r.historyNext.disabled=!atNewest&&(r.loading||pending||!v.history);
 r.historyNext.textContent=atNewest?'Return to live':'Next change';
 r.historyNext.setAttribute('aria-label',atNewest?'Return to live':'Next function change');
 r.historyNext.setAttribute('aria-disabled',String(atNewest&&pending));
 r.historyLabel.textContent=v.history?v.history.commit.slice(0,8):'';
 r.historyLabel.title=v.history?`Commit ${v.history.commit}; first-parent history at ${v.history.tip}`:'';
 r.chooseRevision.hidden=Boolean(v.comparison)||!canChooseRevision(v);
 r.chooseRevision.disabled=r.loading||pending;
 r.pin.hidden=atNewest;
 r.pin.disabled=r.historyLoading&&!hasCapture;
 r.pin.textContent=hasCapture?'Return to live':v.mode==='pinned'?'Follow live':'Pin version';
 r.pin.setAttribute('aria-label',hasCapture?'Return to live':v.mode==='pinned'?'Follow live changes':'Pin version');
 r.pin.setAttribute('aria-disabled',String(pending));
}
function comparisonControls(r,v){
 if(v.captured)r.element.dataset.capturedFile='true';else delete r.element.dataset.capturedFile;
 const c=v.comparison;
 r.compareButton.hidden=Boolean(v.symbol.origin)||Boolean(c&&!v.history);
 r.compareButton.textContent=c?'Older revision (exit diff)':'Compare';
 r.compareButton.setAttribute('aria-label',c?'Older revision (exit diff)':'Compare with previous version');
 r.compareButton.title=c?'Open an older revision and exit the diff in this bubble.':'Compare with the previous version and open at the first changed block.';
 r.compareButton.disabled=r.loading||returnPending(r)||model.views.length>=24&&!c||!v.history&&!model.usable(v.id);
 r.editPrevious.hidden=r.editNext.hidden=!c;r.editPrevious.disabled=r.loading||!c||c.returning||c.cursor===0;r.editNext.disabled=r.loading||!c||c.returning||c.cursor>=c.changes.length-1;
 r.historyPrevious.hidden=Boolean(c||v.captured);if(v.captured){r.historyNext.hidden=true;r.compareButton.hidden=true;r.historyLabel.textContent=shortRevision(v.captured.label);r.historyLabel.title=`Captured file · ${v.captured.label}`;}if(c){r.historyNext.hidden=true;r.historyLabel.textContent=`${comparisonTitle(c)} · ${c.changes.length?c.cursor+1:0}/${c.changes.length}`;r.historyLabel.title=`${c.label}; compared with ${c.other_label}`;r.element.dataset.comparisonSide=c.side;r.element.dataset.comparisonToken=c.token;}else{delete r.element.dataset.comparisonSide;delete r.element.dataset.comparisonToken;}
 r.historyLabel.setAttribute('aria-label',r.historyLabel.title);
 const key=c?`${c.token}:${c.side}`:'';
 if(r.comparisonKey===key&&r.comparisonFirst===r.code.firstElementChild&&r.comparisonLast===r.code.lastElementChild)return;
 for(const row of r.code.querySelectorAll('.code-line')){const number=Number(row.dataset.line),changed=Boolean(c&&c.changes.some(([start,end])=>number>=start&&number<end));row.classList.toggle('comparison-removed',changed&&c.side==='older');row.classList.toggle('comparison-added',changed&&c.side==='newer');if(changed)row.title=`${c.side==='older'?'Removed or replaced':'Added or replaced'} line ${number} · ${shortRevision(c.label)}`;else row.removeAttribute('title');}
 r.comparisonKey=key;r.comparisonFirst=r.code.firstElementChild;r.comparisonLast=r.code.lastElementChild;
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
  const opened=model.openComparison(id,pair);r.historyLoading=r.loading=r.comparing=false;r.liveReturn=null;r.sites=[];r.actions=[];r.callsReady=false;updateFragmentHeading(r,model.view(id));
  applyPreparedSource(id,{...newer,openedAtFirstBlock:pair.newer.changes.length>0});
  mount(opened.older,older);focus([id,opened.older]);save();
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
 if(!model)return;for(const[id,r]of mounted)if(!model.views.some(v=>v.id===id)){r.controller.abort();r.historyController?.abort();r.sourcePane.dispose();r.element.remove();mounted.delete(id);}const c=model.camera;el('workspace-world').style.transform=`translate(${-c.x*c.scale}px,${-c.y*c.scale}px) scale(${c.scale})`;el('visual-zoom').textContent=Math.round(c.scale*100)+'%';el('visual-empty').hidden=model.items().length>0;el('visual-stale').hidden=!model.views.some(v=>v.state==='unresolved'&&!mounted.get(v.id)?.liveReturn);
 for(const v of model.views){const r=mounted.get(v.id);if(!r)continue;r.element.hidden=!model.isVisible(v.id);Object.assign(r.element.style,{left:v.x+'px',top:v.y+'px',width:v.w+'px',height:(v.collapsed?54:v.h)+'px'});const membership=modulePresentation(v,model.views);r.element.classList.toggle('has-module',Boolean(membership));const moduleKey=JSON.stringify(membership);if(moduleKey!==r.moduleKey){r.moduleKey=moduleKey;r.moduleTag.hidden=!membership;if(membership){r.element.dataset.moduleId=membership.id;r.moduleTag.dataset.moduleId=membership.id;r.moduleTag.textContent=membership.label;r.moduleTag.title=membership.title;r.element.style.setProperty('--module-accent',`var(--module-${membership.colour})`);r.element.style.setProperty('--module-tint',`var(--module-tint-${membership.colour})`);}else{delete r.element.dataset.moduleId;delete r.moduleTag.dataset.moduleId;r.moduleTag.textContent='';r.element.style.removeProperty('--module-accent');r.element.style.removeProperty('--module-tint');}}r.body.hidden=v.collapsed;r.collapse.textContent=v.collapsed?'▾':'▴';r.collapse.setAttribute('aria-label',v.collapsed?'Expand bubble':'Collapse bubble');r.element.classList.toggle('is-selected',v.id===model.selected);r.element.classList.toggle('is-stale',!model.usable(v.id));r.prev.disabled=r.next.disabled=Boolean(r.historyLoading);r.callsButton.disabled=!model.navigable(v.id)||r.historyLoading;r.referenceActions.disabled=r.historyLoading||(!model.usable(v.id)&&!(!r.loading&&!returnPending(r)&&(canFindEarlier(v)||v.comparison&&canChooseRevision(v))));r.fitHeight.disabled=r.fitWidth.disabled=r.loading||!r.lines?.length||v.collapsed;inlineAvailability(r.code,v.id);r.keep.disabled=Boolean(v.symbol.origin)||!v.symbol.tracking||!model.usable(v.id)||r.historyLoading||!board||busy;r.keep.title=v.symbol.origin?'Dependency locations stay in this workspace.':'Save location';r.editor.disabled=Boolean(v.symbol.origin?.archive&&(!editorJavaArchives||!v.symbol.origin.editor_archive))||!model.usable(v.id)||r.historyLoading||busy;historyControls(r,v);comparisonControls(r,v);updateFragmentStatus(r);restoreVisibleReading(r,v);}
 if(model.stale)callMenu.hide();links();overview();outlines.refresh();highlights.refresh();taskGroups.refresh();referenceStacks.refresh();
}
function button(text,label,handler){const b=node('button',text);b.type='button';if(label)b.setAttribute('aria-label',label);b.onclick=()=>{try{handler();}catch(e){fail(e);}};return b;}
function bindDrag(handle,id,resizing=false){handle.addEventListener('pointerdown',event=>{if(event.button!==0||(!resizing&&event.target.closest('button')))return;event.preventDefault();model.selected=id;const v=model.view(id),start={x:event.clientX,y:event.clientY,vx:v.x,vy:v.y,w:v.w,h:v.h,scale:model.camera.scale};handle.setPointerCapture(event.pointerId);
 const move=e=>{const dx=(e.clientX-start.x)/start.scale,dy=(e.clientY-start.y)/start.scale;try{if(resizing)model.resize(id,Math.max(320,Math.min(BUBBLE_SIZE.maxWidth,start.w+dx)),Math.max(200,Math.min(BUBBLE_SIZE.maxHeight,start.h+dy)));else model.move(id,start.vx+dx,start.vy+dy);geometry();}catch(e){fail(e);}};
 const end=()=>{handle.removeEventListener('pointermove',move);handle.removeEventListener('pointerup',end);handle.removeEventListener('pointercancel',end);save();};handle.addEventListener('pointermove',move);handle.addEventListener('pointerup',end);handle.addEventListener('pointercancel',end);});}
function bindLayoutKeys(handle,id,resizing=false){handle.tabIndex=0;handle.addEventListener('keydown',e=>{if(e.target!==handle)return;const delta={ArrowLeft:[-16,0],ArrowRight:[16,0],ArrowUp:[0,-16],ArrowDown:[0,16]}[e.key];if(!delta)return;e.preventDefault();try{const v=model.view(id);if(resizing)model.resize(id,Math.max(320,Math.min(BUBBLE_SIZE.maxWidth,v.w+delta[0])),Math.max(200,Math.min(BUBBLE_SIZE.maxHeight,v.h+delta[1])));else model.move(id,v.x+delta[0],v.y+delta[1]);model.selected=id;geometry();save();}catch(e){fail(e);}});}
function fitCode(id,axis){const r=mounted.get(id),v=model.view(id),section=r.sections.find(part=>part.start===v.reading.section),content=r.sourcePane.contentSize(section?.start,section?.lines.length);if(!content)return;const extent=Math.ceil(axis==='width'?v.w-r.code.clientWidth+content.width:v.h-r.code.clientHeight+content.height);model.fitSize(id,axis,extent);geometry();save();if(extent>(axis==='width'?BUBBLE_SIZE.maxWidth:BUBBLE_SIZE.maxHeight))notice.show('Some code still needs scrolling at this size.');}
function mount(id,prepared){
 if(mounted.has(id)){if(prepared&&!mounted.get(id).code.querySelector('.code-line'))applyPreparedSource(id,prepared);return;}const v=model.view(id),element=node('section');element.className='fragment';element.dataset.view=id;element.dataset.name=v.symbol.name;element.setAttribute('aria-label',`${v.symbol.name} bubble`);
 const header=node('div');header.className='fragment-header';header.setAttribute('aria-label','Move bubble with arrow keys');bindLayoutKeys(header,id);const heading=node('div');heading.className='fragment-heading';const context=node('div');context.className='fragment-context';const moduleTag=node('span');moduleTag.className='module-tag';moduleTag.tabIndex=0;moduleTag.hidden=true;context.append(moduleTag,node('small',`${v.symbol.origin?originLabel(v.symbol.origin)+' · ':''}${v.symbol.source.path}:${v.symbol.source.line}`));heading.append(node('strong',[...v.symbol.parents,v.symbol.name].join('::')),context);
 const collapse=button('▴','Collapse bubble',()=>{callMenu.hide();model.collapse(id);geometry();save();}),close=button('×','Close bubble',()=>{editorCalls.manual();callMenu.hide();const r=mounted.get(id);r.controller.abort();r.historyController?.abort();r.sourcePane.dispose();element.remove();mounted.delete(id);model.close(id);hideDestinations();geometry();save();});const pin=button('Pin version','Pin version',()=>{if(model.view(id).history||model.view(id).comparison||model.view(id).captured){returnHistoryLive(id);return;}model.pin(id,model.view(id).mode!=='pinned');geometry();save();refreshLive();});const fitHeight=button('↕','Fit height',()=>fitCode(id,'height')),fitWidth=button('↔','Fit width',()=>fitCode(id,'width'));for(const b of [fitHeight,fitWidth]){b.className='bubble-fit';b.title=b.getAttribute('aria-label')+' to this code section';}heading.title=[...v.symbol.parents,v.symbol.name].join('::');
 const referenceActions=button('⋯',`Actions for ${v.symbol.name} bubble`,()=>{
  const view=model.view(id);
  const actions=view.comparison?[{label:'Choose revision…',run:()=>openRevisionPicker(id,referenceActions)}]:canFindEarlier(view)?[
   {label:'Find earlier declaration…',run:()=>void openLineagePicker(id,referenceActions)},
  ]:[
   {label:'Find callers',run:()=>openReferences(id,'callers')},
   {label:'Find references',run:()=>openReferences(id,'references')},
   {label:'Task group',run:()=>taskControls.showGroups(model.groupFor(id)?.id??'')},
  ];
  callMenu.show(referenceActions,view.symbol.name,actions,Boolean(view.comparison)||canFindEarlier(view));
 });
 header.append(heading,referenceActions,fitHeight,fitWidth,collapse,close);element.append(header);bindDrag(header,id);
 const body=node('div');body.className='fragment-body';const historyRow=node('div');historyRow.className='fragment-history';const historyPrevious=button('Previous change','Previous function change',()=>void changeHistory(id,1)),historyNext=button('Next change','Next function change',()=>{if(newestHistory(model.view(id)))returnHistoryLive(id);else void changeHistory(id,-1);}),historyLabel=node('span');historyLabel.setAttribute('role','status');const compareButton=button('Compare','Compare with previous version',()=>model.view(id).comparison?void changeHistory(id,1):void compareFunction(id)),editPrevious=button('Previous changed block',null,()=>void comparisonEdit(id,-1)),editNext=button('Next changed block',null,()=>void comparisonEdit(id,1));
 const chooseRevision=button('Choose revision…',null,()=>openRevisionPicker(id,chooseRevision));
 chooseRevision.setAttribute('aria-haspopup','dialog');
 editPrevious.title='Show the previous changed block in both visible comparison bubbles.';
 editNext.title='Show the next changed block in both visible comparison bubbles.';
 editPrevious.className='comparison-previous';
 editNext.className='comparison-next';
 compareButton.className='comparison-revision';
 historyRow.append(historyPrevious,editPrevious,historyLabel,historyNext,editNext,compareButton,chooseRevision);const section=node('div');section.className='source-section';const sectionLabel=node('span','Loading captured source…');const prev=button('Previous section',null,()=>jumpSource(id,true)),next=button('Next section',null,()=>jumpSource(id,false));prev.hidden=true;next.hidden=true;section.append(sectionLabel,prev,next);
 const code=node('div');code.className='code-pane';code.tabIndex=0;code.setAttribute('aria-label',`${v.symbol.name} captured code`);code.append(node('p','Loading source…'));const footer=node('div');footer.className='fragment-footer';const callsButton=button('Calls','Calls',()=>toggleCalls(id)),state=node('span','Captured source');state.setAttribute('role','status');const keep=button('Save','Save location',()=>void keepLocation(id)),editor=button('Editor',`Open in ${editorName}`,()=>void openEditor(id));editor.disabled=true;footer.append(callsButton,state,pin,keep,editor);
 const popup=node('div');popup.className='calls-popover';popup.hidden=true;body.append(historyRow,section,code,popup,footer);const resize=button('','Resize bubble',()=>{});resize.title='Drag or use arrow keys to resize';resize.className='resize-handle';body.append(resize);bindDrag(resize,id,true);bindLayoutKeys(resize,id,true);element.append(body);el('fragment-layer').append(element);
 const record={id,compareButton,editPrevious,editNext,historyRow,historyPrevious,historyNext,historyLabel,chooseRevision,referenceActions,moduleTag,pin,fitHeight,fitWidth,loading:true,liveReturn:(v.captured??v.comparison??v.history)?.returning?'waiting':null,element,body,code,collapse,sectionLabel,prev,next,callsButton,popup,state,keep,editor,sites:[],nextCalls:0,matching:0,callsReady:false,controller:new AbortController()};setSections(record,[]);record.sourcePane=new SourcePane(code,(text,line,plan,margin)=>{renderCallText(text,line,record.actionsByLine.get(line.number)??[],(action,button,direct,keyboard)=>inlineAction(id,action,button,direct,keyboard),plan,margin);inlineAvailability(text,id);});record.sourcePane.setReflow(smartReflow);mounted.set(id,record);
 element.addEventListener('pointerdown',()=>{model.selected=id;geometry();});code.addEventListener('scroll',()=>{
  callMenu.hide();if(mounted.get(id)!==record||!code.clientHeight)return;
  const before=code.scrollTop<record.scrollTop,moved=code.scrollTop!==record.scrollTop;
  if(moved){
   record.restoreAnchor=null;
   if(record.openedAtFirstBlock){record.openedAtFirstBlock=false;updateFragmentStatus(record);}
  }
  record.scrollTop=code.scrollTop;recordReading(record);save();
  if(moved)void extendSource(record,before);
 });
 code.addEventListener('wheel',event=>{if(event.deltaY)void extendSource(record,event.deltaY<0);},{passive:true});
 let touchStart;
 code.addEventListener('touchstart',event=>{touchStart=event.touches.length===1?event.touches[0]:null;},{passive:true});
 code.addEventListener('touchmove',event=>{
  if(!touchStart||event.touches.length!==1)return;
  const point=event.touches[0],dy=point.clientY-touchStart.clientY;
  if(Math.abs(dy)<12||Math.abs(dy)<=Math.abs(point.clientX-touchStart.clientX))return;
  touchStart=null;
  void extendSource(record,dy>0);
 },{passive:true});
 code.addEventListener('keydown',event=>{
  if(event.target!==code)return;
  if(['ArrowUp','PageUp'].includes(event.key))void extendSource(record,true);
  if(['ArrowDown','PageDown'].includes(event.key))void extendSource(record,false);
 });
 jobs=jobs.then(async()=>{if(mounted.get(id)!==record)return;if(!record.code.querySelector('.code-line'))await loadSource(id,v.reading.section);if(model.navigable(id))await loadCalls(id,0);}).catch(fail);geometry();if(prepared)applyPreparedSource(id,prepared);
}
async function prepareFragmentSource(symbol,reading,signal,snapshot) {
 const s=symbol.source,start=reading.section,expected=captureFor(symbol,snapshot),endpoint=symbol.origin?'/api/dependency/window':'/api/window';
 const count=Math.min(96,s.end_line-start+1),query=new URLSearchParams({snapshot:expected,file:s.file,version:s.version,line_start:String(start),line_count:String(count),outline_count:'1',reflow:'true'});
 const header=start===s.line?Promise.resolve(null):transport(endpoint+'?'+new URLSearchParams({snapshot:expected,file:s.file,version:s.version,line_start:String(s.line),line_count:'1',outline_count:'1'}),{signal});
 const [data,origin,actions]=await Promise.all([transport(endpoint+'?'+query,{signal}),header,optionalActions(symbol,start,count,snapshot,signal)]);
 if(data.snapshot!==expected||data.file!==s.file||data.version!==s.version||!Array.isArray(data.lines)||!data.lines.length||data.lines.length>count||data.lines.some((line,index)=>line?.number!==start+index))throw new Error('Captured source for the requested function is unavailable');
 if(origin&&(origin.snapshot!==expected||origin.file!==s.file||origin.version!==s.version||origin.lines?.length!==1||origin.lines[0].number!==s.line||origin.lines[0].left||origin.lines[0].column!==0))throw new Error('Fragment indentation source differs');
 return {data,actions,start,margin:functionIndent(origin?.lines[0]??data.lines.find(line=>line.number===s.line))};
}
function setSections(r,sections){
 r.sections=sections;
 r.lines=sections.flatMap(section=>section.lines);
 r.actions=sections.flatMap(section=>section.actions.filter(action=>action.position?.line>=section.start&&action.position.line<section.start+section.lines.length));
 r.actionsByLine=new Map();
 for(const action of r.actions){
  const line=action.position?.line;
  if(!r.actionsByLine.has(line))r.actionsByLine.set(line,[]);
  r.actionsByLine.get(line).push(action);
 }
}
function resetSource(r,v,lines,actions,reading,margin){
 r.adjacent?.abort();r.adjacent=null;
 r.openedAtFirstBlock=false;
 r.margin=margin;
 setSections(r,[{start:reading.section,lines:lines.filter(line=>line.number<=v.symbol.source.end_line),actions}]);
 const restored=r.sourcePane.set(r.lines,reading,margin);
 r.hiddenReading=r.code.clientWidth?null:reading;
 r.restoreAnchor=!r.hiddenReading&&!restored&&r.lines.at(-1)?.number<v.symbol.source.end_line?reading.anchor:null;
 r.scrollTop=r.code.scrollTop;
}
function restoreVisibleReading(r,v){
 if(!r.hiddenReading||!r.code.clientWidth)return;
 const reading=r.hiddenReading;
 r.hiddenReading=null;
 r.sourcePane.update(false,true);
 const restored=r.sourcePane.restoreReading(reading);
 r.restoreAnchor=!restored&&r.lines.at(-1)?.number<v.symbol.source.end_line?reading.anchor:null;
 r.scrollTop=r.code.scrollTop;
 recordReading(r);
 if(r.restoreAnchor)void extendSource(r,false);
}
function recordReading(r){
 const v=model.view(r.id),reading=r.hiddenReading??r.sourcePane.reading(),line=(r.restoreAnchor??reading.anchor)?.line;
 const section=r.sections.find(part=>line>=part.start&&line<=part.lines.at(-1)?.number)??r.sections.find(part=>part.start===v.reading.section)??r.sections[0];
 if(!section)return;
 const row=r.code.querySelector(`[data-line="${section.start}"]`);
 if(row&&!r.hiddenReading)reading.top=Math.max(0,reading.top-row.offsetTop+r.code.firstElementChild.offsetTop);
 if(!r.restoreAnchor)model.read(r.id,{section:section.start,...reading});
 const s=v.symbol.source,last=section.lines.at(-1)?.number??section.start;
 delete r.sectionLabel.dataset.actionable;
 r.sectionLabel.textContent=`Lines ${section.start}–${last} · ${s.end_line-s.line+1}-line function`;
 r.prev.hidden=section.start<=s.line;
 r.next.hidden=last>=s.end_line;
}
function sourcePagingNotice(r,text){
 r.sectionLabel.dataset.actionable='';
 r.sectionLabel.textContent=text;
}
function jumpSource(id,before){
 const r=mounted.get(id),v=model.view(id),s=v.symbol.source;
 const section=r?.sections.find(part=>part.start===v.reading.section);
 if(!section?.lines.length)return;
 const start=before?Math.max(s.line,section.start-SECTION_ROWS):section.lines.at(-1).number+1;
 if(start<=s.end_line)void loadSource(id,start);
}
function nearSourceEdge(r,before){
 const code=r.code;
 return before?code.scrollTop<=SECTION_EDGE:code.scrollHeight-code.scrollTop-code.clientHeight<=SECTION_EDGE;
}
async function extendSource(r,before){
 if(!r.sections.length||r.loading||r.adjacent||!r.code.clientHeight||!nearSourceEdge(r,before))return;
 const v=model.view(r.id),s=v.symbol.source;
 if(model.stale&&v.mode==='live'&&!frozenSource(v))return;
 const first=r.sections[0].start,last=r.sections.at(-1).lines.at(-1)?.number;
 if(!last||(before?first<=s.line:last>=s.end_line))return;
 const trimming=r.sections.length>=SECTION_LIMIT;
 if(trimming&&r.sourcePane.selected()){
  sourcePagingNotice(r,'Clear the selection to load more source.');
  return;
 }
 const start=before?Math.max(s.line,first-SECTION_ROWS):last+1;
 const count=before?first-start:Math.min(SECTION_ROWS,s.end_line-start+1);
 const request=new AbortController(),signal=AbortSignal.any([r.controller.signal,request.signal]);
 const ticket=r.sourceTicket,expected=captureFor(v.symbol,v.snapshot);
 const valid=()=>mounted.get(r.id)===r&&r.sourceTicket===ticket&&!signal.aborted;
 r.adjacent=request;
 let added=false;
 try{
  const query=new URLSearchParams({...sourceBinding(v),snapshot:expected,file:s.file,version:s.version,line_start:String(start),line_count:String(count),outline_count:'1',reflow:'true'});
  const source=transport(sourceEndpointForView(v)+'?'+query,{signal});
  const actions=frozenSource(v)?capturedActions(v,start,count,signal):model.usable(r.id)?optionalActions(v.symbol,start,count,v.snapshot,signal):[];
  const [data,inline]=await Promise.all([source,actions]);
  if(!valid()||r.loading||!r.code.clientHeight||!nearSourceEdge(r,before))return;
  if(data.snapshot!==expected||data.file!==s.file||data.version!==s.version||!Array.isArray(data.lines)||data.lines.length!==count||data.lines.some((line,index)=>line.number!==start+index))throw new Error('Adjacent source differs from the requested section');
  if(trimming&&r.sourcePane.selected()){
   sourcePagingNotice(r,'Clear the selection to load more source.');
   return;
  }
  const section={start,lines:data.lines,actions:inline},sections=[...r.sections];
  const removed=trimming?(before?sections.pop():sections.shift()):null;
  if(before)sections.unshift(section);else sections.push(section);
  r.sourceTicket++;
  setSections(r,sections);
  r.sourcePane.extend(section.lines,before,removed?.lines.length??0,r.margin??0);
  if(r.restoreAnchor&&(r.sourcePane.restore(r.restoreAnchor)||r.lines.at(-1).number>=s.end_line))r.restoreAnchor=null;
  r.scrollTop=r.code.scrollTop;
  recordReading(r);geometry();save();
  added=true;
 }catch(error){
  if(valid()){
   r.restoreAnchor=null;recordReading(r);
   sourcePagingNotice(r,'Could not load adjacent source. Scroll again to retry.');
  }
 }finally{
  if(r.adjacent===request)r.adjacent=null;
  if(added&&r.restoreAnchor)void extendSource(r,false);
 }
}
function applyPreparedSource(id,prepared) {
 const r=mounted.get(id),v=model.view(id),s=v.symbol.source,{data,actions,start,margin,openedAtFirstBlock=false}=prepared;
 if(data.file!==s.file||data.version!==s.version)throw new Error('Prepared source differs from the requested fragment');
 r.sourceTicket=(r.sourceTicket??0)+1;r.loading=false;
 resetSource(r,v,data.lines,actions,{...v.reading,section:start},margin);
 sourceStatus(r,data.coloring==='ready'?'Live':'Styling '+data.coloring);recordReading(r);geometry();
 const block=v.comparison?.changes[v.comparison.cursor];
 if(block){
  const line=Math.min(block[0],s.end_line),revealed=r.sourcePane.revealLine(line);
  if(openedAtFirstBlock&&revealed){
   r.openedAtFirstBlock=true;updateFragmentStatus(r);
   if(!r.sourcePane.revealLine(line)){r.openedAtFirstBlock=false;updateFragmentStatus(r);}
  }
  r.scrollTop=r.code.scrollTop;
  recordReading(r);
 }
 if(r.restoreAnchor)void extendSource(r,false);
}
function requestActions(symbol,start,count,snapshot,signal) {
 return requestSourceActions(post,snapshot,{kind:'actions',symbol:symbol.id,line_start:start,line_count:count},signal,symbol.origin?undefined:symbol.source.path);
}
async function optionalActions(symbol,start,count,snapshot,signal){
 try{return await requestActions(symbol,start,count,snapshot,signal);}
 catch(error){if(signal?.aborted)throw error;return [];}
}
async function loadSource(id,start){
 callMenu.hide();const r=mounted.get(id);if(!r)return;const ticket=r.sourceTicket=(r.sourceTicket??0)+1,v=model.view(id),s=v.symbol.source;const expected=captureFor(v.symbol,v.snapshot),sourceEndpoint=sourceEndpointForView(v);const saved={...v.reading};r.adjacent?.abort();r.loading=true;
 try{const query=new URLSearchParams({...sourceBinding(v),snapshot:expected,file:s.file,version:s.version,line_start:String(start),line_count:String(Math.min(96,s.end_line-start+1)),outline_count:'1',reflow:'true'});const header=r.margin===undefined&&start!==s.line?transport(sourceEndpoint+'?'+new URLSearchParams({...sourceBinding(v),snapshot:expected,file:s.file,version:s.version,line_start:String(s.line),line_count:'1',outline_count:'1'}),{signal:r.controller.signal}):Promise.resolve(null);const [data,actions,origin]=await Promise.all([transport(sourceEndpoint+'?'+query,{signal:r.controller.signal}),frozenSource(v)?capturedActions(v,start,Math.min(96,s.end_line-start+1),r.controller.signal):!model.usable(id)?Promise.resolve([]):loadActions(id,start,Math.min(96,s.end_line-start+1),model.snapshot,r),header]);if(mounted.get(id)!==r||r.sourceTicket!==ticket)return;
  if(data.snapshot!==expected||data.file!==s.file||data.version!==s.version||!Array.isArray(data.lines)||data.lines.length>96)throw new Error('Captured source version differs');r.code.replaceChildren();
  if(origin&&(origin.snapshot!==expected||origin.file!==s.file||origin.version!==s.version||origin.lines?.length!==1||origin.lines[0].number!==s.line||origin.lines[0].left||origin.lines[0].column!==0))throw new Error('Fragment indentation source differs');
  if(r.margin===undefined)r.margin=functionIndent(origin?.lines[0]??data.lines.find(line=>line.number===s.line));
  resetSource(r,v,data.lines,actions,{section:start,top:start===saved.section?saved.top:0,left:saved.left,anchor:start===saved.section?saved.anchor:null},r.margin);
  sourceStatus(r,data.coloring==='ready'?'Live':'Styling '+data.coloring);recordReading(r);save();
 }catch(e){if(e.name==='AbortError')return;if(mounted.get(id)===r&&r.sourceTicket===ticket){if(v.history||v.comparison||v.captured){notice.show(e.message);if(r.lines?.length)return;}setSections(r,[]);r.sourcePane.set([]);r.code.replaceChildren(node('p','Source for this bubble is unavailable.'));r.sectionLabel.textContent='Source unavailable';sourceStatus(r,e.message);}}finally{if(mounted.get(id)===r&&r.sourceTicket===ticket){r.loading=false;geometry();live.resume();if(r.restoreAnchor)void extendSource(r,false);}}
}
async function loadActions(id,start,count,snapshot,record){
 try{return await requestActions(model.view(id).symbol,start,count,snapshot,record.controller.signal);}
 catch(error){
  if(error.name!=='AbortError'&&error.status!==409&&mounted.get(id)===record)notice.show('Inline call actions are unavailable for this section. Calls remains available.');
  return [];
 }
}
async function loadCalls(id,offset){
 const r=mounted.get(id);if(!r||!model.navigable(id))return;const v=model.view(id),source=frozenSource(v),expected=source?.snapshot??model.snapshot,binding=JSON.stringify([v.snapshot,v.symbol.id,source]);
 try{const data=source?await post('/api/captured-functions',{source,query:{kind:'calls',offset}},r.controller.signal):await post('/api/structure',{snapshot:expected,path:v.symbol.origin?undefined:v.symbol.source.path,query:{kind:'calls',symbol:v.symbol.id,area:v.area||v.symbol.source.path,offset}},r.controller.signal);if(mounted.get(id)!==r||!model.navigable(id)||JSON.stringify([model.view(id).snapshot,model.view(id).symbol.id,frozenSource(model.view(id))])!==binding)return;if(data.snapshot!==expected||!Array.isArray(data.items)||data.items.length>8)throw new Error('Call response differs');r.sites=offset===0?data.items:[...r.sites,...data.items];r.nextCalls=data.next;r.matching=data.matching;r.callsReady=true;renderCalls(id);if(!source)void prepareDefinitions(id);}catch(error){if(error.name!=='AbortError'){r.callsReady=true;r.popup.replaceChildren(node('p',error.message));}}
}
function callLine(site){return site.identifier_line??site.source.line;}
function renderCalls(id){
 const r=mounted.get(id);if(!r)return;
 const view=model.view(id);
 r.popup.replaceChildren(node('p',`${r.sites.length} of ${r.matching} written calls.`));
 for(const site of r.sites){
  const action=button(`${site.name??'Dynamic call'} · line ${callLine(site)}`,null,()=>chooseCall(id,site));
  if(!canLookInFile(view,site)){r.popup.append(action);continue;}
  const row=node('div');row.className='captured-call-row';
  row.append(action,button('Look in another file…',`Look for ${site.name} in another file`,()=>void chooseCapturedCall(id,site,{anotherFile:true})));
  r.popup.append(row);
 }
 if(r.nextCalls!=null&&r.sites.length<64)r.popup.append(button('More written calls',null,()=>void loadCalls(id,r.nextCalls)));
 if(!r.matching)r.popup.append(node('p','No attributed calls recorded. This does not prove no runtime calls.'));
}
function toggleCalls(id){editorCalls.manual();const r=mounted.get(id);if(!r||!model.navigable(id))return;hideDestinations();for(const[other,x]of mounted)if(other!==id)x.popup.hidden=true;r.popup.hidden=!r.popup.hidden;if(!r.callsReady){r.popup.replaceChildren(node('p','Loading written calls…'));void loadCalls(id,0);}else renderCalls(id);}
function hideDestinations(){el('destination-stack').hidden=true;}
async function chooseCall(id,site,opener){
 editorCalls.manual();const record=mounted.get(id);if(!record||!model.navigable(id))return;if(frozenSource(model.view(id))){void chooseCapturedCall(id,site,{opener});return;}record.resolvingCaptured=false;
 const symbol=model.view(id).symbol;
 site={...site,owner:symbol.id};
 const ticket=record.navigation=(record.navigation??0)+1,expected=model.snapshot;
 const verify=()=>{if(mounted.get(id)!==record||record.navigation!==ticket||model.snapshot!==expected||!model.usable(id))throw new DOMException('Call navigation superseded','AbortError');};
 await definitionCapabilities;
 let choice={site,attempted:definitionProvider&&!site.binding,note:'',semantic:false};
 record.popup.hidden=true;const pending=setTimeout(()=>{if(mounted.get(id)===record&&record.navigation===ticket){record.resolving=true;updateFragmentStatus(record);}},150);
 try {choice=await resolveCallChoice(site,{snapshot:expected,provider:definitionProvider,warmup:()=>definitionWarmup.wait(),request:(snapshot,call,signal)=>post('/api/call-definition',{snapshot,call,path:symbol.origin?undefined:symbol.source.path},signal),signal:record.controller.signal,verify});}
 catch(error){if(error.name==='AbortError')return;if(error.status===409||site.lookup){fail(error);return;}choice.note=error.message;}
 finally{clearTimeout(pending);if(mounted.get(id)===record&&record.navigation===ticket){record.resolving=false;updateFragmentStatus(record);}}
 try{verify();}catch{return;}
 highlightCall(id,choice.site);const target=directDestination(choice);if(target){follow(id,choice.site,target);return;}showDestinations(id,choice);
}
function highlightCall(id,site){const r=mounted.get(id);r.popup.hidden=true;for(const line of r.code.querySelectorAll('.is-call'))line.classList.remove('is-call');r.code.querySelector(`[data-line="${callLine(site)}"]`)?.classList.add('is-call');}
function showDestinations(id,{site,note,semantic,captured}) {
 highlightCall(id,site);const candidates=site.candidates,stack=el('destination-stack'),v=model.view(id);
 stack.replaceChildren(node('h2',site.spelling),node('p',candidates.length?`Choose among ${site.candidate_count} ${semantic?'definition locations':'terminal-name matches'}. ${candidates.length} captured destinations are shown.${note?' '+note:''}`:note||'No indexed destination. Dynamic or external calls remain unresolved.'));stack.append(button('Close choices',null,hideDestinations));
 if(captured?.browse)stack.append(button('Look in another file…',null,captured.browse));
 for(const candidate of candidates){const b=button([...candidate.parents,candidate.name].join('::'),null,()=>captured?captured.open(candidate):follow(id,site,candidate));b.dataset.candidate=candidate.id;b.dataset.path=candidate.source.path;b.append(node('small',`${candidate.origin?originLabel(candidate.origin)+' · ':''}${candidate.source.path}:${candidate.source.line}`));stack.append(b);}Object.assign(stack.style,{left:v.x+v.w+24+'px',top:v.y+'px'});stack.hidden=false;
}
function follow(parent,site,target){editorCalls.manual();try{const id=model.follow(parent,site,target);hideDestinations();const record=mounted.get(id);mount(id);if(target.origin&&record&&(record.element.classList.contains('is-stale')||!record.lines?.length)){void loadSource(id,model.view(id).reading.section);record.callsReady=false;void loadCalls(id,0);}focus([parent,id]);save();}catch(e){fail(e);}}
async function keepLocation(id){
 if(!mounted.has(id)||!board||busy||!model.usable(id))return;
 const {snapshot,symbol}=model.view(id),{file,version,line}=symbol.source;
 if(symbol.origin||!symbol.tracking)return;
 busy=true;geometry();
 try{
  board=(await transport('/api/investigation')).board;
  const result=await post('/api/search/evidence',{
   revision:board.revision,snapshot,file,version,line,title:symbol.name,
   origin:{view:'call_workspace',declaration:symbol.tracking},
  });
  board=result.view.board;notice.show('Location saved.');
 }catch(error){
  fail(error);
  if(error.status===409){
   try{board=(await transport('/api/investigation')).board;}catch(refreshError){fail(refreshError);}
  }
 }finally{busy=false;geometry();live.resume();}
}
async function openEditor(id){const r=mounted.get(id);if(!r||busy||!model.usable(id))return;const s=model.view(id).symbol.source;busy=true;geometry();try{await post(model.view(id).symbol.origin?'/api/dependency/editor':'/api/editor/open',{snapshot:model.view(id).symbol.origin?.capture??model.snapshot,file:s.file,version:s.version,line:s.line});}catch(e){fail(e);}finally{busy=false;geometry();live.resume();}}
function openReferences(id,query){editorCalls.manual();try{if(!model.usable(id))throw new Error('Open a current function before finding references.');const stack=model.addReferences(model.view(id).symbol,query,id);geometry();focus([id,stack]);save();}catch(e){fail(e);}}
function replaceTask(next){
 editorCalls.manual();closeRevisionPicker();closeLineagePicker();closeCrossFilePicker();callMenu.hide();hideDestinations();clearTimeout(livePendingTimer);
 for(const r of mounted.values()){r.controller.abort();r.historyController?.abort();r.sourcePane.dispose();r.element.remove();}
 mounted.clear();referenceStacks.reset();jobs=Promise.resolve();model=next;
 for(const v of model.views)mount(v.id);
 geometry();live.changed(currentSnapshot,true);save();
}
function receive(status){
 el('visual-source-status').textContent=status.snapshot?'Source ready':status.error?'Source unavailable':'Mapping source…';
 if(!status.snapshot)return;
 const changed=currentSnapshot!==status.snapshot;
 currentSnapshot=status.snapshot;
 if(changed)void updateJavaProject(currentSnapshot);
 if(!model){
  try{model=taskControls.load(status.name??'source',currentSnapshot);}catch(e){model=new Workspace(currentSnapshot);fail(e);}
  for(const v of model.views)mount(v.id);geometry();save();
  if(new URLSearchParams(location.search).get('demo')==='minitosis'&&!model.items().length&&!demoOpened){demoOpened=true;void openMinitosis();}
 }else{model.markSnapshot(currentSnapshot);geometry();}
 live.changed(currentSnapshot,model.views.some(v=>v.mode==='live'&&(!v.symbol.tracking||!Object.hasOwn(v.symbol,'module'))),!model.stale&&model.snapshot===currentSnapshot&&model.views.every(v=>v.mode==='pinned'||v.snapshot===currentSnapshot&&v.state==='current'&&v.symbol.tracking&&Object.hasOwn(v.symbol,'module')));
 void editorCalls.resume();
}
function resetFragmentResults(){
 paletteTicket++;
 el('fragment-results').replaceChildren();
 el('fragment-search-status').textContent='';
 el('more-fragment-results').hidden=true;
}
function openFunction(symbol,prepared){
 const path=symbol.source.path,id=model.openRoot(symbol,path.split('/').slice(0,-1).join('/')||path);
 revealBubble(id,prepared);
}
function revealBubble(id,prepared){
 model.selected=id;
 if(prepared&&model.view(id).collapsed)model.collapse(id);
 mount(id,prepared);focus([id]);
}
async function findFragments(offset=0){
 resetFragmentResults();
 const ticket=paletteTicket,snapshot=model?.snapshot;
 const status=el('fragment-search-status'),area=el('fragment-path').value,search=el('fragment-search').value.trim();
 try{
  if(!snapshot||model.stale)throw new Error('Wait for current source or start a new workspace.');
  const data=await post('/api/structure',{snapshot,query:{kind:'inventory',area,search,offset,functions_only:true}});
  if(ticket!==paletteTicket)return;
  if(model.stale||model.snapshot!==snapshot||data.snapshot!==snapshot)throw new Error('Source changed. Search again.');
  if(area&&!data.scope_coverage){status.textContent='No captured file at this path.';return;}
  const coverage=data.scope_coverage??data.coverage;
  const detail=area?(coverage.omitted_symbols||coverage.partial_files?' File search has partial coverage.':'')
   :(coverage.omitted_files||coverage.omitted_symbols?' Repository search is limited. Enter a source file path to search that file.':'');
  status.textContent=`${data.matching} matching functions.${detail}`;
  for(const {symbol} of data.items){
   if(symbol.kind!=='function')continue;
   const b=button([...symbol.parents,symbol.name].join('::'),null,()=>{
    if(model.stale||model.snapshot!==snapshot){resetFragmentResults();status.textContent='Source changed. Search again.';return;}
    editorCalls.manual();
    openFunction(symbol);el('fragment-dialog').close();
   });
   b.dataset.addSymbol=symbol.id;b.dataset.name=symbol.name;
   b.append(node('small',`${symbol.source.path}:${symbol.source.line}`));el('fragment-results').append(b);
  }
  const more=el('more-fragment-results');more.hidden=data.next===null;more.dataset.offset=String(data.next??0);
 }catch(e){if(ticket===paletteTicket)status.textContent=e.message;}
}
async function openMinitosis(){try{if(!model||model.stale)throw new Error('Wait for current source.');const data=await post('/api/structure',{snapshot:model.snapshot,query:{kind:'inventory',area:'minitosis/src/lib.rs',search:'process',offset:0}});const s=data.items.map(i=>i.symbol).find(s=>s.name==='process'&&s.kind==='function');if(!s)throw new Error('Minitosis example is not in this source workspace. Use Add bubble.');const id=model.openRoot(s,'minitosis/src');mount(id);focus([id]);save();}catch(e){fail(e);}}
const openDialog=()=>{editorCalls.manual();el('fragment-dialog').showModal();el('fragment-search').focus();};el('add-fragment').onclick=openDialog;el('empty-add').onclick=openDialog;el('close-fragment-dialog').onclick=()=>el('fragment-dialog').close();el('fragment-form').onsubmit=e=>{e.preventDefault();void findFragments();};el('more-fragment-results').onclick=()=>void findFragments(Number(el('more-fragment-results').dataset.offset));
el('fragment-form').addEventListener('input',resetFragmentResults);
el('fragment-dialog').addEventListener('close',resetFragmentResults);
el('fit-workspace').onclick=()=>{if(!model)return;const s=size();model.fit(s.w,s.h);geometry();save();};for(const[id,factor]of[['zoom-in',1.2],['zoom-out',1/1.2]])el(id).onclick=()=>{if(!model)return;const s=size();model.zoomAt(model.camera.scale*factor,{x:s.w/2,y:s.h/2});geometry();save();};
const canvas=el('workspace-canvas');canvas.addEventListener('pointerdown',e=>{if(e.button!==0||e.target.closest('.fragment,.reference-stack,.task-group-header,#destination-stack,#workspace-overview,#visual-empty,[data-link-id]'))return;if(!model)return;e.preventDefault();canvas.setPointerCapture(e.pointerId);let last={x:e.clientX,y:e.clientY};const move=event=>{model.pan(event.clientX-last.x,event.clientY-last.y);last={x:event.clientX,y:event.clientY};geometry();};const end=()=>{canvas.removeEventListener('pointermove',move);canvas.removeEventListener('pointerup',end);canvas.removeEventListener('pointercancel',end);save();};canvas.addEventListener('pointermove',move);canvas.addEventListener('pointerup',end);canvas.addEventListener('pointercancel',end);});
canvas.addEventListener('wheel',e=>{callMenu.hide();if(!model||e.target.closest('.code-pane,.reference-body,.calls-popover,#destination-stack'))return;e.preventDefault();if(e.ctrlKey||e.metaKey){const rect=canvas.getBoundingClientRect();model.zoomAt(model.camera.scale*Math.exp(-e.deltaY/500),{x:e.clientX-rect.left,y:e.clientY-rect.top});}else model.pan(-e.deltaX,-e.deltaY);geometry();save();},{passive:false});canvas.addEventListener('keydown',e=>{if(e.target!==canvas||!model)return;const moves={ArrowLeft:[70,0],ArrowRight:[-70,0],ArrowUp:[0,70],ArrowDown:[0,-70]};if(moves[e.key]){e.preventDefault();model.pan(...moves[e.key]);geometry();save();}if(e.key==='Escape')hideDestinations();});new ResizeObserver(()=>{geometry();void editorCalls.resume();}).observe(canvas);
const referenceStacks=installReferenceStacks(el('reference-layer'),{
 read:()=>model,post,transport,changed:()=>{geometry();save();},failed:fail,comparisonTitle,
 open:(id,row,data)=>{editorCalls.manual();try{if(data.snapshot!==model.snapshot||model.stale)throw new Error('Source changed. Refresh references.');const opened=model.openReference(id,row,data.target);for(const v of model.views)mount(v.id);focus([...(opened.destination?[opened.destination]:[]),opened.caller]);save();}catch(e){fail(e);}},
 revealComparison:ids=>{
  editorCalls.manual();
  try{
   const viewport=size();
   model.revealComparison(ids,viewport.w,viewport.h);
   geometry();save();
   mounted.get(model.selected)?.code.focus({preventScroll:true});
  }catch(error){fail(error);}
 },
});
const taskGroups=installTaskGroups(el('task-group-layer'),{read:()=>model,changed:()=>{geometry();save();},edit:id=>taskControls.showGroups(id),failed:fail});
taskControls=installTaskControls({read:()=>model,replace:replaceTask,snapshot:()=>currentSnapshot,changed:()=>{geometry();save();},failed:fail});
void transport('/api/investigation').then(view=>{board=view.board;editorName=view.editor;editorJavaArchives=view.editor_java_archives===true;for(const r of mounted.values()){r.editor.setAttribute('aria-label',`Open in ${editorName}`);r.editor.title=`Open in ${editorName}`;}geometry();}).catch(fail);
followChanges({session:()=>({receive(change){receive(change.status);return true;},close(e){if(e)notice.show('Source updates disconnected; the visual layout is retained.');}})});
void post('/api/workspace/start',{}).catch(fail);
