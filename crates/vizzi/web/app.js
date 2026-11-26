import {installAppHeader} from '/app-header.js';
import {headingPlacement} from "/map-headings.js";
import {workspaceLabel} from "/view-status.js";
import {createMapModel} from "/map-scene.js";
import {createDirectoryControls} from "/directory-controls.js";
import {createMessage,textRow} from "/messages.js";
import {installAppearance,mapFileTypes,mapPalette} from "/appearance.js";
import {coverageLabel,fileCount} from "/presentation.js";
import {fitCamera,zoomCamera,jumpCamera} from "/camera.js";
import {installSplitter} from "/splitter.js";
import { createController } from "/controller.js";
import { followChanges } from "/updates.js";
import { describeSource } from "/source-description.js";
import { createInspector } from "/inspector.js";
import { createNavigation } from "/navigation.js";
import { createStartup } from "/startup.js";
import {installWorkspaceDelivery} from "/editor-delivery.js";
import {createCallMenu,requestSourceActions} from "/call-actions.js";
import {fetchRead} from "/read-retry.js";

const header = document.querySelector('body > header');
installAppHeader(header);
const element = id => document.getElementById(id);
const pathFilter = element("filter"), clearFilters = element("clear-filters");
const searchWithin = element("search-within"), matchPanel = element("source-matches");
const searchRegex = element("search-regex"), searchCase = element("search-match-case");
const searchError = element("search-error");
const canvas = element("map"), context = canvas.getContext("2d");
const overview=element("map-overview"),overviewContext=overview.getContext("2d");
let overviewTransform,overviewDrag=false;
installSplitter(element("explore-workspace"),{key:"explore",label:"Repository map and source widths",initial:.43,minBefore:240,minAfter:320});
const outline = element("outline"), outlinePane = outline.closest(".outline");
const sourceLayout = outlinePane.parentElement;
installSplitter(sourceLayout,{key:"explore-outline",label:"Declarations and source widths",initial:.3,minBefore:100,minAfter:200});
const outlineDivider = sourceLayout.querySelector(".splitter");
const outlineStatus = element("outline-status");
outlinePane.hidden = true;
const source = element("source"), space = element("source-space");
const callMenu = createCallMenu(element("call-action-menu"));
source.addEventListener("wheel",()=>callMenu.hide(),{passive:true});
source.addEventListener("touchmove",()=>callMenu.hide(),{passive:true});
const keepEvidence=document.createElement("button");keepEvidence.type="button";keepEvidence.id="keep-evidence";keepEvidence.textContent="Save location";keepEvidence.disabled=true;element("snapshot").before(keepEvidence);
keepEvidence.addEventListener("click", async () => {
  const captured = view;
  if (!captured?.source) return;
  keepEvidence.disabled = true;
  try {
    const response = await fetch('/api/investigation');
    if (!response.ok) throw new Error(await response.text());
    const state = await response.json();
    const {id, version, path, summary: {active}} = captured.source;
    const kept = await fetch('/api/investigation/evidence', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({
        revision: state.board.revision, snapshot: captured.snapshot, file: id, version,
        line: active?.start_line ?? 1, title: active?.name ?? path, origin: {view: 'source_browser'},
      }),
    });
    if (!kept.ok) throw new Error(await kept.text());
    location.assign('/investigate#saved');
  } catch (error) {
    showError(error);
    keepEvidence.disabled = !view?.source;
  }
});
const metrics = { drawCpuMs: [], interactionFrameMs: [], requestMs: [], pickingMs: [], lastFrame: null };
window.vizziMetrics = metrics;
function sample(series, value) { series.push(value); if (series.length > 512) series.shift(); }
let view = null, camera = { x: 20, y: 20, scale: 1 };
let scheduled = false, drag = null, previousFrame = null, filterTimer = null,fitMode="readable",mapSize;
let updates, viewFilter = "",mapModel,scene,mapWorkspace;
let viewTypes = "", selectedTypes = new Set();
let browseMap, searchOrigin, viewSearchDirectory = null;
let viewSearchRegex = false, viewSearchCase = false;
const dismissible=new Set(["navigation-notice","error","map-notice"]);
const messages=Object.fromEntries([["status","workspace status"],["navigation-notice","navigation notice"],["coverage","source status"],["diagnostics","source diagnostics"],["error","workspace error"],["map-notice","map notice"]].map(([id,label])=>[id,dismissible.has(id)?createMessage(element(id),{label:`Dismiss ${label}`}):textRow(element(id))]));
let palette=mapPalette(canvas);
const directories=createDirectoryControls({
  section:element("map-directories"),toggle:element("toggle-directories"),tree:element("directory-tree"),
  breadcrumbs:element("map-breadcrumbs"),onOpen:openDirectory,onChange:()=>{refreshScene();saveMap();fit(true);},
});
function refreshScene() {
  const projection = mapModel.project();
  const files = projection.files.map(file => ({ ...file, type: file.file_type, marker: coverageMarker(file) }));
  scene = { ...projection, files };
  directories.apply(mapModel);
  metrics.scene = scene;
  renderCounts();
  renderLandmarks();
  renderMapLegend();
}
function coverageMarker(file) {
  if (file.coverage === "partial") return "partial";
  if (file.coverage === "deferred" || (file.coverage === "unavailable" && file.classification?.kind !== "source_only")) {
    return "unavailable";
  }
  return null;
}
function fileDescription(file) {
  const type = mapFileTypes[file.file_type];
  const coverage = coverageLabel(file.coverage, file.classification?.kind === "source_only");
  return `${type} · ${coverage}`;
}
function renderMapLegend() {
  const legend = element("map-legend");
  const types = new Set(view?.file_types);
  const counts = view?.file_type_counts ?? {};
  for (const type of selectedTypes) types.add(type);
  const markers = new Set(scene?.files.map(file => file.marker));
  const entries = Object.entries(mapFileTypes)
    .filter(([type]) => types.has(type))
    .sort(([leftType, leftLabel], [rightType, rightLabel]) =>
      (counts[rightType] ?? 0) - (counts[leftType] ?? 0) || leftLabel.localeCompare(rightLabel))
    .map(([type, label]) => [label, palette.types[type], type]);
  if (markers.has("partial")) entries.push(["Syntax recovery", palette.partial]);
  if (markers.has("unavailable")) entries.push(["Analysis deferred or unavailable", palette.unavailable]);
  const key = JSON.stringify(entries);
  const focused = document.activeElement;
  const focusedType = legend.contains(focused) ? focused.dataset.mapType : undefined;
  const rebuild = legend.dataset.entries !== key;
  if (rebuild) {
    legend.dataset.entries = key;
    legend.hidden = !entries.length;
    legend.replaceChildren(...entries.map(([label, color, type]) => {
      const item = document.createElement("li");
      item.style.setProperty("--swatch", color);
      if (type) {
        const button = document.createElement("button");
        button.type = "button";
        button.dataset.mapType = type;
        button.textContent = label;
        button.addEventListener("click", () => {
          if (!selectedTypes.delete(type)) selectedTypes.add(type);
          renderMapLegend();
          void navigate({});
        });
        item.append(button);
      } else {
        item.className = "map-status";
        item.textContent = label;
      }
      return item;
    }));
  }
  const buttons = [...legend.querySelectorAll("button")];
  for (const button of buttons) {
    button.setAttribute("aria-pressed", String(selectedTypes.has(button.dataset.mapType)));
  }
  if (rebuild && focusedType) {
    const nextFocus = buttons.find(button => button.dataset.mapType === focusedType) ?? buttons[0] ?? pathFilter;
    nextFocus.focus({ preventScroll: true });
  }
}
function updateClearFilters() {
  clearFilters.hidden = !pathFilter.value && !selectedTypes.size;
}
function renderCounts(){
  if(!view)return;
  const page=view.browse;
  const count=page?fileCount(page.files,page.files,view.total):fileCount(view.files.length,view.matching,view.total);
  element("counts").textContent=count+(!page && scene?.focus!=="."?` · ${scene?.files.length ?? 0} drawn here`:"")+
    (scene?.hiddenFiles?` · ${scene.hiddenFiles} inside groups`:"")+(view.deferred?` · ${view.deferred} awaiting analysis`:"");
}
function saveMap(){if(view?.matches)return;try{const value=JSON.stringify(mapModel.preferences());if(new TextEncoder().encode(value).length>65536)throw new Error("layout size");localStorage.setItem(`vizzi.map.v1.${mapWorkspace}`,value);messages["map-notice"].show(null);}catch{messages["map-notice"].show("Map preferences could not be saved. The current layout is kept until reload.");}}
element("reset-map").addEventListener("click",()=>{if(!mapModel)return;if(view.browse){openDirectory(".");return;}mapModel.reset();refreshScene();saveMap();fit(true);});
installAppearance(element("theme"),{onChange:()=>{palette=mapPalette(canvas);renderMapLegend();invalidate();}});
const fetchJSON = async (uri, options) => {
  const start = performance.now();
  try { const response = await fetch(uri, options); if (!response.ok) throw new Error(await response.text()); return await response.json(); }
  finally { sample(metrics.requestMs, performance.now() - start); }
};
const startup=createStartup({transport:fetchJSON,onError:showError,onState:state=>{
  const button=element("start-index");button.hidden=!state.action;button.disabled=state.busy;button.textContent=state.action ?? "Start indexing";
  if(state.initial){messages.status.show(state.label);element("counts").textContent=state.label;}
}});
element("start-index").addEventListener("click",()=>startup.activate());
const inspector = createInspector({
  source,space,outline,transport:fetchJSON,
  onError:error=>{showError(error);updates?.restart();},
  select:async fact=>{if(await navigate({entity:fact.id}))await inspector.revealActive();},
  loadActions:async (captured,window,signal)=>{
    if(!captured.source.summary.declarations)return [];
    const post=async (path,body,signal)=>{
      const response=await fetchRead(path,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(body),signal});
      if(!response.ok)throw new Error("Captured source actions are unavailable");
      return response.json();
    };
    return requestSourceActions(post,captured.snapshot,{kind:"source_actions",file:captured.source.id,line_start:window.line_start,line_count:window.lines.length},signal);
  },
  activateAction:(captured,action,trigger,direct,keyboard)=>{
    editorFiles.manual();
    const isFunction=action.kind==="function";
    const open=()=>{
      if(!trigger.isConnected || captured.snapshot!==view?.snapshot || captured.source.id!==view.source?.id)return;
      const hash=new URLSearchParams({[isFunction?"source-function":"source-call"]:action.id,snapshot:captured.snapshot,file:captured.source.id});
      location.assign(`/workspace#${hash}`);
    };
    if(direct){callMenu.hide();open();return;}
    callMenu.show(trigger,action.name,[{label:isFunction?"Open in Call workspace":"Open definition in Call workspace",hint:"Ctrl/⌘-click",run:open}],keyboard);
  },
});

