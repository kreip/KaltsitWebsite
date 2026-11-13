/**
 * The markup of a game text, parsed once for everything the section does
 * with it: showing a text the way the game does (StoryText.tsx), the
 * editor's wrapped spans and palette (TokenEditor.tsx), and the check that
 * a translation keeps the original's tags (checks.ts).
 *
 * Spans: a game style `<@ba.vup>…</>`, a game term `<$ba.stun>…</>`, a story
 * paragraph `<p=1>…</>` (all three closed by `</>`), and Unity's `<i>`,
 * `<b>`, `<u>`, `<color=…>`, `<size=…>` with their own closers. A
 * substitution `{@nickname}`, `{0}`, `{atk:0%}` is a token. Anything else in
 * angle brackets is text: the game writes names that way (`<Roadblock>`).
 *
 * The parse loses nothing: `serializeMarkup(parseMarkup(x)) === x`. A closer
 * with nothing to close stays text; a span never closed has an empty closer.
 */

import { PLUGIN_STYLES } from "./pluginStyles";

export type SpanType = "style" | "term" | "paragraph" | "inline" | "color" | "size";

export interface IMarkSpan {
    kind: "span";
    type: SpanType;
    /** What names it in a tag set: `@ba.vup`, `$ba.stun`, `p`, `i`, `color`, `size`. */
    name: string;
    /** The opener and the closer as written (`<color=#FF0000>`, `</color>`); the closer is empty when the span was never closed. */
    open: string;
    close: string;
    children: MarkNode[];
}

export type MarkNode = { kind: "text"; value: string } | { kind: "token"; raw: string } | IMarkSpan;

const TAG_NAME = String.raw`[@$][A-Za-z0-9_]+(?:\.[A-Za-z0-9_]+)*`;

/** A substitution: `{@nickname}`, `{0}`, `{atk:0%}`, `{-cost}`. */
export const TOKEN_SOURCE = String.raw`\{[^{}<>\n]{1,60}\}`;

function scanner(): RegExp {
    // 1 game opener · 2 universal closer · 3 paragraph · 4 inline opener · 5 inline closer · 6 colour / size opener · 7 colour / size closer · 8 substitution
    return new RegExp(`<(${TAG_NAME})>|(</>)|<(p=\\d+)>|<(i|b|u)>|</(i|b|u)>|<(color|size)[ =][^<>]*>|</(color|size)>|(${TOKEN_SOURCE})`, "gi");
}

/** The spans `</>` closes. */
const UNIVERSAL: readonly SpanType[] = ["style", "term", "paragraph"];

export function parseMarkup(text: string): MarkNode[] {
    const root: MarkNode[] = [];
    const open: IMarkSpan[] = [];
    const siblings = () => (open.length ? open[open.length - 1].children : root);
    const pushText = (value: string) => {
        if (!value) return;
        const list = siblings();
        const last = list[list.length - 1];
        if (last && last.kind === "text") last.value += value;
        else list.push({ kind: "text", value });
    };
    const start = (span: IMarkSpan) => {
        siblings().push(span);
        open.push(span);
    };
    /** Close the innermost span `matches`; the spans opened inside it end there unclosed. False when there is none. */
    const end = (matches: (span: IMarkSpan) => boolean, closer: string): boolean => {
        for (let i = open.length - 1; i >= 0; i--) {
            if (matches(open[i])) {
                open[i].close = closer;
                open.length = i;
                return true;
            }
        }
        return false;
    };
    const scan = scanner();
    let last = 0;
    for (let m = scan.exec(text); m !== null; m = scan.exec(text)) {
        pushText(text.slice(last, m.index));
        last = m.index + m[0].length;
        const raw = m[0];
        if (m[1]) start({ kind: "span", type: m[1].startsWith("$") ? "term" : "style", name: m[1], open: raw, close: "", children: [] });
        else if (m[2]) {
            if (!end((s) => UNIVERSAL.includes(s.type), raw)) pushText(raw);
        } else if (m[3]) start({ kind: "span", type: "paragraph", name: "p", open: raw, close: "", children: [] });
        else if (m[4]) start({ kind: "span", type: "inline", name: m[4].toLowerCase(), open: raw, close: "", children: [] });
        else if (m[5]) {
            const name = m[5].toLowerCase();
            if (!end((s) => s.type === "inline" && s.name === name, raw)) pushText(raw);
        } else if (m[6]) start({ kind: "span", type: m[6].toLowerCase() === "color" ? "color" : "size", name: m[6].toLowerCase(), open: raw, close: "", children: [] });
        else if (m[7]) {
            const name = m[7].toLowerCase();
            if (!end((s) => s.name === name && (s.type === "color" || s.type === "size"), raw)) pushText(raw);
        } else if (m[8]) siblings().push({ kind: "token", raw });
    }
    pushText(text.slice(last));
    return root;
}

