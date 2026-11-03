import { test } from "node:test";
import assert from "node:assert/strict";
import { createNavigation } from "./navigation.js";

const location = (file, entity = null, version = "v1") => ({
  file, entity, version, snapshot: "snapshot", path: file, filter: "",
  position: { inspector: { sourceTop: 123, sourceLeft: 7, outlineTop: 48, cursorIndex: 1 }, camera: { x: 1, y: 2, scale: 1 } },
});
function fixture() {
  let current = location("a"), state;
  const pushes = [], replacements = [], moves = [], notices = [], loads = [];
  const navigation = createNavigation({
    capture: () => structuredClone(current),
    load: async (query, intent) => { loads.push({ query, intent }); return false; },
    history: { push: token => pushes.push(token), replace: token => replacements.push(token), go: delta => moves.push(delta) },
    onState: next => { state = next; }, onNotice: notice => notices.push(notice),
  });
  const show = (next, intent) => { current = next; navigation.observe(intent); };
  navigation.observe();
  return { navigation, pushes, replacements, moves, notices, loads, show, state: () => state,
    current: () => current };
}
test("accepted visits preserve the previous position, filter changes do not add visits, and Back/Forward branch", async () => {
  const f = fixture();
  assert.deepEqual(f.state(), { canBack: false, canForward: false, count: 1, pending: false });
  await f.navigation.navigate({ file: "b", entity: null });
  const request = f.loads.at(-1);
  f.show(location("b"), request.intent);
  assert.equal(f.pushes.length, 1);
  assert.equal(f.state().canBack, true);
  await f.navigation.traverse(f.replacements[0]);
  const returning = f.loads.at(-1);
  assert.equal(returning.query.file, "a");
  assert.equal(f.navigation.allow({ selection: { file: "a", entity: null } }, returning.intent), true);
  assert.equal(f.navigation.restoration({ source: { id: "a", version: "v1" }, selection: { entity: null } }, returning.intent).position.inspector.sourceTop, 123);
  f.show(location("a"), returning.intent);
  assert.equal(f.state().canForward, true);
  await f.navigation.navigate({ filter: "compiler" });
  f.show({ ...location("a"), filter: "compiler" }, f.loads.at(-1).intent);
  assert.equal(f.pushes.length, 1);
  await f.navigation.navigate({ file: "c", entity: null });
  f.show(location("c"), f.loads.at(-1).intent);
  assert.equal(f.state().canForward, false);
  assert.equal(f.state().count, 2);
});

test("history bounds expire old markers, ignores foreign states and never moves outside known ends", async () => {
  const f=fixture(),old=f.replacements[0];
  f.navigation.move(-1);f.navigation.move(1);f.navigation.move(0);f.navigation.move(2);
  assert.deepEqual(f.moves,[]);
  for(let i=0;i<55;i++){await f.navigation.navigate({file:"file"+i});f.show(location("file"+i),f.loads.at(-1).intent);}
  assert.equal(f.state().count,50);
  f.navigation.move(-1);assert.deepEqual(f.moves,[-1]);
  const loads=f.loads.length;
  assert.equal(await f.navigation.traverse({foreign:true}),false);
  assert.equal(f.loads.length,loads);
  assert.equal(await f.navigation.traverse(old),false);
  assert.equal(f.loads.length,loads);
  assert.deepEqual(f.state(),{canBack:false,canForward:false,count:1,pending:false});
  assert.match(f.notices.at(-1),/expired/);
});