const mapPosition = () => ({camera:{...camera},layoutRevision:mapModel?.revision,fitMode,mapSize});
function restoreMapPosition(position) {
  camera={...position.camera};
  fitMode=position.fitMode ?? null;
  const old=position.mapSize;
  mapSize={width:canvas.clientWidth,height:canvas.clientHeight};
  if(old)camera=jumpCamera(camera,{x:(old.width/2-camera.x)/camera.scale,y:(old.height/2-camera.y)/camera.scale},mapSize.width,mapSize.height);
  invalidate();
}
const navigation = createNavigation({
  capture: () => ({ file:view?.selection.file ?? null,entity:view?.selection.entity ?? null,
    path:view?.source?.path ?? null,version:view?.source?.version ?? null,snapshot:view?.snapshot ?? null,
    filter:viewFilter,types:viewTypes,directory:view?.browse?.directory ?? ".",offset:view?.browse?.offset ?? 0,
    find:true,searchDirectory:viewSearchDirectory,searchRegex:viewSearchRegex,searchCaseSensitive:viewSearchCase,
    resultsOffset:view?.matches?.offset ?? 0,searchOrigin,
    position:{inspector:inspector.position(),...mapPosition()} }),
  load: (query,intent) => controller.load(query,{intent,rollbackOnFailure:Boolean(intent.returning || intent.anchor)}),
  prepare: (query,intent,signal) => controller.prepare(query,{intent,signal,rollbackOnFailure:true}),
  history: {push:state=>history.pushState(state,""),replace:state=>history.replaceState(state,""),go:delta=>history.go(delta)},
  onState: state => {element("nav-back").disabled=!state.canBack;element("nav-forward").disabled=!state.canForward;element("navigation").setAttribute("aria-busy",String(state.pending));},
  onNotice: notice => messages["navigation-notice"].show(notice),
});
function selectedTypeQuery() { return [...selectedTypes].sort().join(","); }
function searchText() { return searchRegex.checked ? pathFilter.value : pathFilter.value.trim(); }
function draftSearch() {
  const filter=searchText();
  return {filter,types:selectedTypeQuery(),
    search_directory:filter && searchWithin.checked ? searchOrigin?.directory ?? null : null,
    search_regex:searchRegex.checked,search_case_sensitive:searchCase.checked};
}
function searchChanged({filter="",types="",search_directory=null,search_regex=false,search_case_sensitive=false}) {
  return filter!==viewFilter || types!==viewTypes || search_directory!==viewSearchDirectory ||
    search_regex!==viewSearchRegex || search_case_sensitive!==viewSearchCase;
}
function navigate(patch, anchor) {
  editorFiles.manual();
  clearTimeout(filterTimer);
  updateClearFilters();
  const filter=searchText();
  if(filter && !searchOrigin)searchOrigin={directory:scene?.focus ?? ".",offset:view?.browse?.offset ?? 0,position:mapPosition()};
  const query=draftSearch(), changed=searchChanged(query);
  if(changed)renderSearchError();
  const map=filter && changed ? {directory:query.search_directory ?? ".",offset:0}
    : !filter && searchOrigin ? {directory:searchOrigin.directory,offset:searchOrigin.offset} : {};
  return navigation.navigate({...query, ...map,
    ...(changed ? {results_offset:0} : {}), ...patch},anchor,{replaceHistory:false});
}
function openDirectory(directory) {
  if(view?.matches && view.browse){
    searchWithin.checked=directory===searchOrigin?.directory && directory!==".";
    return navigate({offset:0});
  }
  pathFilter.value="";
  return navigate({directory,offset:0});
}
function changePage(direction) {
  const page=view?.browse;
  if(!page || searchChanged(draftSearch()))return;
  return navigate({offset:Math.max(0,page.offset+direction*page.size)});
}
const controller = createController({
  viewSchema: 3,
  acceptView: (next,info) => navigation.allow(next,info.intent),
  transport: async (uri, options) => {
    const url=new URL(uri,location.origin);
    url.searchParams.set("projection","summary");
    if(!url.searchParams.has("directory"))url.searchParams.set("directory",".");
    const current=view, file=url.searchParams.get("file"), snapshot=url.searchParams.get("snapshot");
    if(current && file && file!==current.selection.file && snapshot===current.snapshot &&
       options.intent?.before && !options.intent.anchor && !url.searchParams.has("entity") &&
       !searchChanged(options.query) &&
       Number(url.searchParams.get("results_offset"))===(current.matches?.offset ?? 0) &&
       url.searchParams.get("directory")===(current.browse?.directory ?? ".") &&
       Number(url.searchParams.get("offset"))===(current.browse?.offset ?? 0)) {
      const query=new URLSearchParams({file,snapshot});
      const {source,window:initialWindow}=await fetchJSON(`/api/inspection?${query}`,options);
      if(source?.id!==file || initialWindow?.line_start!==1 || initialWindow?.outline_start!==0)
        throw new Error("Mismatched initial inspection");
      const next={...current,source,selection:{file,entity:null}};
      return inspector.prepare(next,{...options,initialWindow});
    }
    const next=await fetchJSON(url,options);
    const restore=navigation.restoration(next,options.intent);
    return inspector.prepare(next,{...options,restore:restore?.position.inspector,line:options.intent?.anchor?.line,reveal:Boolean(options.intent?.anchor || options.intent?.returning && !restore)});
  },
  onView: (next,info) => {
    const previousSelection=view?.selection.file;
    const wasSearching=Boolean(view?.matches), searching=Boolean(next.matches);
    const types=info.query.types ?? "";
    const first = !view, changedMapFilter=searchChanged(info.query) && (searching || wasSearching || viewTypes!==types);
    const changedMap=view?.files!==next.files;
    const changedPage=view?.browse?.directory!==next.browse?.directory || view?.browse?.offset!==next.browse?.offset;
    view = next;keepEvidence.disabled=!next.source;
    viewFilter=info.query.filter ?? "";
    viewTypes=types;
    viewSearchDirectory=info.query.search_directory ?? null;
    viewSearchRegex=info.query.search_regex ?? false;
    viewSearchCase=info.query.search_case_sensitive ?? false;
    selectedTypes=new Set(viewTypes ? viewTypes.split(",") : []);
    if(info.intent?.returning || info.intent?.editor){
      pathFilter.value=viewFilter;
      searchRegex.checked=viewSearchRegex;
      searchCase.checked=viewSearchCase;
    }
    if(info.intent?.returning && searching)searchOrigin=info.intent.target.searchOrigin;
    const leavingSearch=searching ? null : searchOrigin;
    const restoreSearchMap=wasSearching && leavingSearch && !info.intent?.returning && !info.intent?.anchor;
    if(!searching && !searchText())searchOrigin=null;
    searchWithin.checked=Boolean(viewSearchDirectory);
    updateClearFilters();
    if(mapWorkspace!==next.workspace){
      let preferences={};
      try{const saved=localStorage.getItem(`vizzi.map.v1.${next.workspace}`);if(saved && saved.length<=65536 && new TextEncoder().encode(saved).length<=65536)preferences=JSON.parse(saved);}catch{}
      browseMap=createMapModel(searching?[]:next.files,preferences,searching?null:next.browse);
      mapModel=searching?createMapModel(next.files,{},next.browse):browseMap;
      mapWorkspace=next.workspace;
    }
    else if(searching!==wasSearching){
      mapModel=searching?createMapModel(next.files,{},next.browse):browseMap;
      if(!searching)mapModel.setFiles(next.files,next.browse);
    }
    else if(changedMap)mapModel.setFiles(next.files,next.browse);
    if(restoreSearchMap && !next.browse && !mapModel.focus(leavingSearch.directory))mapModel.focus(".");
    if(searching && changedMapFilter && !next.browse)mapModel.focus(".");
    const layoutRevision=mapModel.revision;
    const revealSelection=!restoreSearchMap && (!first && previousSelection!==next.selection.file || info.intent?.anchor);
    if(revealSelection)mapModel.reveal(next.selection.file,{focusDeep:searching});
    if(changedMap || mapModel.revision!==layoutRevision)refreshScene();
    element("empty").hidden = Boolean(next.matches?.error) || (next.browse?.entries ?? next.files.length) > 0;
    renderPage();
    renderMatches(next);
    element("file-title").textContent = next.source?.path ?? "Choose a file";
    element("snapshot").textContent = next.snapshot.slice(0, 8);
    element("snapshot").title = `Captured observation ${next.snapshot}`;
    const { coverage, diagnostics } = describeSource(next.source);
    messages.coverage.show(coverage,next.source?.id);
    messages.diagnostics.show(diagnostics.join(". "),next.source?.version);
    const hideOutline = !next.source?.summary.declarations;
    const outlineMessage = next.source && hideOutline ? "No declarations extracted" : "";
    if (outlineStatus.textContent !== outlineMessage) outlineStatus.textContent = outlineMessage;
    const focused = document.activeElement;
    if (hideOutline && (outlinePane.contains(focused) || focused === outlineDivider)) {
      source.focus({ preventScroll: true });
    }
    outlinePane.hidden = hideOutline;
    callMenu.hide();inspector.apply(next); renderFileList();
    const restore=navigation.restoration(next,info.intent);
    const sameScene=restore && restore.snapshot===next.snapshot && restore.position.layoutRevision===mapModel.revision &&
      !searchChanged({filter:restore.filter,types:restore.types,search_directory:restore.searchDirectory,
        search_regex:restore.searchRegex,search_case_sensitive:restore.searchCaseSensitive}) &&
      restore.directory===(next.browse?.directory ?? ".") && restore.offset===(next.browse?.offset ?? 0);
    if(sameScene)restoreMapPosition(restore.position);
    else if(restoreSearchMap)restoreMapPosition(leavingSearch.position);
    else if(first || changedMapFilter || changedPage || info.intent?.returning)fit(true);else invalidate();
    if(revealSelection && !first && !info.intent?.returning){const file=scene.files.find(f=>f.id===next.selection.file);if(file){const x=camera.x+(file.rect.x+file.rect.width/2)*camera.scale,y=camera.y+(file.rect.y+file.rect.height/2)*camera.scale;if(x<12 || y<12 || x>canvas.clientWidth-12 || y>canvas.clientHeight-12){camera=jumpCamera(camera,{x:file.rect.x+file.rect.width/2,y:file.rect.y+file.rect.height/2},canvas.clientWidth,canvas.clientHeight);fitMode=null;invalidate();}}}
    if(info.intent || !editorFiles.pending())navigation.observe(info.intent);
    void editorFiles.resume();
  },
  onStatus: (status,info) => {
    startup.receive(status,info.stream);
    element("workspace").textContent = status.name;
    if(status.snapshot!==null)messages.status.show(workspaceLabel(status));
    element("status").title = status.observation;
    element("explore-workspace").setAttribute("aria-busy",String(status.pending));
    if (status.error && !(status.snapshot===null && status.pending)) showError(new Error(status.error));
    else messages.error.show(null);
  },
  onError: showError,
  onResync: () => updates?.restart(),
});
const editorFiles=installWorkspaceDelivery({mode:'file',
  ready:action=>action.kind==='reveal'
    ? header.clientWidth>0 && header.clientHeight>0
    : Boolean(view && canvas.clientWidth>0 && source.clientHeight>0),
  prepare:async(action,{signal,replaceHistory})=>{
    if(action.kind==='reveal')return {commit(){}};
    clearTimeout(filterTimer);
    return navigation.prepare({file:action.file,entity:null,filter:'',types:'',search_directory:null,results_offset:0},action,{signal,replaceHistory});
  },failed:showError,changed:pending=>{if(!pending && view)navigation.observe();}});
