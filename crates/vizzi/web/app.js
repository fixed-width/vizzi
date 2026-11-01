import { createController } from "/controller.js";

const element = id => document.getElementById(id);
const canvas = element("map"), context = canvas.getContext("2d");
const source = element("source"), space = element("source-space");
const metrics = { drawCpuMs: [], interactionFrameMs: [], requestMs: [] };
window.vizziMetrics = metrics;
function sample(series, value) { series.push(value); if (series.length > 512) series.shift(); }
let view = null, lines = [], active = null, camera = { x: 20, y: 20, scale: 1 };
let scheduled = false, drag = null, previousFrame = null, filterTimer = null;

const controller = createController({
  transport: async uri => {
    const start = performance.now();
    try { const response = await fetch(uri); if (!response.ok) throw new Error(await response.text()); return await response.json(); }
    finally { sample(metrics.requestMs, performance.now() - start); }
  },
  onView: next => {
    const first = !view, changedFile = view?.selection.file !== next.selection.file;
    const changedSource = changedFile || view?.source?.text !== next.source?.text;
    view = next;
    element("error").hidden = true;
    element("counts").textContent = `${next.files.length} shown / ${next.matching} matching / ${next.total} Rust files`;
    element("empty").hidden = next.files.length > 0;
    element("file-title").textContent = next.source?.path ?? "Choose a file";
    element("snapshot").textContent = next.snapshot.slice(0, 8);
    element("snapshot").title = `Captured observation ${next.snapshot}`;
    const outlines = next.source?.outlines ?? [];
    const diagnostics = outlines.flatMap(outline => outline.diagnostics);
    element("coverage").textContent = outlines.length ? `Read-only · ${outlines.map(outline => outline.coverage === "syntax" ? "syntax parsed" : outline.coverage === "partial" ? "partial syntax" : "analysis unavailable").join(" · ")}` : "Read-only · no syntax analysis";
    element("diagnostics").hidden = diagnostics.length === 0;
    element("diagnostics").textContent = diagnostics.join(". ");
    const facts = outlines.flatMap(outline => outline.declarations);
    active = facts.find(fact => fact.id === next.selection.entity) ?? null;
    element("outline").replaceChildren();
    for (const fact of facts) {
      const button = document.createElement("button");
      button.textContent = fact.name; button.title = `${fact.kind} ${fact.name} · line ${fact.start_line}`;
      button.setAttribute("aria-current", String(fact.id === next.selection.entity));
      const kind = document.createElement("small"); kind.textContent = `${fact.kind} · ${fact.start_line}`; button.append(kind);
      button.addEventListener("click", () => controller.load({ entity: fact.id }));
      element("outline").append(button);
    }
    if (!facts.length) { const note = document.createElement("p"); note.textContent = "No declarations extracted"; element("outline").append(note); }
    if (changedSource) { lines = next.source?.text?.split(/\r?\n/) ?? []; if (changedFile) source.scrollTop = 0; }
    if (active) source.scrollTop = Math.max(0, (active.start_line - 4) * 22);
    renderSource(); renderFileList();
    if (first) fit(); else invalidate();
  },
  onStatus: status => {
    element("workspace").textContent = status.name;
    element("status").textContent = status.error ? "Refresh failed; showing captured source" : status.pending ? "Analyzing changes…" : `Observation ${status.generation}`;
    element("status").title = status.observation;
    const m = status.measurements;
    if (m) element("measurements").textContent = `${m.total_ms.toFixed(1)} ms refresh · ${m.parsed} parsed · ${m.reused} reused · ${m.cache_hits} cache hits${m.cache_warnings ? ` · ${m.cache_warnings} cache write failures` : ""}`;
    if (status.error) showError(new Error(status.error));
  },
  onError: showError,
});
function showError(error) { element("error").textContent = `${error.message || error} · Keeping the last captured view; retrying.`; element("error").hidden = false; }

