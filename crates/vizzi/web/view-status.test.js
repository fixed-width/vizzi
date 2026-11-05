import {test} from 'node:test';import assert from 'node:assert/strict';import {workspaceLabel,reviewLabel} from './view-status.js';
test('ready views keep stable text through brief reanalysis while initial work and failures remain explicit',()=>{
 assert.equal(workspaceLabel({snapshot:'s',pending:false,error:null}),'Source ready');assert.equal(workspaceLabel({snapshot:'s',pending:true,error:null}),'Source ready');
 assert.equal(reviewLabel({active:{},pending:false,error:null}),'Comparison ready');assert.equal(reviewLabel({active:{},pending:true,error:null}),'Comparison ready');
 assert.equal(reviewLabel({active:null,pending:true,error:null}),'Capturing comparison…');assert.equal(reviewLabel({active:null,pending:false,error:null}),'Choose a base revision to begin');assert.equal(reviewLabel({active:{},pending:false,error:{message:'bad'}}),'Comparison failed');
 assert.equal(workspaceLabel({snapshot:'s',pending:false,error:'bad'}),'Refresh failed; showing captured source');
});
