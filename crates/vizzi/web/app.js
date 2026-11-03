import { createController } from "/controller.js";
import { followChanges } from "/updates.js";
import { describeSource } from "/source-description.js";
import { createInspector } from "/inspector.js";
import { createNavigation } from "/navigation.js";

const element = id => document.getElementById(id);
const canvas = element("map"), context = canvas.getContext("2d");
const source = element("source"), space = element("source-space");
const metrics = { drawCpuMs: [], interactionFrameMs: [], requestMs: [], pickingMs: [], lastFrame: null };
window.vizziMetrics = metrics;
function sample(series, value) { series.push(value); if (series.length > 512) series.shift(); }
let view = null, camera = { x: 20, y: 20, scale: 1 };
let scheduled = false, drag = null, previousFrame = null, filterTimer = null;
let updates, viewFilter = "";
const fetchJSON = async (uri, options) => {
  const start = performance.now();
  try { const response = await fetch(uri, options); if (!response.ok) throw new Error(await response.text()); return await response.json(); }
  finally { sample(metrics.requestMs, performance.now() - start); }
};
const inspector = createInspector({source,space,outline:element("outline"),transport:fetchJSON,onError:error=>{showError(error);updates?.restart();},select:async fact=>{
  if(await navigate({entity:fact.id}))await inspector.revealActive();
}});

const navigation = createNavigation({
  capture: () => ({ file:view?.selection.file ?? null,entity:view?.selection.entity ?? null,
    path:view?.source?.path ?? null,version:view?.source?.version ?? null,snapshot:view?.snapshot ?? null,
    filter:viewFilter,position:{inspector:inspector.position(),camera:{...camera}} }),
  load: (query,intent) => controller.load(query,{intent,rollbackOnFailure:Boolean(intent.returning)}),
  history: {push:state=>history.pushState(state,""),replace:state=>history.replaceState(state,""),go:delta=>history.go(delta)},
  onState: state => {element("nav-back").disabled=!state.canBack;element("nav-forward").disabled=!state.canForward;element("navigation").setAttribute("aria-busy",String(state.pending));},
  onNotice: notice => {element("navigation-notice").textContent=notice ?? "";element("navigation-notice").hidden=!notice;},
});
function navigate(patch) { clearTimeout(filterTimer);return navigation.navigate({filter:element("filter").value,...patch}); }
const controller = createController({
  viewSchema: 3,
  acceptView: (next,info) => navigation.allow(next,info.intent),
  transport: async (uri, options) => {
    const next=await fetchJSON(`${uri}&projection=summary`,options);
    const restore=navigation.restoration(next,options.intent);
    return inspector.prepare(next,{...options,restore:restore?.position.inspector,reveal:Boolean(options.intent?.returning && !restore)});
  },
  onView: (next,info) => {
    const first = !view;
    view = next;
    viewFilter=info.query.filter ?? "";
    if(info.intent?.returning)element("filter").value=viewFilter;
    element("counts").textContent = `${next.files.length} shown / ${next.matching} matching / ${next.total} source files · ${next.analyzed - (next.deferred ?? 0)} with analysis / ${next.deferred ?? 0} deferred / ${next.source_only} source only`;
    element("empty").hidden = next.files.length > 0;
    element("file-title").textContent = next.source?.path ?? "Choose a file";
    element("snapshot").textContent = next.snapshot.slice(0, 8);
    element("snapshot").title = `Captured observation ${next.snapshot}`;
    const { coverage, diagnostics } = describeSource(next.source);
    element("coverage").textContent = coverage;
    element("diagnostics").hidden = diagnostics.length === 0;
    element("diagnostics").textContent = diagnostics.join(". ");
    inspector.apply(next); renderFileList();
    const restore=navigation.restoration(next,info.intent);
    const sameScene=restore && restore.snapshot===next.snapshot;
    if(sameScene)camera={...restore.position.camera};
    if (first || (info.intent?.returning && !sameScene)) fit(true); else invalidate();
    navigation.observe(info.intent);
  },
  onStatus: status => {
    element("workspace").textContent = status.name;
    element("status").textContent = status.error ? "Refresh failed; showing captured source" : status.pending ? "Analyzing changes…" : `Observation ${status.generation}`;
    element("status").title = status.observation;
    const m = status.measurements;
    if (m) element("measurements").textContent = `${m.total_ms.toFixed(1)} ms refresh · ${m.parsed} parsed · ${m.reused} reused · ${m.cache_hits} cache hits${m.cache_warnings ? ` · ${m.cache_warnings} cache write failures` : ""}`;
    if (status.error) showError(new Error(status.error));
    else element("error").hidden = true;
  },
  onError: showError,
  onResync: () => updates?.restart(),
});
function showError(error) { element("error").textContent = `${error.message || error} · Keeping the last captured view; retrying.`; element("error").hidden = false; }

