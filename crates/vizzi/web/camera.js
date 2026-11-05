const clamp=(value,min,max)=>Math.max(min,Math.min(max,value));
export function fitCamera(worldWidth,worldHeight,width,height,readable=false) {
  const scale=clamp(Math.min(Math.max(1,width-36)/Math.max(1,worldWidth),Math.max(1,height-44)/Math.max(1,worldHeight)),readable ? .55 : .001,1.5);
  return {x:Math.max(18,(width-worldWidth*scale)/2),y:Math.max(22,(height-worldHeight*scale)/2),scale};
}
export function zoomCamera(camera,point,factor) {
  const scale=clamp(camera.scale*factor,.001,4);
  return {x:point.x-(point.x-camera.x)*scale/camera.scale,y:point.y-(point.y-camera.y)*scale/camera.scale,scale};
}
export function jumpCamera(camera,world,width,height) {
  return {x:width/2-world.x*camera.scale,y:height/2-world.y*camera.scale,scale:camera.scale};
}
export function splitFraction(value,extent,before,after) {
  const minimum=Math.min(.45,before/Math.max(1,extent)),maximum=1-Math.min(.45,after/Math.max(1,extent));
  return clamp(Number.isFinite(value)?value:.5,minimum,maximum);
}
