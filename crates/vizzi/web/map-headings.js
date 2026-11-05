// Position the whole glyph box inside a header, or suppress the label at small scales.
export function headingPlacement(box,metrics) {
 const ascent=Math.max(0,metrics.actualBoundingBoxAscent ?? 9),descent=Math.max(0,metrics.actualBoundingBoxDescent ?? 3),height=ascent+descent;
 if(box.width<32 || box.height<height+4)return null;
 return {x:box.x+4,baseline:box.y+(box.height-height)/2+ascent};
}
