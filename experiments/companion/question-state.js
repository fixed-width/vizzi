// Offline helpers for archived question-search evaluations.
const bytes=text=>new TextEncoder().encode(text).length;
const opaque=value=>typeof value==='string'&&/^[a-f0-9]{64}$/.test(value);
const integer=value=>Number.isSafeInteger(value)&&value>=0;
const groups=[['definition','Definition candidates'],['registration','Registration candidates'],['entry','Entry and use candidates'],['validation','Check candidates'],['example','Example candidates'],[null,'Matching source']];
export function groupResults(results){return groups.map(([role,label])=>({role,label,items:results.filter(r=>r.suggested_role===role)})).filter(g=>g.items.length);}
function validate(data,expected){
 if(!data||data.snapshot!==expected.snapshot||data.question!==expected.question||data.selector!==expected.selector||!integer(data.available_files)||!integer(data.searched_files)||!integer(data.omitted_files)||data.searched_files+data.omitted_files!==data.available_files||!Array.isArray(data.results)||data.results.length>8)throw new Error('Invalid source-search response');
 let used=0;const ids=new Set();
 for(const r of data.results){
  if(!r||!opaque(r.id)||ids.has(r.id)||!opaque(r.file)||!opaque(r.version)||typeof r.title!=='string'||bytes(r.title)>128||typeof r.path!=='string'||bytes(r.path)>4096||r.path.includes('\\')||r.path.split('/').some(p=>!p||p==='.'||p==='..')||!integer(r.line)||r.line<1||!integer(r.end_line)||r.end_line<r.line||typeof r.excerpt!=='string'||!groups.some(([role])=>role===r.suggested_role))throw new Error('Invalid source-search location');
  ids.add(r.id);used+=bytes(r.excerpt);
 }
 if(used>4000)throw new Error('Source-search excerpts exceed the response allowance');
 return data;
}
export class SearchSession {
 constructor(){this.ticket=0;this.snapshot=null;this.data=null;this.pending=false;this.stale=false;this.expected=null;}
 setSnapshot(snapshot){if(snapshot===this.snapshot)return;this.snapshot=snapshot;this.ticket++;this.pending=false;this.stale=Boolean(this.data&&this.data.snapshot!==snapshot);}
 begin(question,selector){
  if(typeof question!=='string'||!question.trim()||bytes(question)>2048)throw new Error('Shorten the question and try again.');
  if(!['focused','original'].includes(selector))throw new Error('Invalid search comparison');
  if(!opaque(this.snapshot))throw new Error('Wait for source indexing before searching.');
  this.expected={snapshot:this.snapshot,question,selector};this.pending=true;return{ticket:++this.ticket,body:this.expected};
 }
 complete(ticket,data){if(ticket!==this.ticket)return false;this.data=validate(data,this.expected);this.pending=false;this.stale=false;return true;}
 fail(ticket){if(ticket===this.ticket)this.pending=false;}
 get canKeep(){return Boolean(this.data&&!this.pending&&!this.stale&&this.data.question===this.expected.question&&this.data.selector===this.expected.selector);}
}
