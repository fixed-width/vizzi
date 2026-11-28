const encoder = new TextEncoder();
const decoder = new TextDecoder('utf-8', {fatal: true, ignoreBOM: true});

function styledLine(line) {
    const bytes = encoder.encode(line.text);
    const styles = line.styles ?? [];
    const text = (start, end) => decoder.decode(bytes.subarray(start, end));
    let styleIndex = 0;

    function append(target, start, end) {
        while (start < end && styleIndex < styles.length) {
            const style = styles[styleIndex];
            if (style.end <= start) {
                styleIndex++;
                continue;
            }
            if (style.start >= end) break;
            if (start < style.start) {
                target.append(text(start, style.start));
                start = style.start;
            }
            const stop = Math.min(end, style.end);
            const span = target.ownerDocument.createElement('span');
            span.className = `syntax-${style.kind}`;
            span.textContent = text(start, stop);
            target.append(span);
            start = stop;
            if (stop === style.end) styleIndex++;
        }
        if (start < end) target.append(text(start, end));
    }

    return {byteLength: bytes.length, text, append};
}

export function renderStyledText(node, line) {
    const source = styledLine(line);
    const content = node.ownerDocument.createDocumentFragment();
    if (line.left) content.append('…');
    source.append(content, 0, source.byteLength);
    if (line.right) content.append('…');
    node.replaceChildren(content);
}

export function copySourceSelection(element,rows,event) {
    const document=element.ownerDocument,selection=document.getSelection();
    if(!event.clipboardData||!selection||selection.isCollapsed||selection.rangeCount!==1)return;
    const range=selection.getRangeAt(0);
    if(!element.contains(range.commonAncestorContainer))return;
    const selected=rows.filter(row=>range.intersectsNode(row.text));
    if(!selected.length||selected.some(row=>row.line.left||row.line.right))return;
    const pieces=[];
    for(const [index,row] of selected.entries()){
        const walker=document.createTreeWalker(row.text,4);
        let node,text='',at=0,first=null;
        while((node=walker.nextNode())){
            if(!node.parentElement.closest('.source-chunk'))continue;
            if(range.intersectsNode(node)){
                const start=range.startContainer===node?range.startOffset:0;
                const end=range.endContainer===node?range.endOffset:node.length;
                if(start<end){first??=at+start;text+=node.textContent.slice(start,end);}
            }
            at+=node.length;
        }
        if(first===row.margin?.bytes&&first>0)text=row.line.text.slice(0,first)+text;
        const contents=document.createRange();
        contents.selectNodeContents(row.text);
        if(index<selected.length-1||range.compareBoundaryPoints(2,contents)>0)text+=row.line.ending??'\n';
        pieces.push(text);
    }
    event.clipboardData.setData('text/plain',pieces.join(''));
    event.preventDefault();
}

function appendClip(segment) {
    const mark = segment.ownerDocument.createElement('span');
    mark.className = 'source-clip';
    mark.textContent = '…';
    segment.append(mark);
}

// Display segments retain captured byte offsets through margins and clipping marks.
export function renderReflowText(node, line, plan, margin = {bytes: 0, columns: 0}) {
    const document = node.ownerDocument;
    const source = styledLine(line);
    const pieces = plan ?? [{start: 0, end: source.byteLength, indent: 0}];
    const content = document.createDocumentFragment();

    for (const [index, piece] of pieces.entries()) {
        const segment = document.createElement('span');
        segment.className = 'reflow-segment';
        segment.dataset.start = String(piece.start);
        segment.dataset.end = String(piece.end);
        segment.style.paddingLeft = piece.indent + 'px';
        if (index > 0) {
            const mark = document.createElement('span');
            mark.className = 'reflow-marker';
            mark.setAttribute('aria-hidden', 'true');
            segment.append(mark);
        } else if (line.left) {
            appendClip(segment);
        }

        const literal = document.createElement('span');
        literal.className = 'source-chunk';
        let target = literal;
        let start = piece.start;
        if (index === 0 && margin.bytes) {
            const hidden = document.createElement('span');
            hidden.className = 'source-margin';
            hidden.textContent = source.text(0, margin.bytes);
            literal.append(hidden);
            if (margin.columns) {
                const remainder = document.createElement('span');
                remainder.className = 'source-margin-remainder';
                remainder.setAttribute('aria-hidden', 'true');
                remainder.style.width = margin.columns + 'ch';
                literal.append(remainder);
            }
            target = document.createElement('span');
            literal.append(target);
            start = margin.bytes;
        }
        source.append(target, start, piece.end);
        segment.append(literal);
        if (index === pieces.length - 1 && line.right) appendClip(segment);
        content.append(segment);
    }
    node.replaceChildren(content);
}
