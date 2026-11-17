/**
 * "Replace the word in other lines": a proposal that changed one word of the
 * text it started from into another word (`Бибек` → `Байбик`) — the same
 * words around it, one Cyrillic word for another, not a change of the word's
 * ending (`Бибек` → `Бибека` is grammar, not a new name).
 */

const WORD = /[\p{L}\p{N}]+/gu;
const CYRILLIC = /^[а-яё]+$/i;

function words(text: string): string[] {
    const plain = text
        .replaceAll("\\n", " ")
        .replace(/<[^<>]*>/g, " ")
        .replace(/\{[^{}]*\}/g, " ");
    return plain.match(WORD) ?? [];
}

function fold(s: string): string {
    return s.toLowerCase().replaceAll("ё", "е");
}

/** Russian endings: two words differing only in these are one word in two cases. */
const ENDINGS = new Set(["", "а", "я", "ы", "и", "е", "у", "ю", "о", "ь", "й", "ой", "ою", "ей", "ею", "ом", "ем", "ов", "ев", "ам", "ям", "ами", "ями", "ах", "ях", "ый", "ий", "ого", "его", "ому", "ему", "ым", "им", "ую", "юю", "ая", "яя", "ое", "ее", "ые", "ие", "ых", "их", "ыми", "ими"]);

function commonPrefix(a: string, b: string): number {
    let i = 0;
    while (i < a.length && i < b.length && a[i] === b[i]) i++;
    return i;
}

/** The one word `after` changed in `before`, as written in each; null when it changed something else (or more). */
export function wordSwap(before: string, after: string): { from: string; to: string } | null {
    const a = words(before);
    const b = words(after);
    if (a.length !== b.length) return null;
    let at = -1;
    for (let i = 0; i < a.length; i++) {
        if (a[i] === b[i]) continue;
        if (at >= 0) return null;
        at = i;
    }
    if (at < 0) return null;
    const [from, to] = [a[at], b[at]];
    if (!CYRILLIC.test(from) || !CYRILLIC.test(to) || from.length < 3 || to.length < 3) return null;
    const [f, t] = [fold(from), fold(to)];
    if (f === t) return null;
    // Only the ending changed: the same word in another case.
    const at2 = commonPrefix(f, t);
    if (ENDINGS.has(f.slice(at2)) && ENDINGS.has(t.slice(at2))) return null;
    return { from, to };
}