export function serializeMarkup(nodes: readonly MarkNode[]): string {
    let out = "";
    for (const n of nodes) {
        if (n.kind === "text") out += n.value;
        else if (n.kind === "token") out += n.raw;
        else out += n.open + serializeMarkup(n.children) + n.close;
    }
    return out;
}

/** The text without its tags (the substitutions stay). */
export function plainMarkup(nodes: readonly MarkNode[]): string {
    let out = "";
    for (const n of nodes) out += n.kind === "text" ? n.value : n.kind === "token" ? n.raw : plainMarkup(n.children);
    return out;
}

function walk(nodes: readonly MarkNode[], each: (span: IMarkSpan) => void): void {
    for (const n of nodes) {
        if (n.kind !== "span") continue;
        each(n);
        walk(n.children, each);
    }
}

/** A tag a translation can be wrapped in: its opener and closer as the original writes them. */
export interface ITagPair {
    type: SpanType;
    name: string;
    open: string;
    close: string;
}

/** The closer a span of a type takes when the text never wrote one. */
function closerOf(span: IMarkSpan): string {
    if (span.close) return span.close;
    return span.type === "inline" || span.type === "color" || span.type === "size" ? `</${span.name}>` : "</>";
}

/** The distinct tags of a text, in order: what the editor's palette offers (a story's paragraph split is not offered). */
export function tagPairs(text: string): ITagPair[] {
    const out: ITagPair[] = [];
    walk(parseMarkup(text), (s) => {
        if (s.type === "paragraph") return;
        if (!out.some((p) => p.open === s.open)) out.push({ type: s.type, name: s.name, open: s.open, close: closerOf(s) });
    });
    return out;
}

/** The substitutions of a text, each once, in order (`{@Nickname}` as `{@nickname}`). */
export function tokensIn(text: string): string[] {
    const out: string[] = [];
    for (const raw of text.match(new RegExp(TOKEN_SOURCE, "g")) ?? []) {
        const token = raw.toLowerCase() === "{@nickname}" ? "{@nickname}" : raw;
        if (!out.includes(token)) out.push(token);
    }
    return out;
}

/** How many spans of each kind a text has (`@ba.vup` 2, `i` 1): two colours of one text count as `color` twice. */
export function tagCounts(text: string): Map<string, number> {
    const out = new Map<string, number>();
    walk(parseMarkup(text), (s) => out.set(s.name, (out.get(s.name) ?? 0) + 1));
    return out;
}

export interface ITagDiff {
    /** Tags the original has and the translation lacks (one entry per missing span). */
    missing: string[];
    /** Tags the translation has and the original does not. */
    extra: string[];
}

/** What the translation's tags lack against the original's, and what they add. */
export function tagDiff(original: string, translation: string): ITagDiff {
    const en = tagCounts(original);
    const ru = tagCounts(translation);
    const missing: string[] = [];
    const extra: string[] = [];
    for (const [name, n] of en) for (let i = ru.get(name) ?? 0; i < n; i++) missing.push(name);
    for (const [name, n] of ru) for (let i = en.get(name) ?? 0; i < n; i++) extra.push(name);
    return { missing, extra };
}

export function sameTags(original: string, translation: string): boolean {
    const d = tagDiff(original, translation);
    return d.missing.length === 0 && d.extra.length === 0;
}

/** Something that starts like a tag but is not one: half a tag, a closer with nothing to close. */
const GARBAGE = /<[@$/]/g;

interface IHealth {
    unclosed: number;
    garbage: number;
    tokens: string[];
}

function health(text: string): IHealth {
    const out: IHealth = { unclosed: 0, garbage: 0, tokens: [] };
    const visit = (nodes: readonly MarkNode[]) => {
        for (const n of nodes) {
            if (n.kind === "text") out.garbage += (n.value.match(GARBAGE) ?? []).length;
            else if (n.kind === "token") out.tokens.push(n.raw.toLowerCase());
            else {
                if (!n.close) out.unclosed++;
                visit(n.children);
            }
        }
    };
    visit(parseMarkup(text));
    out.tokens.sort();
    return out;
}

/** A span of the original with its text's place in the plain text (tags removed, substitutions kept). */
interface IPlainSpan {
    span: IMarkSpan;
    start: number;
    end: number;
}

/** The plain text of nodes and where each span stands in it. */
function plainSpans(nodes: readonly MarkNode[]): { plain: string; spans: IPlainSpan[] } {
    const spans: IPlainSpan[] = [];
    let plain = "";
    const visit = (list: readonly MarkNode[]) => {
        for (const n of list) {
            if (n.kind === "text") plain += n.value;
            else if (n.kind === "token") plain += n.raw;
            else {
                const start = plain.length;
                visit(n.children);
                spans.push({ span: n, start, end: plain.length });
            }
        }
    };
    visit(nodes);
    return { plain, spans };
}

/** A number as the plugin finds one to colour again: `230%`, `1.25`, full-width digits too. */
const NUMBER = /[0-9０-９]+(?:[.．][0-9０-９]+)?[%％]?/g;