function showError(error) { messages.error.show(`${error.message || error} · ${view?"Keeping the last captured view; retrying.":"No source observation is available. Retry indexing."}`); }

function renderLandmarks() {
  const list=element("landmark-files");
  const entrySelect=element("entry-point-landmarks");
  const prefix=scene.focus==="."?"":`${scene.focus}/`;
  const kind=file=>file.kind ?? file.landmark;
  const files=viewFilter?[]:view.browse?.landmarks ?? view.files.filter(file=>
    file.landmark && file.path.startsWith(prefix) &&
    (kind(file)==="entry_point" || !file.path.slice(prefix.length).includes("/")));
  const key=JSON.stringify(files.map(file=>[file.id,file.path,kind(file)]));
  if(list.dataset.files===key)return;
  list.dataset.files=key;
  const entryPoints=files.filter(file=>kind(file)==="entry_point").sort((a,b)=>a.path.localeCompare(b.path));
  const isReadme=file=>kind(file)==="readme";
  const readme=files.find(isReadme);
  const landmarks=files.filter(file=>kind(file)==="manifest");
  if(readme)landmarks.unshift(readme);
  element("map-landmarks").hidden=!landmarks.length && !entryPoints.length;
  list.hidden=!landmarks.length;
  entrySelect.hidden=!entryPoints.length;
  const options=document.createDocumentFragment();
  options.append(new Option(`Entry points (${entryPoints.length.toLocaleString()})`, ""));
  for(const file of entryPoints)options.append(new Option(file.path,file.id));
  entrySelect.replaceChildren(options);
  list.replaceChildren(...landmarks.map(file=>{
    const button=document.createElement("button");
    button.type="button";
    button.textContent=file.path.split("/").at(-1);
    button.title=file.path;
    button.onclick=()=>navigate({file:file.id,entity:null});
    return button;
  }));
}
element("entry-point-landmarks").addEventListener("change",event=>{
  const select=event.currentTarget,file=select.value;
  select.value="";
  if(file)navigate({file,entity:null});
});

