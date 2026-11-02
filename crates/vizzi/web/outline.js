const ROW = 48, OVERSCAN = 8, MAX_WINDOW_ROWS = 199, MAX_VISIBLE_ROWS = 180;

export function createOutline({ viewport, select }) {
  const space = viewport.ownerDocument.createElement("div");
  viewport.replaceChildren(space);
  viewport.setAttribute("role", "list");
  viewport.setAttribute("aria-label", "Declarations");
  viewport.style.maxHeight = `${MAX_VISIBLE_ROWS * ROW}px`;
  let key, file, facts = [], positions = new Map(), selected = null, cursor = 0, scheduled;
  const nodes = new Map();
  const focusedId = () => viewport.contains(viewport.ownerDocument.activeElement) ? viewport.ownerDocument.activeElement.closest("button[data-declaration]")?.dataset.declaration : undefined;
  function reveal(index) {
    const top = index * ROW;
    viewport.scrollTop = Math.max(0, Math.min(top, Math.max(viewport.scrollTop, top + ROW - viewport.clientHeight)));
  }
  function render() {
    scheduled = false;
    if (!facts.length) {
      nodes.clear(); viewport.tabIndex = 0;
      const note = viewport.ownerDocument.createElement("p"); note.textContent = "No declarations extracted"; space.replaceChildren(note); return;
    }
    viewport.tabIndex = -1;
    const first = Math.max(0, Math.floor(viewport.scrollTop / ROW) - OVERSCAN);
    const last = Math.min(facts.length, Math.ceil((viewport.scrollTop + viewport.clientHeight) / ROW) + OVERSCAN, first + MAX_WINDOW_ROWS);
    const indices = new Set();
    for (let index = first; index < last; index++) indices.add(index);
    indices.add(cursor);
    const keep = new Set([...indices].map(index => facts[index].id));
    for (const [id, node] of nodes) if (!keep.has(id)) { node.row.remove(); nodes.delete(id); }
    for (const index of indices) {
      const fact = facts[index];
      let node = nodes.get(fact.id);
      if (!node) {
        const row = viewport.ownerDocument.createElement("div"); row.className = "outline-entry"; row.setAttribute("role", "listitem");
        const button = viewport.ownerDocument.createElement("button"), name = viewport.ownerDocument.createElement("span"), detail = viewport.ownerDocument.createElement("small");
        button.append(name, detail); row.append(button); space.append(row);
        node = { row, button, name, detail }; nodes.set(fact.id, node);
      }
      node.row.style.top = `${index * ROW}px`;
      node.row.setAttribute("aria-posinset", String(index + 1)); node.row.setAttribute("aria-setsize", String(facts.length));
      node.button.dataset.declaration = fact.id; node.button.tabIndex = index === cursor ? 0 : -1;
      node.button.setAttribute("aria-current", String(fact.id === selected));
      node.button.title = `${fact.kind} ${fact.name} · line ${fact.start_line}`;
      node.name.textContent = fact.name; node.detail.textContent = `${fact.kind} · ${fact.start_line}`;
    }
  }
  function schedule() { if (!scheduled) { scheduled = true; requestAnimationFrame(render); } }
  viewport.addEventListener("scroll", () => {
    if (facts.length && !focusedId()) cursor = Math.min(facts.length - 1, Math.max(0, Math.floor(viewport.scrollTop / ROW)));
    schedule();
  });
  viewport.addEventListener("focusin", event => {
    const index = positions.get(event.target.closest("button[data-declaration]")?.dataset.declaration);
    if (index !== undefined) { cursor = index; render(); }
  });
  viewport.addEventListener("click", event => {
    const index = positions.get(event.target.closest("button[data-declaration]")?.dataset.declaration);
    if (index !== undefined) { cursor = index; select(facts[index]); }
  });
  viewport.addEventListener("keydown", event => {
    if (!facts.length) return;
    const page = Math.max(1, Math.floor(viewport.clientHeight / ROW));
    const destination = { ArrowDown: cursor + 1, ArrowUp: cursor - 1, PageDown: cursor + page, PageUp: cursor - page, Home: 0, End: Infinity }[event.key];
    if (destination === undefined) return;
    event.preventDefault(); cursor = Math.max(0, Math.min(facts.length - 1, destination)); reveal(cursor); render();
    nodes.get(facts[cursor].id).button.focus({ preventScroll: true });
  });
  new ResizeObserver(schedule).observe(viewport);
  return {
    update(source, entity, revealSelection = false) {
      const nextKey = source?.version ? JSON.stringify([source.id, source.version]) : source;
      let restoreFocus = false;
      if (nextKey !== key) {
        const focus = focusedId(), cursorId = facts[cursor]?.id;
        const previous = facts, previousCursor = cursor;
        const top = Math.floor(viewport.scrollTop / ROW), anchor = facts[top]?.id, offset = viewport.scrollTop - top * ROW;
        const changedFile = file !== source?.id;
        facts = source?.outlines.flatMap(outline => outline.declarations) ?? [];
        positions = new Map(facts.map((fact, index) => [fact.id, index]));
        if (!nodes.size) space.replaceChildren();
        space.style.height = `${Math.max(1, facts.length) * ROW}px`;
        restoreFocus = focus !== undefined && !positions.has(focus);
        cursor = changedFile ? positions.get(entity) ?? 0 : positions.get(cursorId) ?? positions.get(entity) ?? nearest(previous, previousCursor, positions);
        if (changedFile) viewport.scrollTop = 0;
        else if (positions.has(anchor)) viewport.scrollTop = positions.get(anchor) * ROW + offset;
        key = nextKey; file = source?.id;
      }
      selected = positions.has(entity) ? entity : null;
      if (selected && revealSelection) reveal(positions.get(selected));
      render();
      if (restoreFocus) {
        if (facts.length) nodes.get(facts[cursor].id).button.focus({ preventScroll: true });
        else viewport.focus({ preventScroll: true });
      }
      return selected ? facts[positions.get(selected)] : null;
    },
  };
}

function nearest(previous, cursor, positions) {
  for (let index = cursor + 1; index < previous.length; index++) if (positions.has(previous[index].id)) return positions.get(previous[index].id);
  for (let index = cursor - 1; index >= 0; index--) if (positions.has(previous[index].id)) return positions.get(previous[index].id);
  return 0;
}
