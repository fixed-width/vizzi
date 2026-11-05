import assert from 'node:assert/strict';
import {json,validateLibrary,libraryPath,templatesPath} from './support.mjs';
const library=await json(libraryPath),templates=await json(templatesPath);
validateLibrary(library,templates);
for(const field of ['id','template','expected_status']){
 const broken=structuredClone(library);
 if(field==='id')broken.cases[1].id=broken.cases[0].id;
 if(field==='template')broken.cases[0].template='missing';
 if(field==='expected_status')broken.cases[0].expected_status='insufficient_evidence';
 assert.throws(()=>validateLibrary(broken,templates));
}
console.log(`question library: ${templates.templates.length} templates, ${library.cases.length} pinned/temporal cases; malformed identities, templates and answerability rejected`);