function renderPage() {
  const page=view.browse,pager=element("map-pages");
  pager.hidden=!page || page.entries<=page.size;
  if(pager.hidden)return;
  const end=Math.min(page.offset+page.size,page.entries);
  const range=`${(page.offset+1).toLocaleString()}–${end.toLocaleString()} of ${page.entries.toLocaleString()} ${viewFilter?"matches":"entries"}`;
  if(element("map-page-range").textContent!==range)element("map-page-range").textContent=range;
  element("map-page-previous").disabled=page.offset===0;
  element("map-page-next").disabled=end===page.entries;
}

function renderSearchScope() {
  const directory=searchOrigin?.directory ?? scene?.focus ?? ".";
  const label=element("search-directory");
  element("search-scope").hidden=!searchText() || directory===".";
  label.textContent=directory;
  label.title=directory;
}

function renderSearchError(error) {
  const message=error ? `Invalid regex. ${error}` : "";
  if(searchError.textContent!==message)searchError.textContent=message;
  if(error)pathFilter.setAttribute("aria-invalid","true");
  else pathFilter.removeAttribute("aria-invalid");
}

function renderMatches(next) {
  renderSearchScope();
  const page=next.matches;
  matchPanel.hidden=!page || Boolean(page.error);
  renderSearchError(searchChanged(draftSearch()) ? undefined : page?.error);
  if(matchPanel.hidden)return;
  const list=element("match-list");
  const key=JSON.stringify([next.snapshot,viewFilter,viewTypes,viewSearchDirectory,viewSearchRegex,viewSearchCase,page.offset]);
  if(list.dataset.page!==key){
    list.dataset.page=key;
    list.replaceChildren(...page.items.map(result=>{
      const row=document.createElement("li"),button=document.createElement("button");
      button.type="button";
      button.className="match-row";
      const kind=document.createElement("span"),name=document.createElement("code"),path=document.createElement("small");
      kind.classList.add("kind-badge");
      if(["function","module","type","constant"].includes(result.kind))kind.classList.add(`kind-${result.kind}`);
      kind.textContent=result.kind;
      name.textContent=result.name;
      path.textContent=result.entity ? `${result.path}:${result.line}` : result.path;
      button.title=`${result.name}\n${path.textContent}`;
      button.append(kind,name,path);
      button.onclick=()=>{
        if(searchChanged(draftSearch()))return;
        void navigate({file:result.file,entity:result.entity,reveal_match:true},
          {snapshot:next.snapshot,file:result.file,version:result.version,entity:result.entity,line:result.line});
      };
      row.append(button);
      return row;
    }));
    list.scrollTop=0;
  }
  page.items.forEach((result,index)=>{
    const active=result.file===next.selection.file && result.entity===next.selection.entity;
    list.children[index].firstElementChild.setAttribute("aria-current",active?"location":"false");
  });
  const end=page.offset+page.items.length;
  const count=page.total>page.size ? `${page.offset+1}–${end} of ${page.total.toLocaleString()}`
    : page.total ? page.total.toLocaleString() : "No matches";
  if(element("match-count").textContent!==count)element("match-count").textContent=count;
  element("match-pages").hidden=page.total<=page.size;
  const focused=document.activeElement;
  element("match-previous").disabled=page.offset===0;
  element("match-next").disabled=end>=page.total;
  if(focused?.disabled && focused.parentElement===element("match-pages"))list.querySelector("button")?.focus();
}

