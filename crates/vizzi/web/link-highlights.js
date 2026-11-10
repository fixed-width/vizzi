// Project retained relationships onto the exact captured code currently displayed.
export function linkHighlights(views,links,targets){
 const byId=new Map(views.map(v=>[v.id,v])),nodes=new Set(),edges=new Set(),lines=[];
 for(const link of links){
  const from=byId.get(link.from),to=byId.get(link.to);if(!from||!to)continue;
  const s=link.source,o=from.symbol.source,matching=s&&['file','version','context','path'].every(k=>s[k]===o[k])&&Number.isSafeInteger(s.line)&&Number.isSafeInteger(s.end_line)&&s.line>=o.line&&s.end_line>=s.line&&s.end_line<=o.end_line;
  const arrow=targets.some(t=>t?.link===link.id),bubble=targets.some(t=>t?.view===to.id),line=matching&&targets.some(t=>t?.view===from.id&&Number.isSafeInteger(t.line)&&t.line>=s.line&&t.line<=s.end_line);
  if(!arrow&&!bubble&&!line)continue;
  nodes.add(to.id);if(arrow)nodes.add(from.id);edges.add(link.id);
  if(matching)lines.push({view:from.id,start:s.line,end:s.end_line});
 }
 return {nodes,links:edges,lines};
}
