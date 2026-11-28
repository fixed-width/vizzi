import {planReflow} from './reflow.js';

const noMargin = {bytes: 0, columns: 0};
const originalLine = () => null;

function measurementFont(style) {
    return `${style.fontStyle} ${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
}

function measureCode(context, font, fontSize) {
    if (context) {
        context.font = font;
        context.fontKerning = 'none';
    }
    const width = text => context ? context.measureText(text).width : text.length * parseFloat(fontSize) * .6;
    const tab = width('    ');
    return text => {
        let result = 0;
        const parts = text.split('\t');
        for (const [index, part] of parts.entries()) {
            result += width(part);
            if (index < parts.length - 1) result = (Math.floor(result / tab) + 1) * tab;
        }
        return result;
    };
}

function samePlan(before, after) {
    if (before === after) return true;
    if (!before || !after || before.length !== after.length) return false;
    return before.every((segment, index) => {
        const next = after[index];
        return segment.start === next.start && segment.end === next.end && segment.indent === next.indent;
    });
}

export class SourceLayout {
    #element;
    #context;
    #plans = new WeakMap();
    #pending = false;
    #selectionChanged;

    constructor(element, resume) {
        this.#element = element;
        const document = element.ownerDocument;
        try {
            this.#context = document.createElement('canvas').getContext('2d');
        } catch {
            this.#context = null;
        }
        this.#selectionChanged = () => {
            if (this.#pending && !this.selected()) {
                this.#pending = false;
                resume();
            }
        };
        document.addEventListener('selectionchange', this.#selectionChanged);
    }

    selected() {
        const selection = this.#element.ownerDocument.getSelection();
        if (!selection || selection.isCollapsed) return false;
        for (let index = 0; index < selection.rangeCount; index++) {
            if (selection.getRangeAt(index).intersectsNode(this.#element)) return true;
        }
        return false;
    }

    defer(shouldDefer = () => true) {
        this.#pending = this.selected() && shouldDefer();
        return this.#pending;
    }

    measure(style) {
        return measureCode(this.#context, measurementFont(style), style.fontSize);
    }

    // Retain one plan per mounted row and preserve its identity when the segments are equal.
    planRows(width, enabled, {style, endInsetFor} = {}) {
        if (!enabled || !this.#context || width <= 0) return originalLine;
        style ??= this.#element.ownerDocument.defaultView.getComputedStyle(this.#element);
        const font = measurementFont(style);
        let measure;
        return (row, line, margin = noMargin) => {
            if (!line.reflow || line.left) return null;
            const endInset = line.right ? endInsetFor?.(row) : undefined;
            const previous = this.#plans.get(row);
            if (previous?.line === line && previous.font === font && previous.width === width &&
                previous.bytes === margin.bytes && previous.columns === margin.columns && previous.endInset === endInset) {
                return previous.plan;
            }

            measure ??= measureCode(this.#context, font, style.fontSize);
            const available = endInset === undefined ? width : width - endInset - measure('…');
            const segments = planReflow(line, available, measure, margin);
            let plan = segments.length > 1 ? segments : null;
            if (previous && samePlan(previous.plan, plan)) plan = previous.plan;
            this.#plans.set(row, {line, font, width, bytes: margin.bytes, columns: margin.columns, endInset, plan});
            return plan;
        };
    }

    dispose() {
        this.#element.ownerDocument.removeEventListener('selectionchange', this.#selectionChanged);
    }
}