function changeMatchPage(direction) {
  const page=view?.matches;
  if(page && !searchChanged(draftSearch()))void navigate({results_offset:Math.max(0,page.offset+direction*page.size)});
}

function renderFileList() {
  const list = element("file-list"); list.replaceChildren();
  if (!list.closest("details").open || !view) return;
  for (const file of view.files) {
    const button = document.createElement("button"); button.textContent = file.path;
    button.setAttribute("aria-current", String(file.id === view.selection.file));
    button.setAttribute("aria-description", fileDescription(file));
    button.addEventListener("click", () => navigate({ file: file.id, entity: null })); list.append(button);
  }
}
function invalidate() { if (!scheduled) { scheduled = true; requestAnimationFrame(draw); } }
function fit(readable = false) {
  if (!view) return;
  fitMode=readable?"readable":"overview";
  const bounds=worldBounds();
  camera = fitCamera(bounds.width,bounds.height,canvas.clientWidth,canvas.clientHeight,readable);
  invalidate();
}
function drawDirectorySummary(group,x,y,width,height) {
  const available=width-12;
  if(available<18 || height<18)return;
  let label=group.label;
  context.font="600 11px 'DejaVu Sans', sans-serif";
  if(context.measureText(label).width>available){
    const characters=Array.from(label);
    let low=0,high=characters.length;
    while(low<high){
      const middle=Math.ceil((low+high)/2);
      if(context.measureText(characters.slice(0,middle).join("")+"…").width<=available)low=middle;
      else high=middle-1;
    }
    label=characters.slice(0,low).join("")+"…";
  }
  context.fillStyle=palette.text;
  context.fillText(label,x+6,y+14);
  if(height>=33){
    context.font="10px 'DejaVu Sans', sans-serif";
    const count=`${group.count.toLocaleString()} ${group.count===1?"file":"files"}`;
    context.fillText(count,x+6,y+27,available);
  }
}
function drawFileTile(ctx, file, x, y, width, height) {
  ctx.fillStyle = palette.types[file.type];
  ctx.fillRect(x, y, width, height);
  if (file.id === view.selection.file) {
    const stroke = Math.min(3, width / 3, height / 3);
    ctx.strokeStyle = palette.focus;
    ctx.lineWidth = stroke;
    ctx.strokeRect(x + stroke / 2, y + stroke / 2, width - stroke, height - stroke);
  }
  if (!file.marker) return;

  ctx.fillStyle = palette[file.marker];
  const corner = Math.min(10, Math.min(width, height) / 3);
  if (corner < 4) {
    const bandWidth = Math.min(8, width * 2 / 3), bandHeight = Math.min(2, height);
    ctx.fillRect(x + width - bandWidth, y + height - bandHeight, bandWidth, bandHeight);
    return;
  }
  const right = x + width - 3, bottom = y + height - 3;
  ctx.beginPath();
  ctx.moveTo(right, bottom - corner);
  ctx.lineTo(right, bottom);
  ctx.lineTo(right - corner, bottom);
  ctx.fill();
}
function draw(timestamp) {
  scheduled = false; const start = performance.now();
  if (drag && previousFrame !== null) sample(metrics.interactionFrameMs, timestamp - previousFrame);
  previousFrame = drag ? timestamp : null;
  const width = canvas.clientWidth, height = canvas.clientHeight, dpr = Math.min(window.devicePixelRatio || 1, 2);
  if (canvas.width !== Math.round(width * dpr) || canvas.height !== Math.round(height * dpr)) { canvas.width = Math.round(width * dpr); canvas.height = Math.round(height * dpr); }
  context.setTransform(dpr, 0, 0, dpr, 0, 0); context.clearRect(0, 0, width, height);
  let visibleFiles = 0, labels = 0;
  if (view) {
    context.font = "11px 'DejaVu Sans', sans-serif";
    for(const group of scene.groups){
      const r=group.rect,x=camera.x+r.x*camera.scale,y=camera.y+r.y*camera.scale,w=r.width*camera.scale,h=r.height*camera.scale;
      if(x+w<0 || y+h<0 || x>width || y>height)continue;
      context.fillStyle=palette.group;context.fillRect(x,y,w,h);context.strokeStyle=palette.border;context.lineWidth=1;context.strokeRect(x+.5,y+.5,Math.max(0,w-1),Math.max(0,h-1));
      if(group.summary){drawDirectorySummary(group,x,y,w,h);continue;}
      const title=`${group.collapsed?"▸":"▾"} ${group.label} (${group.count.toLocaleString()})`;
      context.font="600 11px 'DejaVu Sans', sans-serif";
      const band={x:x+3,y,width:Math.max(0,w-6),height:Math.min(h,group.header*camera.scale)},placement=headingPlacement(band,context.measureText(title));
      if(placement){context.save();context.beginPath();context.rect(band.x,band.y,band.width,band.height);context.clip();context.fillStyle=palette.text;context.fillText(title,placement.x,placement.baseline);context.restore();}
    }
    for (const file of scene.files) {
      const rect = file.rect, x = camera.x + rect.x * camera.scale, y = camera.y + rect.y * camera.scale, w = rect.width * camera.scale, h = rect.height * camera.scale;
      if (x + w < 0 || y + h < 0 || x > width || y > height) continue;
      visibleFiles++;
      drawFileTile(context, file, x, y, w, h);
      if (w > 32 && h > 18) {
        labels++;
        const rightPadding = file.marker && h < 32 ? 13 : 5;
        context.save(); context.beginPath(); context.rect(x + 5, y, w - 5 - rightPadding, h); context.clip();
        context.fillStyle = palette.text; context.font = "10px 'DejaVu Sans Mono', monospace";
        context.fillText(file.path.split("/").at(-1), x + 7, y + Math.min(h - 5, 17));
        context.restore();
      }
    }
  }
  drawOverview(dpr);
  sample(metrics.drawCpuMs, performance.now() - start);
  metrics.lastFrame = { snapshot: view?.snapshot, camera: { ...camera }, visibleFiles, labels, width, height };
}
function worldBounds() { return {width:scene?.width ?? 1,height:scene?.height ?? 1}; }
function drawOverview(dpr) {
  const world=worldBounds();
  overview.hidden=Boolean(scene?.paged && camera.x>=0 && camera.y>=0 &&
    camera.x+world.width*camera.scale<=canvas.clientWidth && camera.y+world.height*camera.scale<=canvas.clientHeight);
  if(overview.hidden){overviewTransform=null;return;}
  const width=overview.clientWidth,height=overview.clientHeight;
  overview.width=Math.round(width*dpr);overview.height=Math.round(height*dpr);
  overviewContext.setTransform(dpr,0,0,dpr,0,0);overviewContext.clearRect(0,0,width,height);
  if(!view)return;
  const scale=Math.min((width-12)/world.width,(height-12)/world.height);
  const x=(width-world.width*scale)/2,y=(height-world.height*scale)/2;overviewTransform={x,y,scale};
  for(const group of scene.groups){if(!group.summary)continue;const r=group.rect;overviewContext.fillStyle=palette.group;overviewContext.fillRect(x+r.x*scale,y+r.y*scale,Math.max(1,r.width*scale-1),Math.max(1,r.height*scale-1));}
  for (const file of scene.files) {
    const r = file.rect;
    drawFileTile(overviewContext, file, x + r.x * scale, y + r.y * scale, Math.max(1, r.width * scale), Math.max(1, r.height * scale));
  }
  overviewContext.strokeStyle=palette.focus;overviewContext.lineWidth=1.5;
  overviewContext.strokeRect(x-camera.x/camera.scale*scale,y-camera.y/camera.scale*scale,canvas.clientWidth/camera.scale*scale,canvas.clientHeight/camera.scale*scale);
}
function jumpOverview(event) {
  if(!overviewTransform)return;
  const r=overview.getBoundingClientRect(),t=overviewTransform,world=worldBounds();
  const x=Math.max(0,Math.min(world.width,(event.clientX-r.left-t.x)/t.scale)),y=Math.max(0,Math.min(world.height,(event.clientY-r.top-t.y)/t.scale));
  fitMode=null;camera=jumpCamera(camera,{x,y},canvas.clientWidth,canvas.clientHeight);invalidate();
}
overview.addEventListener("pointerdown",event=>{if(event.button!==0)return;event.preventDefault();overview.focus();overviewDrag=true;overview.setPointerCapture(event.pointerId);jumpOverview(event);});
overview.addEventListener("pointermove",event=>{if(overviewDrag)jumpOverview(event);});
overview.addEventListener("pointerup",()=>{overviewDrag=false;});overview.addEventListener("pointercancel",()=>{overviewDrag=false;});
overview.addEventListener("keydown",event=>{const world=worldBounds(),step={ArrowLeft:[-world.width/8,0],ArrowRight:[world.width/8,0],ArrowUp:[0,-world.height/8],ArrowDown:[0,world.height/8]}[event.key];if(!step)return;event.preventDefault();fitMode=null;camera.x-=step[0]*camera.scale;camera.y-=step[1]*camera.scale;invalidate();});
for(const [id,factor] of [["zoom-in",1.5],["zoom-out",1/1.5]])element(id).addEventListener("click",()=>{fitMode=null;camera=zoomCamera(camera,{x:canvas.clientWidth/2,y:canvas.clientHeight/2},factor);invalidate();});
function hit(x, y) {
  if (!view) return null;
  const worldX = (x - camera.x) / camera.scale, worldY = (y - camera.y) / camera.scale;
  const start = performance.now();
  const file = scene.files.find(file => worldX >= file.rect.x && worldX <= file.rect.x + file.rect.width && worldY >= file.rect.y && worldY <= file.rect.y + file.rect.height);
  sample(metrics.pickingMs, performance.now() - start);
  return file;
}
function groupAt(x,y,header=false){if(!scene)return null;const wx=(x-camera.x)/camera.scale,wy=(y-camera.y)/camera.scale;return [...scene.groups].reverse().find(g=>wx>=g.rect.x && wx<=g.rect.x+g.rect.width && wy>=g.rect.y && wy<=g.rect.y+(header && !g.summary?g.header:g.rect.height));}
function point(event) { const bounds = canvas.getBoundingClientRect(); return { x: event.clientX - bounds.left, y: event.clientY - bounds.top }; }
canvas.addEventListener("pointerdown", event => { if (event.button !== 0) return; const p = point(event); drag = { ...p, startX: p.x, startY: p.y, moved: false, group:groupAt(p.x,p.y,true)?.path }; canvas.setPointerCapture(event.pointerId); element("tooltip").hidden = true; });
canvas.addEventListener("pointermove", event => {
  const p = point(event);
  if (drag) { if(!drag.group){fitMode=null;camera.x += p.x - drag.x; camera.y += p.y - drag.y;} drag.moved ||= Math.hypot(p.x - drag.startX, p.y - drag.startY) > 4; drag.x = p.x; drag.y = p.y; invalidate(); return; }
  const file = hit(p.x, p.y), tooltip = element("tooltip"); tooltip.hidden = !file;
  if (!file){const group=groupAt(p.x,p.y);if(group){tooltip.hidden=false;tooltip.textContent=`${group.path}\n${group.count.toLocaleString()} files · ${group.summary?"click to open":group.limited?"click header to focus":group.collapsed?"click header to expand":"click header to collapse"}`;tooltip.style.left=`${Math.min(p.x+12,canvas.clientWidth*.45)}px`;tooltip.style.top=`${Math.min(p.y+12,canvas.clientHeight-65)}px`;}}
  if (file) { tooltip.textContent = `${file.path}\n${file.bytes.toLocaleString()} bytes · ${fileDescription(file)}`; tooltip.style.left = `${Math.min(p.x + 12, canvas.clientWidth * .45)}px`; tooltip.style.top = `${Math.max(0, Math.min(p.y + 12, canvas.clientHeight - 65))}px`; }
});
canvas.addEventListener("pointerup",event=>{
  const p=point(event);
  if(drag?.group){
    const target=groupAt(p.x,p.y,true);
    if(target?.summary && !drag.moved && target.path===drag.group)openDirectory(target.path);
    else if(target && !target.summary){
      const changed=drag.moved?mapModel.move(drag.group,target.path):target.limited?mapModel.focus(drag.group):mapModel.toggle(drag.group);
      if(changed){refreshScene();saveMap();fit(true);}
    }
  }else if(drag && !drag.moved){
    const file=hit(p.x,p.y);if(file)navigate({file:file.id,entity:null});
  }
  drag=null;previousFrame=null;
});
canvas.addEventListener("pointercancel", () => { drag = null; previousFrame = null; });
canvas.addEventListener("pointerleave", () => { element("tooltip").hidden = true; });
canvas.addEventListener("wheel", event => {
  event.preventDefault();fitMode=null;camera=zoomCamera(camera,point(event),Math.exp(-event.deltaY*.0015));
  element("tooltip").hidden = true; invalidate();
}, { passive: false });
canvas.addEventListener("keydown", event => { const direction = { ArrowLeft: [40, 0], ArrowRight: [-40, 0], ArrowUp: [0, 40], ArrowDown: [0, -40] }[event.key]; if (direction) { event.preventDefault();fitMode=null; camera.x += direction[0]; camera.y += direction[1]; invalidate(); } else if (event.key === "Home") { event.preventDefault(); fit(); } });
element("fit").addEventListener("click", () => fit());
pathFilter.addEventListener("input", () => {
  editorFiles.manual();
  clearTimeout(filterTimer);
  updateClearFilters();
  renderSearchScope();
  renderSearchError();
  filterTimer = setTimeout(() => navigate({}), 160);
});
for(const option of [searchWithin,searchRegex,searchCase])option.addEventListener("change",()=>void navigate({}));
element("match-previous").addEventListener("click",()=>changeMatchPage(-1));
element("match-next").addEventListener("click",()=>changeMatchPage(1));
clearFilters.addEventListener("click", () => {
  const hadTypeFilters = selectedTypes.size > 0;
  pathFilter.value = "";
  selectedTypes.clear();
  renderMapLegend();
  const nextFocus = hadTypeFilters ? element("map-legend").querySelector("button") : null;
  (nextFocus ?? pathFilter).focus();
  void navigate({});
});
element("map-page-previous").addEventListener("click",()=>changePage(-1));
element("map-page-next").addEventListener("click",()=>changePage(1));
element("nav-back").addEventListener("click",()=>{editorFiles.manual();clearTimeout(filterTimer);navigation.move(-1);});
element("nav-forward").addEventListener("click",()=>{editorFiles.manual();clearTimeout(filterTimer);navigation.move(1);});
window.addEventListener("popstate",event=>{editorFiles.manual();clearTimeout(filterTimer);void navigation.traverse(event.state);});
element("file-list").closest("details").addEventListener("toggle", renderFileList);
const viewportObserver = new ResizeObserver(()=>{
  const next={width:canvas.clientWidth,height:canvas.clientHeight};
  if(view && mapSize && (next.width!==mapSize.width || next.height!==mapSize.height)){
    if(fitMode)fit(fitMode==="readable");
    else camera=jumpCamera(camera,{x:(mapSize.width/2-camera.x)/camera.scale,y:(mapSize.height/2-camera.y)/camera.scale},next.width,next.height);
  }
  mapSize=next;invalidate();
  void editorFiles.resume();
});
viewportObserver.observe(canvas);
viewportObserver.observe(source);
viewportObserver.observe(header);
function legacyLaunchDestination() {
  if (location.pathname !== '/') return;
  if (location.hash === '#saved') return '/investigate#saved';

  const fragment = new URLSearchParams(location.hash.slice(1));
  const demo = new URLSearchParams(location.search).get('demo');
  if (demo === 'minitosis' || ['source-function', 'source-call', 'editor-call'].some(key => fragment.has(key))) {
    return `/workspace${location.search}${location.hash}`;
  }
}

const destination = legacyLaunchDestination();
if (destination) {
  location.replace(destination);
} else {
  if (location.pathname === '/') window.addEventListener('hashchange', () => {
    const destination = legacyLaunchDestination();
    if (destination) location.replace(destination);
  });
  updates = followChanges({ session: () => controller.updates() });
}