function isAsciiWord(c: string | undefined): boolean {
    return c !== undefined && /[0-9A-Za-z]/.test(c);
}

/** Where `value` stands in `text`, as the plugin looks: an ASCII word only whole. */
function findAll(text: string, value: string): [number, number][] {
    const out: [number, number][] = [];
    if (!value) return out;
    let from = 0;
    while (from <= text.length - value.length) {
        const at = text.indexOf(value, from);
        if (at < 0) break;
        const end = at + value.length;
        const before = at > 0 && isAsciiWord(value[0]) && isAsciiWord(text[at - 1]);
        const after = end < text.length && isAsciiWord(value[value.length - 1]) && isAsciiWord(text[end]);
        if (!before && !after) out.push([at, end]);
        from = end;
    }
    return out;
}

/** Where a number (or a substitution standing for one) is in `text`, by its digits. */
function findNumbers(text: string, value: string): [number, number][] {
    const key = value.normalize("NFKC");
    const out: [number, number][] = [];
    for (const m of text.matchAll(new RegExp(`${TOKEN_SOURCE}|${NUMBER.source}`, "g"))) if (m[0].normalize("NFKC") === key) out.push([m.index, m.index + m[0].length]);
    return out;
}

/**
 * Whether the RichTextFix plugin puts the span's style back into a plain
 * translation (its `RichTextStyleRestorer`): the style covers the whole
 * text; or the span's text stands in the translation as it is (a number,
 * `SP`) as many times as in the original and only inside the span; or a
 * number (a substitution counts as one) of the span stands in both as many
 * times. A game style the plugin does not know is dropped before that.
 */
function pluginRestores(s: IPlainSpan, plain: string, translation: string): boolean {
    if (s.span.type === "paragraph") return false;
    // The game shows a term's words plain (no colour, no underline): nothing to lose.
    if (s.span.type === "term") return true;
    if (s.span.type === "style" && !PLUGIN_STYLES.has(s.span.name.slice(1))) return false;
    const text = plain.slice(s.start, s.end).trim();
    if (!text) return false;
    if (text === plain.trim()) return true;
    const inside = (hits: [number, number][]) => hits.length > 0 && hits.every(([a, b]) => a >= s.start && b <= s.end);
    const literal = findAll(plain, text);
    if (inside(literal) && literal.length === findAll(translation, text).length) return true;
    const numbers = new Set((text.match(new RegExp(`${TOKEN_SOURCE}|${NUMBER.source}`, "g")) ?? []).map((n) => n));
    for (const n of numbers) {
        const hits = findNumbers(plain, n);
        if (inside(hits) && hits.length === findNumbers(translation, n).length) return true;
    }
    return false;
}

/**
 * The original's spans whose highlight the game would show without, given
 * the translation: the translation lacks the tag and the plugin would not
 * put it back. A translation with tags of its own gets only whole-text
 * styles back from the plugin; a plain one, what `pluginRestores` says. A
 * term `<$…>` is never one: the game shows its words plain.
 */
export function lostHighlights(original: string, translation: string): string[] {
    const { plain, spans } = plainSpans(parseMarkup(original));
    if (spans.length === 0) return [];
    const ruTags = tagCounts(translation);
    const wholeText = (s: IPlainSpan) => plain.slice(s.start, s.end).trim() === plain.trim() && plain.trim() !== "";
    if (ruTags.size > 0) {
        // Per name: the spans the translation lacks, the whole-text ones (put back) not counted.
        const out: string[] = [];
        const byName = new Map<string, IPlainSpan[]>();
        for (const s of spans) byName.set(s.span.name, [...(byName.get(s.span.name) ?? []), s]);
        for (const [name, list] of byName) {
            if (name.startsWith("$")) continue;
            const missing = list.length - (ruTags.get(name) ?? 0);
            const lost = missing - list.filter(wholeText).length;
            for (let i = 0; i < lost; i++) out.push(name);
        }
        return out;
    }
    return spans.filter((s) => !pluginRestores(s, plain, translation)).map((s) => s.span.name);
}

/**
 * A translation's tags against the original's (the backend's
 * `texts/problems.rs` says the same): `broken` — a tag left open or broken,
 * a closer with nothing to close, a substitution `{…}` lost or added, a tag
 * the original lacks; `noHighlight` — a tag of the original missing that the
 * RichTextFix plugin would not put back (`lostHighlights`), the text
 * otherwise whole.
 */
export function tagHealth(original: string, translation: string): { broken: boolean; noHighlight: boolean } {
    const e = health(original);
    const r = health(translation);
    const diff = tagDiff(original, translation);
    const broken = r.unclosed > e.unclosed || r.garbage > e.garbage || r.tokens.join("\u0000") !== e.tokens.join("\u0000") || diff.extra.length > 0;
    return { broken, noHighlight: !broken && lostHighlights(original, translation).length > 0 };
}
