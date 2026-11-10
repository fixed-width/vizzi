const opaque=s=>typeof s==='string'&&/^[0-9a-f]{64}$/.test(s);
const text=(s,n)=>typeof s==='string'&&s.length>0&&new TextEncoder().encode(s).length<=n&&!/[\r\n\0]/.test(s);
export function validOrigin(o){return o==null||Boolean(o&&opaque(o.id)&&opaque(o.capture)&&text(o.label,256)&&text(o.language,64)&&(o.version==null||text(o.version,128)));}
export function captureFor(symbol,project){return symbol.origin?.capture??project;}
export function originLabel(o){return o?`Dependency · ${o.label}${o.version?' '+o.version:''}`:'';}
