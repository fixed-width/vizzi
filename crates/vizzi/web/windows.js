const integer = value => Number.isSafeInteger(value) && value >= 0;
const opaque = value => typeof value === "string" && value.length > 0 && value.length <= 256;
const bytes = value => typeof value === "string" ? new TextEncoder().encode(value).length : Infinity;

function validate(view, window) {
  const source = view.source, meta = source.summary;
  if (window?.schema !== 1 || window.snapshot !== view.snapshot || window.file !== source.id || window.version !== source.version || window.line_total !== meta.lines || window.outline_total !== meta.declarations) throw new Error("Mismatched captured window");
  if (!integer(window.line_start) || window.line_start < 1 || window.line_start > Math.max(1, meta.lines) || !integer(window.outline_start) || window.outline_start > Math.max(0, meta.declarations - 1) || !Array.isArray(window.lines) || window.lines.length > 96 || !Array.isArray(window.outline) || window.outline.length > 200) throw new Error("Invalid window bounds");
  let size = 0;
  for (const [index, line] of window.lines.entries()) {
    if (!line || line.number !== window.line_start + index || line.number > meta.lines || ![line.column,line.end_column,line.line_bytes].every(integer) || line.end_column > line.line_bytes || bytes(line.text) !== line.end_column - line.column || line.left !== (line.column > 0) || line.right !== (line.end_column < line.line_bytes)) throw new Error("Invalid source fragment");
    size += bytes(line.text);
  }
  if (size > 32768) throw new Error("Source window exceeds byte budget");
  const entry = row => {
    if (!row || !integer(row.index) || row.index >= meta.declarations || !integer(row.analysis) || !opaque(row.id) || bytes(row.name) > 128 || bytes(row.kind) > 32 || typeof row.name_truncated !== "boolean" || typeof row.kind_truncated !== "boolean" || !integer(row.start_line) || row.start_line < 1 || !integer(row.end_line) || row.end_line < row.start_line || row.end_line > meta.lines) throw new Error("Invalid outline entry");
  };
  for (const [index, row] of window.outline.entries()) { entry(row); if (row.index !== window.outline_start + index) throw new Error("Noncontiguous outline window"); }
  if ((meta.declarations > 0) !== (window.cursor !== null)) throw new Error("Invalid outline cursor");
  if (window.cursor !== null) entry(window.cursor);
  return window;
}

export function createWindowReader({ transport, onWindow = () => {}, onError = () => {} }) {
  let binding, token, fetch;
  async function prepare(view, query, options = {}) {
    if (view.source === null) return null;
    const source = view.source;
    if (!opaque(view.snapshot) || !source) throw new Error("Invalid captured document summary");
    const {id, version, summary} = source;
    if (!opaque(id) || !opaque(version) || !summary) throw new Error("Invalid captured document summary");
    if (!integer(summary.lines) || !integer(summary.declarations)) throw new Error("Invalid captured document summary");
    const params = new URLSearchParams(Object.entries({ ...query, snapshot:view.snapshot, file:source.id, version:source.version }).filter(([,value]) => value != null));
    return validate(view, await transport(`/api/window?${params}`,options));
  }
  return {
    prepare,
    activate(view) { fetch?.abort(); fetch = undefined; token = {}; binding = view; },
    async request(query, intent) {
      const view = binding, current = token = {}; fetch?.abort(); const controller = fetch = new AbortController();
      try {
        const window = await prepare(view, query, { signal:controller.signal });
        if (current !== token) return false;
        onWindow(window, query, intent); return true;
      } catch (error) {
        if (current === token) onError(error);
        return false;
      } finally { if (current === token) fetch = undefined; }
    },
  };
}
