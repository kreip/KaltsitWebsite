/**
 * A selection without the blanks and punctuation at its edges: a double click
 * takes the space after a word, a drag a comma or a quote — wrapping that in a
 * tag, or looking it up, would carry them along («Гвард,» → «Гвард»). Inner
 * hyphens and apostrophes stay («Фристон-3», «Ch'en»).
 */

/** A character cut from the edges: a blank or punctuation. */
const EDGE = /[\s\p{P}]/u;

export function trimEdges(text: string): string {
    const chars = [...text];
    let start = 0;
    let end = chars.length;
    while (start < end && EDGE.test(chars[start])) start++;
    while (end > start && EDGE.test(chars[end - 1])) end--;
    return chars.slice(start, end).join("");
}

/** The range moved inward past the edge characters of its text nodes (a range ending at an element stays). */
export function trimRange(range: Range): Range {
    const r = range.cloneRange();
    const { startContainer, endContainer } = r;
    if (startContainer.nodeType === Node.TEXT_NODE) {
        const s = startContainer.textContent ?? "";
        let at = r.startOffset;
        const limit = startContainer === endContainer ? r.endOffset : s.length;
        while (at < limit && EDGE.test(s[at])) at++;
        r.setStart(startContainer, at);
    }
    if (endContainer.nodeType === Node.TEXT_NODE) {
        const s = endContainer.textContent ?? "";
        let at = r.endOffset;
        const limit = startContainer === endContainer ? r.startOffset : 0;
        while (at > limit && EDGE.test(s[at - 1])) at--;
        r.setEnd(endContainer, at);
    }
    return r;
}