test("only matching file, version and declaration restore a copied position; changed and removed declarations are explicit", async () => {
  const f=fixture();
  await f.navigation.navigate({file:"b"});f.show(location("b"),f.loads.at(-1).intent);
  await f.navigation.traverse(f.replacements[0]);
  const intent=f.loads.at(-1).intent;
  const v={source:{id:"a",version:"v1"},selection:{file:"a",entity:null}};
  const restore=f.navigation.restoration(v,intent);restore.position.inspector.sourceTop=999;
  assert.equal(f.navigation.restoration(v,intent).position.inspector.sourceTop,123);
  for(const altered of [{...v,source:{id:"other",version:"v1"}},{...v,source:{id:"a",version:"v2"}},{...v,selection:{file:"a",entity:"other"}}]) {
    assert.equal(f.navigation.restoration(altered,intent),undefined);
  }
  assert.equal(f.navigation.restoration(v,undefined),undefined);
  assert.equal(f.navigation.allow({selection:{file:"other"}},intent),false);
  assert.match(f.notices.at(-1),/Cannot restore a/);
  f.show(location("a",null,"v2"),intent);
  assert.match(f.notices.at(-1),/location changed/);
  await f.navigation.navigate({entity:"anchor"});f.show(location("a","anchor","v2"),f.loads.at(-1).intent);
  const anchor=f.pushes.at(-1);
  await f.navigation.navigate({file:"c",entity:null});f.show(location("c"),f.loads.at(-1).intent);
  await f.navigation.traverse(anchor);
  f.show(location("a",null,"v3"),f.loads.at(-1).intent);
  assert.match(f.notices.at(-1),/declaration is no longer available/);
});

test("live updates and duplicate selections replace metadata without visits; superseded returns cannot commit", async () => {
  const f=fixture();
  f.show(location("a",null,"v2"));
  await f.navigation.navigate({file:"a"});f.show(location("a",null,"v2"),f.loads.at(-1).intent);
  assert.equal(f.pushes.length,0);
  await f.navigation.navigate({file:"b"});f.show(location("b"),f.loads.at(-1).intent);
  await f.navigation.traverse(f.replacements[0]);const stale=f.loads.at(-1).intent;
  await f.navigation.navigate({file:"c"});const current=f.loads.at(-1).intent;
  assert.equal(f.navigation.allow({selection:{file:"a"}},stale),false);
  assert.equal(f.navigation.restoration({source:{id:"a",version:"v2"},selection:{entity:null}},stale),undefined);
  f.show(location("c"),current);
  const count=f.state().count;f.show(location("a"),stale);
  assert.equal(f.state().count,count);
  assert.equal(f.state().canForward,false);
  assert.equal(f.navigation.allow({selection:{file:"anything"}},undefined),true);
});

test("failed targets remain retryable and never overwrite the previously displayed visit", async () => {
  const f=fixture(),a=f.replacements[0];
  await f.navigation.navigate({file:"b"});f.show(location("b"),f.loads.at(-1).intent);
  const b=f.pushes.at(-1);
  await f.navigation.traverse(a);const missing=f.loads.at(-1).intent;
  assert.equal(f.navigation.allow({selection:{file:"b"}},missing),false);
  assert.equal(f.state().canForward,true);
  f.navigation.observe();
  await f.navigation.traverse(b);f.show(location("b"),f.loads.at(-1).intent);
  await f.navigation.traverse(a);
  assert.equal(f.loads.at(-1).query.file,"a");
});

test("markers from an earlier session or schema expire safely", async () => {
  for(const mutate of [s=>s.vizziNavigation.session="previous",s=>s.vizziNavigation.session=[s.vizziNavigation.session],s=>s.vizziNavigation.schema=2,s=>s.vizziNavigation.schema="1",s=>s.vizziNavigation.key=[s.vizziNavigation.key]]) {
    const f=fixture(),marker=structuredClone(f.replacements[0]);mutate(marker);
    assert.equal(await f.navigation.traverse(marker),false);
    assert.equal(f.state().count,1);assert.equal(f.loads.length,0);
    assert.match(f.notices.at(-1),/expired/);
  }
});

test("departure and live-update metadata preserve the actual scroll/filter and cannot be mutated through restoration", async () => {
  const f=fixture(),a=f.replacements[0];
  f.current().position.inspector.sourceTop=789;f.current().filter="src";
  await f.navigation.navigate({file:"b",entity:null});
  assert.deepEqual(f.loads.at(-1).query,{file:"b",entity:null,filter:"src"});
  f.show(location("b"),f.loads.at(-1).intent);
  await f.navigation.traverse(a);
  let call=f.loads.at(-1);
  assert.equal(call.query.filter,"src");
  const v={source:{id:"a",version:"v1"},selection:{file:"a",entity:null}};
  assert.equal(f.navigation.restoration(v,call.intent).position.inspector.sourceTop,789);
  f.show({...location("a"),filter:"src"},call.intent);
  assert.equal(f.notices.at(-1),null);
  f.current().version="v2";f.current().position.inspector.sourceTop=456;f.navigation.observe();
  await f.navigation.navigate({filter:"changed"});f.show({...f.current(),filter:"changed"},f.loads.at(-1).intent);
  await f.navigation.navigate({file:"c"});f.show(location("c"),f.loads.at(-1).intent);
  await f.navigation.traverse(a);call=f.loads.at(-1);
  assert.equal(call.query.filter,"changed");
  assert.equal(f.navigation.restoration({...v,source:{id:"a",version:"v2"}},call.intent).position.inspector.sourceTop,456);
});

