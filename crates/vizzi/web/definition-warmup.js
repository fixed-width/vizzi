// Scheduling only: definition results are discarded and foreground navigation is revalidated.
export class DefinitionWarmup {
    constructor(request) { this.request=request;this.snapshot=null;this.prepared=false;this.attempted=new Set();this.pending=null; }
    start(snapshot, sites) {
        if(this.snapshot!==snapshot){this.snapshot=snapshot;this.prepared=false;this.attempted.clear();}
        if(this.pending||this.prepared)return false;
        const call=sites.find(s=>s.id&&s.name&&!s.binding&&!this.attempted.has(s.id));
        if(!call)return false;
        this.attempted.add(call.id);
        const job=Promise.resolve().then(()=>this.request(snapshot,call.id)).then(result=>{
            if(this.snapshot===snapshot&&result.snapshot===snapshot&&['unique','ambiguous','outside','unresolved'].includes(result.state))this.prepared=true;
        }).catch(()=>{}).finally(()=>{if(this.pending===job)this.pending=null;});
        this.pending=job;return true;
    }
    wait(){return this.pending??Promise.resolve();}
}
