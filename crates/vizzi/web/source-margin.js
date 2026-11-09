// Columns match the source pane's four-column tab stops; original bytes stay in the DOM.
export function functionIndent(line) {
    if(!line||line.left||typeof line.text!=='string')return 0;
    const prefix=line.text.match(/^[ \t]*/)[0];
    if(prefix.length===line.text.length)return 0;
    let columns=0;
    for(const char of prefix)columns+=char==='\t'?4-columns%4:1;
    return columns;
}
export function sourceMargin(line,baseline) {
    if(line.left||!Number.isSafeInteger(baseline)||baseline<=0)return {bytes:0,columns:0};
    let bytes=0,columns=0;
    for(const char of line.text){
        if(char!==' '&&char!=='\t')break;
        columns+=char==='\t'?4-columns%4:1;bytes++;
    }
    return {bytes,columns:Math.max(0,columns-baseline)};
}