test("pending states, idempotent completion and late results keep the newest operation in charge", async () => {
  let current=location("a"),state;const calls=[],pushes=[],notices=[];
  const n=createNavigation({capture:()=>current,history:{push:s=>pushes.push(s),replace:()=>{},go:()=>{}},
    load:(query,intent)=>new Promise(resolve=>calls.push({query,intent,resolve})),
    onState:s=>{state=s;},onNotice:s=>notices.push(s)});
  n.observe();
  const old=n.navigate({file:"b"});assert.equal(state.pending,true);
  current=location("unrelated");n.observe();
  const latest=n.navigate({file:"c"});assert.equal(state.pending,true);
  calls[0].resolve(false);assert.equal(await old,false);assert.equal(state.pending,true);
  current=location("c");n.observe(calls[1].intent);n.observe(calls[1].intent);
  calls[1].resolve(true);assert.equal(await latest,true);
  assert.equal(state.pending,false);assert.equal(pushes.length,1);
  assert.equal(notices.at(-1),null);
  n.observe(calls[0].intent);assert.equal(pushes.length,1);
});

test("bootstrap requests seed a single visit and empty/normal intents never restore a viewport", async () => {
  let current=location(null);const calls=[];let state;
  const n=createNavigation({capture:()=>current,load:async(q,intent)=>{calls.push(intent);return false;},
    history:{push:()=>assert.fail("bootstrap must replace"),replace:()=>{},go:()=>{}},onState:s=>{state=s;}});
  assert.equal(n.restoration({source:null,selection:{entity:null}},undefined),undefined);
  assert.equal(await n.traverse(null),false);
  await n.navigate({filter:"empty"});assert.equal(state.canBack,false);
  assert.equal(n.allow({selection:{file:null}},calls[0]),true);
  assert.equal(n.restoration({source:null,selection:{entity:null}},calls[0]),undefined);
  n.observe(calls[0]);assert.equal(state.count,1);assert.equal(state.pending,false);
});

test("a stale completion cannot rewrite an earlier visit, and unavailable/null source is handled without throwing", async () => {
  const f=fixture(),a=f.replacements[0];
  await f.navigation.navigate({file:"b"});f.show(location("b"),f.loads.at(-1).intent);
  await f.navigation.traverse(a);const stale=f.loads.at(-1).intent;
  await f.navigation.navigate({file:"c"});f.show(location("c"),f.loads.at(-1).intent);
  f.navigation.observe(stale);
  await f.navigation.traverse(a);assert.equal(f.loads.at(-1).query.file,"a");
  const intent=f.loads.at(-1).intent;
  assert.equal(f.navigation.restoration({source:null,selection:{file:null,entity:null}},intent),undefined);
  assert.equal(f.navigation.allow({selection:{file:"a"}},intent),true);
  f.show(location("a"),intent);f.navigation.move(1);assert.deepEqual(f.moves,[1]);
  f.navigation.move(0);f.navigation.move(-2);assert.deepEqual(f.moves,[1]);
});

test("failed return notices remain explicit, clear on new requests, and do not replace an unavailable-file notice", async () => {
  let current=location(null),state;const calls=[],notices=[],markers=[];
  let n;let missing=false;
  n=createNavigation({capture:()=>current,onState:s=>{state=s;},onNotice:s=>notices.push(s),
    history:{push:s=>markers.push(s),replace:s=>markers.push(s),go:()=>{}},
    load:async(query,intent)=>{calls.push(intent);return missing?n.allow({selection:{file:"other"}},intent):false;}});
  n.observe();const empty=markers[0];
  await n.navigate({file:"b"});current=location("b");n.observe(calls.at(-1));
  assert.equal(n.restoration({source:null,selection:{entity:null}},undefined),undefined);
  assert.equal(await n.traverse(empty),false);
  assert.match(notices.at(-1),/Could not restore/);
  assert.equal(state.pending,false);
  missing=true;await n.traverse(empty);assert.match(notices.at(-1),/Cannot restore this location/);
  assert.equal(state.pending,false);
  missing=false;await n.navigate({file:"c"});assert.equal(notices.at(-1),null);
});

