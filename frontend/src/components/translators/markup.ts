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
