/**
 * A selection without the blanks and punctuation at its edges: a double click
 * takes the space after a word, a drag a comma or a quote — wrapping that in a
 * tag, or looking it up, would carry them along («Гвард,» → «Гвард»). Inner
 * hyphens and apostrophes stay («Фристон-3», «Ch'en»).
 */

/** A character cut from the edges: a blank (the editor's zero-width guard too) or punctuation. */
const EDGE = /[\s\u200B\p{P}]/u;

export function trimEdges(text: string): string {
    const chars = [...text];
    let start = 0;
    let end = chars.length;
    while (start < end && EDGE.test(chars[start])) start++;
    while (end > start && EDGE.test(chars[end - 1])) end--;
    return chars.slice(start, end).join("");
}

/** An editor's chip (`data-token`) is one thing, whatever its text: never cut into. */
function chipOf(text: Text): Element | null {
    return text.parentElement?.closest("[data-token]") ?? null;
}

/** The text nodes the range touches, in order. */
function textsIn(range: Range): Text[] {
    const root = range.commonAncestorContainer;
    if (root.nodeType === Node.TEXT_NODE) return [root as Text];
    const walk = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    const out: Text[] = [];
    for (let node = walk.nextNode(); node; node = walk.nextNode()) if (range.intersectsNode(node)) out.push(node as Text);
    return out;
}

/**
 * The range moved inward past the edge characters, across text nodes (the space after a word may be a node of its
 * own, or inside the next tag's span); collapsed when nothing else is in it.
 */
export function trimRange(range: Range): Range {
    const out = range.cloneRange();
    const spans = textsIn(range).map((node) => ({
        node,
        from: node === range.startContainer ? range.startOffset : 0,
        to: node === range.endContainer ? range.endOffset : node.data.length,
    }));
    const inner = spans.filter(({ node, from, to }) => from < to && (chipOf(node) !== null || [...node.data.slice(from, to)].some((c) => !EDGE.test(c))));
    if (inner.length === 0) {
        out.collapse(true);
        return out;
    }
    const last = inner[inner.length - 1];
    let { from } = inner[0];
    let { to } = last;
    while (EDGE.test(inner[0].node.data.charAt(from))) from++;
    while (EDGE.test(last.node.data.charAt(to - 1))) to--;
    const chipFirst = chipOf(inner[0].node);
    const chipLast = chipOf(last.node);
    if (chipFirst) out.setStartBefore(chipFirst);
    else out.setStart(inner[0].node, from);
    if (chipLast) out.setEndAfter(chipLast);
    else out.setEnd(last.node, to);
    return out;
}
