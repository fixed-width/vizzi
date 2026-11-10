import {BubbleSet,PointPath,ShapeSimplifier,BSplineShapeGenerator} from './vendor/bubblesets.js';

const overlaps=(a,b)=>a.x<b.x+b.width&&b.x<a.x+a.width&&a.y<b.y+b.height&&b.y<a.y+a.height;
function contains(points,x,y){let inside=false;for(let i=0,j=points.length-1;i<points.length;j=i++){const a=points[i],b=points[j];if((a[1]>y)!==(b[1]>y)&&x<(b[0]-a[0])*(y-a[1])/(b[1]-a[1])+a[0])inside=!inside;}return inside;}
export function moduleContours(views){
 if(!Array.isArray(views)||views.length>24||views.some(v=>![v.x,v.y,v.w,v.h].every(Number.isFinite)||v.w<=0||v.h<=0))return [];
 const known=views.filter(v=>v.module);if(known.length<2)return [];
 const x=Math.min(...views.map(v=>v.x)),y=Math.min(...views.map(v=>v.y)),span=Math.max(1,...views.map(v=>v.x+v.w-x),...views.map(v=>v.y+v.h-y)),scale=Math.min(1,800/span);
 const all=views.map(v=>({module:v.module,x:(v.x-x)*scale+30,y:(v.y-y)*scale+30,width:v.w*scale,height:v.h*scale})),result=[];let bytes=0;
 for(const id of new Set(known.map(v=>v.module))){
  const members=all.filter(v=>v.module===id),others=all.filter(v=>v.module!==id);if(members.length<2||members.some(v=>v.width<6||v.height<6)||members.some(a=>others.some(b=>overlaps(a,b))))continue;
  const set=new BubbleSet();set.pixelGroup(4);set.maxRoutingIterations(40);set.maxMarchingIterations(12);
  const raw=set.createOutline(BubbleSet.addPadding(members,8),BubbleSet.addPadding(others,5),undefined);if(raw.length<3||raw.length>4096)continue;
  const points=raw.map(p=>[p[0],p[1]]);
  if(points.some(p=>!p.every(Number.isFinite))||members.some(r=>!contains(points,r.x+r.width/2,r.y+r.height/2))||others.some(r=>contains(points,r.x+r.width/2,r.y+r.height/2)))continue;
  const path=new PointPath(raw).transform([new ShapeSimplifier(0),new BSplineShapeGenerator(),new ShapeSimplifier(0)]).toString();
  if(path.length>32768||bytes+path.length>131072||/NaN|Infinity/.test(path))continue;bytes+=path.length;result.push({id,path,transform:`translate(${x-30/scale} ${y-30/scale}) scale(${1/scale})`});
 }
 return result;
}