function renderFileList() {
  const list = element("file-list"); list.replaceChildren();
  if (!list.closest("details").open || !view) return;
  for (const file of view.files) {
    const button = document.createElement("button"); button.textContent = file.path;
    button.setAttribute("aria-current", String(file.id === view.selection.file));
    button.addEventListener("click", () => navigate({ file: file.id, entity: null })); list.append(button);
  }
}
function invalidate() { if (!scheduled) { scheduled = true; requestAnimationFrame(draw); } }
function fit(readable = false) {
  if (!view) return;
  const width = Math.max(1, ...view.files.map(file => file.rect.x + file.rect.width));
  const scale = Math.min((canvas.clientWidth - 36) / width, (canvas.clientHeight - 44) / Math.max(view.height, 1), 1.5);
  camera = { x: 18, y: 22, scale: Math.max(readable ? .55 : .001, scale) };
  invalidate();
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
    for (const group of view.groups) {
      const y = camera.y + group.y * camera.scale;
      if (camera.scale > .28 && y > -20 && y < height) { context.fillStyle = "#6b7c8f"; context.fillText(group.path, camera.x, y + 14); }
    }
    for (const file of view.files) {
      const rect = file.rect, x = camera.x + rect.x * camera.scale, y = camera.y + rect.y * camera.scale, w = rect.width * camera.scale, h = rect.height * camera.scale;
      if (x + w < 0 || y + h < 0 || x > width || y > height) continue;
      visibleFiles++;
      const selected = file.id === view.selection.file;
      context.fillStyle = selected ? "#265dc5" : file.coverage === "partial" ? "#f3deb2" : ["unavailable", "deferred"].includes(file.coverage) ? "#d5dce4" : "#d0deed";
      context.fillRect(x, y, w, h);
      if (w > 80 && h > 20) {
        labels++;
        context.save(); context.beginPath(); context.rect(x + 5, y, w - 10, h); context.clip();
        context.fillStyle = selected ? "white" : "#183044"; context.font = "10px 'DejaVu Sans Mono', monospace";
        context.fillText(file.path.split("/").at(-1), x + 7, y + Math.min(h - 5, 17));
        context.restore();
      }
    }
  }
  sample(metrics.drawCpuMs, performance.now() - start);
  metrics.lastFrame = { snapshot: view?.snapshot, camera: { ...camera }, visibleFiles, labels, width, height };
}
function hit(x, y) {
  if (!view) return null;
  const worldX = (x - camera.x) / camera.scale, worldY = (y - camera.y) / camera.scale;
  const start = performance.now();
  const file = view.files.find(file => worldX >= file.rect.x && worldX <= file.rect.x + file.rect.width && worldY >= file.rect.y && worldY <= file.rect.y + file.rect.height);
  sample(metrics.pickingMs, performance.now() - start);
  return file;
}
function point(event) { const bounds = canvas.getBoundingClientRect(); return { x: event.clientX - bounds.left, y: event.clientY - bounds.top }; }
canvas.addEventListener("pointerdown", event => { if (event.button !== 0) return; const p = point(event); drag = { ...p, startX: p.x, startY: p.y, moved: false }; canvas.setPointerCapture(event.pointerId); element("tooltip").hidden = true; });
canvas.addEventListener("pointermove", event => {
  const p = point(event);
  if (drag) { camera.x += p.x - drag.x; camera.y += p.y - drag.y; drag.moved ||= Math.hypot(p.x - drag.startX, p.y - drag.startY) > 4; drag.x = p.x; drag.y = p.y; invalidate(); return; }
  const file = hit(p.x, p.y), tooltip = element("tooltip"); tooltip.hidden = !file;
  if (file) { tooltip.textContent = `${file.path}\n${file.bytes.toLocaleString()} bytes · ${file.classification.kind === "source_only" ? "source only" : file.coverage === "deferred" ? "analysis deferred" : `${file.coverage} syntax`}`; tooltip.style.left = `${Math.min(p.x + 12, canvas.clientWidth * .45)}px`; tooltip.style.top = `${Math.max(0, Math.min(p.y + 12, canvas.clientHeight - 65))}px`; }
});
canvas.addEventListener("pointerup", event => { if (drag && !drag.moved) { const p = point(event), file = hit(p.x, p.y); if (file) navigate({ file: file.id, entity: null }); } drag = null; previousFrame = null; });
canvas.addEventListener("pointercancel", () => { drag = null; previousFrame = null; });
canvas.addEventListener("pointerleave", () => { element("tooltip").hidden = true; });
canvas.addEventListener("wheel", event => {
  event.preventDefault(); const p = point(event), old = camera.scale;
  camera.scale = Math.max(.001, Math.min(4, old * Math.exp(-event.deltaY * .0015)));
  camera.x = p.x - (p.x - camera.x) * camera.scale / old; camera.y = p.y - (p.y - camera.y) * camera.scale / old;
  element("tooltip").hidden = true; invalidate();
}, { passive: false });
canvas.addEventListener("keydown", event => { const direction = { ArrowLeft: [40, 0], ArrowRight: [-40, 0], ArrowUp: [0, 40], ArrowDown: [0, -40] }[event.key]; if (direction) { event.preventDefault(); camera.x += direction[0]; camera.y += direction[1]; invalidate(); } else if (event.key === "Home") { event.preventDefault(); fit(); } });
element("fit").addEventListener("click", () => fit());
element("filter").addEventListener("input", event => { clearTimeout(filterTimer); filterTimer = setTimeout(() => navigate({ filter: event.target.value }), 160); });
element("nav-back").addEventListener("click",()=>{clearTimeout(filterTimer);navigation.move(-1);});
element("nav-forward").addEventListener("click",()=>{clearTimeout(filterTimer);navigation.move(1);});
window.addEventListener("popstate",event=>{clearTimeout(filterTimer);void navigation.traverse(event.state);});
element("file-list").closest("details").addEventListener("toggle", renderFileList);
const viewportObserver = new ResizeObserver(invalidate);
viewportObserver.observe(canvas);
viewportObserver.observe(source);
updates = followChanges({ session: () => controller.updates() });
