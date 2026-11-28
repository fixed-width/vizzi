const count = value => value.toLocaleString('en-US');
const noun = value => value===1?'file':'files';
export function coverageLabel(coverage,sourceOnly=false) {
  if(sourceOnly)return 'Source only';
  return {syntax:'Syntax available',partial:'Syntax with recovery',deferred:'Analysis deferred'}[coverage] ?? 'Analysis unavailable';
}
export function fileCount(shown,matching,total) {
  if(shown<matching)return `Showing ${count(shown)} of ${count(matching)}${matching<total?' matching':''} files`;
  return matching<total?`${count(matching)} of ${count(total)} ${noun(total)}`:`${count(total)} ${noun(total)}`;
}
export function pageCount(offset,shown,total) {
  return shown===total?`${count(total)} ${noun(total)}`:`${count(shown?offset+1:0)}–${count(offset+shown)} of ${count(total)} ${noun(total)}`;
}
export function kindGroup(kind) {
  if(['struct','enum','class','trait','union','type','type_alias','typedef'].includes(kind))return 'type';
  if(['function','method','macro'].includes(kind))return 'function';
  if(['constant','const','static','variable','field'].includes(kind))return 'constant';
  if(['module','namespace'].includes(kind))return 'module';
  return 'other';
}