test("a pending bootstrap never records an unaccepted view and the first Back reaches index zero",async()=>{
  let current=location("a"),state;const calls=[],replacements=[],moves=[];
  const n=createNavigation({capture:()=>current,onState:s=>{state=s;},history:{push:()=>{},replace:s=>replacements.push(s),go:d=>moves.push(d)},
    load:(query,intent)=>new Promise(resolve=>calls.push({intent,resolve}))});
  const job=n.navigate({file:"b"});n.observe();
  assert.equal(state.count,0);assert.equal(state.pending,true);
  current=location("b");n.observe(calls[0].intent);calls[0].resolve(true);await job;
  assert.equal(state.count,1);assert.equal(replacements.length,1);
  const next=n.navigate({file:"c"});current=location("c");n.observe(calls[1].intent);calls[1].resolve(true);await next;
  n.move(-1);assert.deepEqual(moves,[-1]);
});

test("returning to an existing declaration clears notices and changed source keeps the declaration distinction",async()=>{
  const f=fixture();
  await f.navigation.navigate({entity:"anchor"});f.show(location("a","anchor"),f.loads.at(-1).intent);
  const anchor=f.pushes.at(-1);
  await f.navigation.navigate({file:"b",entity:null});f.show(location("b"),f.loads.at(-1).intent);
  await f.navigation.traverse(anchor);f.show(location("a","anchor"),f.loads.at(-1).intent);
  assert.equal(f.notices.at(-1),null);
  await f.navigation.navigate({file:"b",entity:null});f.show(location("b"),f.loads.at(-1).intent);
  await f.navigation.traverse(anchor);f.show(location("a","anchor","v2"),f.loads.at(-1).intent);
  assert.match(f.notices.at(-1),/location changed/);
});
test("filtering after a failed return establishes a new branch instead of rewriting the failed location",async()=>{
  const f=fixture(),a=f.replacements[0];
  await f.navigation.navigate({file:"b"});f.show(location("b"),f.loads.at(-1).intent);
  await f.navigation.traverse(a);
  await f.navigation.navigate({filter:"src"});f.show({...location("b"),filter:"src"},f.loads.at(-1).intent);
  assert.equal(f.pushes.length,2);assert.equal(f.state().canBack,true);assert.equal(f.state().canForward,false);
  await f.navigation.traverse(a);assert.equal(f.loads.at(-1).query.file,"a");
});
test("Forward restores reading positions saved when browser Back departed",async()=>{
  const f=fixture(),a=f.replacements[0];
  await f.navigation.navigate({file:"b"});f.show(location("b"),f.loads.at(-1).intent);
  const b=f.pushes.at(-1);f.current().position.inspector.sourceTop=888;
  await f.navigation.traverse(a);f.show(location("a"),f.loads.at(-1).intent);
  await f.navigation.traverse(b);
  assert.equal(f.navigation.restoration({source:{id:"b",version:"v1"},selection:{entity:null}},f.loads.at(-1).intent).position.inspector.sourceTop,888);
});
test("a late failed traversal cannot post a notice over a newer request",async()=>{
  let current=location("a"),state;const calls=[],markers=[],notices=[];
  const n=createNavigation({capture:()=>current,onState:s=>{state=s;},onNotice:s=>notices.push(s),
    history:{push:s=>markers.push(s),replace:s=>markers.push(s),go:()=>{}},
    load:(query,intent)=>new Promise(resolve=>calls.push({intent,resolve}))});
  n.observe();const a=markers[0];
  let job=n.navigate({file:"b"});current=location("b");n.observe(calls[0].intent);calls[0].resolve(true);await job;
  const old=n.traverse(a);assert.equal(state.pending,true);
  const latest=n.navigate({file:"c"});calls[1].resolve(false);await old;
  assert.equal(notices.at(-1),null);assert.equal(state.pending,true);
  current=location("c");n.observe(calls[2].intent);calls[2].resolve(true);await latest;
});
