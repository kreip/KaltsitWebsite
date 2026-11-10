/**
 * Comparing two versions of a translation by words: what the new text
 * added and what it dropped from the old one, inside the text.
 */

export interface IDiffPart {
    kind: "same" | "add" | "del";
    text: string;
}

/**
 * A text as the pieces compared: a word (letters, digits, with an inner
 * apostrophe or hyphen), a run of blanks, a tag (`<i>`), a substitution
 * (`{@nickname}`, `{0}`), or one other character.
 */
export function diffTokens(text: string): string[] {
    return text.match(/<[^<>\n]*>|\{[^{}\n]*\}|[\p{L}\p{N}]+(?:['’-][\p{L}\p{N}]+)*|\s+|[^\s]/gu) ?? [];
}

function push(out: IDiffPart[], kind: IDiffPart["kind"], text: string): void {
    const last = out[out.length - 1];
    if (last?.kind === kind) last.text += text;
    else out.push({ kind, text });
}

/** The longest common subsequence of the two token lists, as the parts in order. */
function lcsParts(a: readonly string[], b: readonly string[]): IDiffPart[] {
    const n = a.length;
    const m = b.length;
    // len[i][j]: the longest common subsequence of a[i..] and b[j..].
    const len: Uint16Array[] = Array.from({ length: n + 1 }, () => new Uint16Array(m + 1));
    for (let i = n - 1; i >= 0; i--) for (let j = m - 1; j >= 0; j--) len[i][j] = a[i] === b[j] ? len[i + 1][j + 1] + 1 : Math.max(len[i + 1][j], len[i][j + 1]);
    const out: IDiffPart[] = [];
    let i = 0;
    let j = 0;
    while (i < n && j < m) {
        if (a[i] === b[j]) {
            push(out, "same", a[i]);
            i++;
            j++;
        } else if (len[i + 1][j] >= len[i][j + 1]) push(out, "del", a[i++]);
        else push(out, "add", b[j++]);
    }
    while (i < n) push(out, "del", a[i++]);
    while (j < m) push(out, "add", b[j++]);
    return out;
}

/**
 * What changed from `before` to `after`, by words: the parts of the new
 * text in order, with the dropped words where they stood. A blank between
 * two changed words goes with them, so a rewritten phrase reads as one
 * dropped run and one added run, the dropped one first.
 */
export function diffWords(before: string, after: string): IDiffPart[] {
    const parts = lcsParts(diffTokens(before), diffTokens(after));
    // A blank kept between two changes joins them.
    const joined: IDiffPart[] = [];
    parts.forEach((p, i) => {
        const between = p.kind === "same" && p.text.trim() === "" && i > 0 && i < parts.length - 1 && parts[i - 1].kind !== "same" && parts[i + 1].kind !== "same";
        if (between) {
            joined.push({ kind: "del", text: p.text }, { kind: "add", text: p.text });
        } else joined.push({ ...p });
    });
    // Within a run of changes: everything dropped, then everything added.
    const out: IDiffPart[] = [];
    let del = "";
    let add = "";
    const flush = () => {
        // Blanks both runs start or end with were not changed.
        let tail = "";
        if (del && add) {
            const head = /^\s*/.exec(del)?.[0] ?? "";
            if (head && add.startsWith(head) && del.trim() && add.trim()) {
                push(out, "same", head);
                del = del.slice(head.length);
                add = add.slice(head.length);
            }
            const end = /\s*$/.exec(del)?.[0] ?? "";
            if (end && add.endsWith(end) && del.trim() && add.trim()) {
                tail = end;
                del = del.slice(0, del.length - end.length);
                add = add.slice(0, add.length - end.length);
            }
        }
        if (del) push(out, "del", del);
        if (add) push(out, "add", add);
        if (tail) push(out, "same", tail);
        del = "";
        add = "";
    };
    for (const p of joined) {
        if (p.kind === "del") del += p.text;
        else if (p.kind === "add") add += p.text;
        else {
            flush();
            push(out, "same", p.text);
        }
    }
    flush();
    return out;
}