function renderSource() {
  space.replaceChildren();
  if (!lines.length) {
    space.style.height = "auto";
    const note = document.createElement("p"); note.className = "source-empty";
    note.textContent = view?.source ? "Source encoding is unavailable in this inspector." : "Select a file in the repository map.";
    space.append(note); return;
  }
  space.style.height = `${lines.length * 22}px`;
  const first = Math.max(0, Math.floor(source.scrollTop / 22) - 25);
  const last = Math.min(lines.length, Math.ceil((source.scrollTop + source.clientHeight) / 22) + 25);
  for (let i = first; i < last; i++) {
    const row = document.createElement("div"); row.className = "code-line";
    if (active && i + 1 >= active.start_line && i + 1 <= active.end_line) row.classList.add("active");
    row.style.top = `${i * 22}px`;
    const number = document.createElement("span"); number.className = "line-no"; number.textContent = i + 1; number.setAttribute("aria-hidden", "true");
    const code = document.createElement("span"); code.textContent = lines[i]; row.append(number, code); space.append(row);
  }
}
function renderFileList() {
  const list = element("file-list"); list.replaceChildren();
  if (!list.closest("details").open || !view) return;
  for (const file of view.files) {
    const button = document.createElement("button"); button.textContent = file.path;
    button.setAttribute("aria-current", String(file.id === view.selection.file));
    button.addEventListener("click", () => controller.load({ file: file.id, entity: null })); list.append(button);
  }
}
function invalidate() { if (!scheduled) { scheduled = true; requestAnimationFrame(draw); } }
function fit() {
  if (!view) return;
  camera = { x: 18, y: 22, scale: Math.max(0.04, Math.min((canvas.clientWidth - 36) / 1424, (canvas.clientHeight - 44) / Math.max(view.height, 1), 1.5)) };
  invalidate();
}
function draw(timestamp) {
  scheduled = false; const start = performance.now();
  if (drag && previousFrame !== null) sample(metrics.interactionFrameMs, timestamp - previousFrame);
  previousFrame = drag ? timestamp : null;
  const width = canvas.clientWidth, height = canvas.clientHeight, dpr = Math.min(window.devicePixelRatio || 1, 2);
  if (canvas.width !== Math.round(width * dpr) || canvas.height !== Math.round(height * dpr)) { canvas.width = Math.round(width * dpr); canvas.height = Math.round(height * dpr); }
  context.setTransform(dpr, 0, 0, dpr, 0, 0); context.clearRect(0, 0, width, height);
  if (view) {
    context.font = "11px 'DejaVu Sans', sans-serif";
    for (const group of view.groups) {
      const y = camera.y + group.y * camera.scale;
      if (camera.scale > .28 && y > -20 && y < height) { context.fillStyle = "#6b7c8f"; context.fillText(group.path, camera.x, y + 14); }
    }
    for (const file of view.files) {
      const rect = file.rect, x = camera.x + rect.x * camera.scale, y = camera.y + rect.y * camera.scale, w = rect.width * camera.scale, h = rect.height * camera.scale;
      if (x + w < 0 || y + h < 0 || x > width || y > height) continue;
      const selected = file.id === view.selection.file;
      context.fillStyle = selected ? "#265dc5" : file.coverage === "partial" ? "#f3deb2" : file.coverage === "unavailable" ? "#d5dce4" : "#d0deed";
      context.fillRect(x, y, w, h);
      if (w > 80 && h > 20) {
        context.save(); context.beginPath(); context.rect(x + 5, y, w - 10, h); context.clip();
        context.fillStyle = selected ? "white" : "#183044"; context.font = "10px 'DejaVu Sans Mono', monospace";
        context.fillText(file.path.split("/").at(-1), x + 7, y + Math.min(h - 5, 17));
        context.restore();
      }
    }
  }
  sample(metrics.drawCpuMs, performance.now() - start);
}
function hit(x, y) {
  if (!view) return null;
  const worldX = (x - camera.x) / camera.scale, worldY = (y - camera.y) / camera.scale;
  return view.files.find(file => worldX >= file.rect.x && worldX <= file.rect.x + file.rect.width && worldY >= file.rect.y && worldY <= file.rect.y + file.rect.height);
}
function point(event) { const bounds = canvas.getBoundingClientRect(); return { x: event.clientX - bounds.left, y: event.clientY - bounds.top }; }
canvas.addEventListener("pointerdown", event => { if (event.button !== 0) return; const p = point(event); drag = { ...p, startX: p.x, startY: p.y, moved: false }; canvas.setPointerCapture(event.pointerId); element("tooltip").hidden = true; });
canvas.addEventListener("pointermove", event => {
  const p = point(event);
  if (drag) { camera.x += p.x - drag.x; camera.y += p.y - drag.y; drag.moved ||= Math.hypot(p.x - drag.startX, p.y - drag.startY) > 4; drag.x = p.x; drag.y = p.y; invalidate(); return; }
  const file = hit(p.x, p.y), tooltip = element("tooltip"); tooltip.hidden = !file;
  if (file) { tooltip.textContent = `${file.path}\n${file.bytes.toLocaleString()} bytes · ${file.coverage} syntax`; tooltip.style.left = `${Math.min(p.x + 12, canvas.clientWidth * .45)}px`; tooltip.style.top = `${Math.max(0, Math.min(p.y + 12, canvas.clientHeight - 65))}px`; }
});
canvas.addEventListener("pointerup", event => { if (drag && !drag.moved) { const p = point(event), file = hit(p.x, p.y); if (file) controller.load({ file: file.id, entity: null }); } drag = null; previousFrame = null; });
canvas.addEventListener("pointercancel", () => { drag = null; previousFrame = null; });
canvas.addEventListener("pointerleave", () => { element("tooltip").hidden = true; });
canvas.addEventListener("wheel", event => {
  event.preventDefault(); const p = point(event), old = camera.scale;
  camera.scale = Math.max(.025, Math.min(4, old * Math.exp(-event.deltaY * .0015)));
  camera.x = p.x - (p.x - camera.x) * camera.scale / old; camera.y = p.y - (p.y - camera.y) * camera.scale / old;
  element("tooltip").hidden = true; invalidate();
}, { passive: false });
canvas.addEventListener("keydown", event => { const direction = { ArrowLeft: [40, 0], ArrowRight: [-40, 0], ArrowUp: [0, 40], ArrowDown: [0, -40] }[event.key]; if (direction) { event.preventDefault(); camera.x += direction[0]; camera.y += direction[1]; invalidate(); } else if (event.key === "Home") { event.preventDefault(); fit(); } });
element("fit").addEventListener("click", fit);
element("filter").addEventListener("input", event => { clearTimeout(filterTimer); filterTimer = setTimeout(() => controller.load({ filter: event.target.value }), 160); });
element("file-list").closest("details").addEventListener("toggle", renderFileList);
source.addEventListener("scroll", renderSource, { passive: true });
new ResizeObserver(() => { invalidate(); renderSource(); }).observe(document.querySelector("main"));
async function tick() { await controller.poll(); setTimeout(tick, document.hidden ? 2000 : 500); }
tick();
