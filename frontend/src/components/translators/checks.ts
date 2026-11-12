/**
 * The automatic marks on every variant of a line, each shown only when something is wrong, and the one place where the
 * "recommended" variant is picked. Pure functions: the glossary check comes
 * from the backend (`TranslatorLine.variantGlossary`), the rest is worked out
 * here from the texts.
 */
import type { MissingTerm } from "#/types/generated/MissingTerm";
import type { TranslatorLine } from "#/types/generated/TranslatorLine";
import { TOKEN_SOURCE, tagHealth } from "./markup";

/** "Noticeably longer than in the game": both more than this share longer… */
export const LONGER_SHARE = 0.15;
/** …and more than this many characters longer (short lines are left alone). */
export const LONGER_CHARS = 15;
/** Lines of the same speaker looked at on each side for "ты / вы". */
export const ADDRESS_WINDOW = 5;

export type Address = "ты" | "вы";

export interface IVariantCheck {
    /** Glossary terms whose form the text lacks. */
    glossary: readonly MissingTerm[];
    /** A tag broken: left open, half a tag, a stray closer, a substitution lost or added, a tag the English lacks (`markup.ts`, `tagHealth`). */
    brokenTag: boolean;
    /** A tag of the English missing, the text otherwise whole: the game shows it without its highlight. */
    noHighlight: boolean;
    /** Substitutions (`{0}`, `{@nickname}`) missing from the text or not in the English. */
    placeholders: string[];
    /** Line breaks: the English's count and the text's, when they differ. */
    newlines: { en: number; ru: number } | null;
    /** How much longer than the game's Russian (0.45 = 45 %), when noticeably. */
    longer: number | null;
    /** The text says "ты" where the speaker's neighbouring lines say "вы", or the reverse: the neighbours' form. */
    address: Address | null;
}

/** Literal `\n` as a line break, trimmed: what the reader sees. */
function plainBreaks(s: string): string {
    return s.replaceAll("\\n", "\n").trim();
}

/** The text without tags and substitutions, for its length. */
function visible(s: string): string {
    return plainBreaks(s)
        .replace(/<[^<>]*>/g, "")
        .replace(new RegExp(TOKEN_SOURCE, "g"), "Доктор");
}

function tokens(s: string): string[] {
    return (s.match(new RegExp(TOKEN_SOURCE, "g")) ?? []).map((t) => t.toLowerCase()).sort();
}

function breaks(s: string): number {
    return plainBreaks(s).split("\n").length - 1;
}

/** Tokens in one list and not the other (each extra copy counts). */
function difference(a: readonly string[], b: readonly string[]): string[] {
    const left = [...b];
    const out: string[] = [];
    for (const x of a) {
        const at = left.indexOf(x);
        if (at >= 0) left.splice(at, 1);
        else out.push(x);
    }
    return out;
}

const TY = new Set(["ты", "тебя", "тебе", "тобой", "тобою", "твой", "твоя", "твоё", "твое", "твои", "твоего", "твоей", "твоему", "твоим", "твоими", "твоих", "твою"]);
const VY = new Set(["вы", "вас", "вам", "вами", "ваш", "ваша", "ваше", "ваши", "вашего", "вашей", "вашему", "вашим", "вашими", "ваших", "вашу"]);

/** How a text addresses someone: the form used more, or null when it uses neither (or both equally). */
export function addressOf(text: string): Address | null {
    let ty = 0;
    let vy = 0;
    for (const w of text.toLowerCase().split(/[^а-яё]+/)) {
        if (TY.has(w)) ty++;
        else if (VY.has(w)) vy++;
    }
    return ty > vy ? "ты" : vy > ty ? "вы" : null;
}

/**
 * How the speaker of line `index` addresses people in the scene: the form most
 * of their nearest lines use in the game's Russian (up to ADDRESS_WINDOW on each
 * side, the line itself left out), when at least two agree.
 */
export function neighbourAddress(lines: readonly TranslatorLine[], index: number): Address | null {
    const speaker = lines[index].speakerEn;
    if (!speaker) return null;
    const counts = { ты: 0, вы: 0 };
    for (const step of [-1, 1]) {
        let seen = 0;
        for (let i = index + step; i >= 0 && i < lines.length && seen < ADDRESS_WINDOW; i += step) {
            if (lines[i].speakerEn !== speaker) continue;
            seen++;
            const a = addressOf(lines[i].ruGame ?? "");
            if (a) counts[a]++;
        }
    }
    if (counts.ты === counts.вы || Math.max(counts.ты, counts.вы) < 2) return null;
    return counts.ты > counts.вы ? "ты" : "вы";
}

/** Every mark of one variant's text against its line. `variant` names it as the backend does (`game`, `sova`, `proposal:12`). */
export function checkVariant(text: string, variant: string, line: TranslatorLine, neighbours: Address | null): IVariantCheck {
    const enTokens = tokens(line.en);
    const ruTokens = tokens(text);
    const enBreaks = breaks(line.en);
    const ruBreaks = breaks(text);
    let longer: number | null = null;
    if (variant !== "game" && line.ruGame) {
        const base = visible(line.ruGame).length;
        const own = visible(text).length;
        if (base > 0 && own - base > LONGER_CHARS && own / base - 1 > LONGER_SHARE) longer = own / base - 1;
    }
    const own = addressOf(text);
    const tags = tagHealth(line.en, text);
    return {
        glossary: line.variantGlossary.find((v) => v.variant === variant)?.missing ?? [],
        brokenTag: tags.broken,
        noHighlight: tags.noHighlight,
        placeholders: [...difference(enTokens, ruTokens), ...difference(ruTokens, enTokens)],
        newlines: enBreaks === ruBreaks ? null : { en: enBreaks, ru: ruBreaks },
        longer,
        address: own && neighbours && own !== neighbours ? neighbours : null,
    };
}

/** Whether a variant passes every check. */
export function passes(c: IVariantCheck): boolean {
    return c.glossary.length === 0 && !c.brokenTag && !c.noHighlight && c.placeholders.length === 0 && c.newlines === null && c.longer === null && c.address === null;
}

export interface ICandidate {
    /** `game`, `sova`, `ai` or `proposal:<id>`. */
    variant: string;
    agreements: number;
    check: IVariantCheck;
}

/** Which rule picks "recommended": the team's call. */
export type RecommendRule = "A" | "B";

/**
 * The team's rule is the lead's switch above the story table (kept by the
 * backend, `TranslatorsMe.recommendRule`); RECOMMEND_RULE is the default.
 *
 * Rule B (the default): the game's Russian that passes every check needs no
 * change, so nothing is recommended; only when the game's text has a mark
 * (glossary, tags, substitutions, line breaks, length, ты / вы) is the best
 * passing variant recommended: the proposal with the most "agree", else
 * Sova's, else the AI's.
 *
 * Rule A: among the variants that pass every check, the
 * proposal with the most "agree", else Sova's, else the AI's, else the
 * game's Russian; it put a star on most lines.
 */
export const RECOMMEND_RULE: RecommendRule = "B";

/**
 * THE RULE for "recommended" (the one place to change it, with
 * RECOMMEND_RULE): `candidates` hold the game's Russian as `game`. None when
 * the lead has already approved a variant.
 */
export function recommend(candidates: readonly ICandidate[], approved: boolean, rule: RecommendRule = RECOMMEND_RULE): string | null {
    if (approved) return null;
    const game = candidates.find((c) => c.variant === "game");
    if (rule === "B" && game && passes(game.check)) return null;
    const ok = candidates.filter((c) => passes(c.check));
    const proposals = ok.filter((c) => c.variant.startsWith("proposal:")).sort((a, b) => b.agreements - a.agreements);
    const best = proposals[0] ?? ok.find((c) => c.variant === "sova") ?? ok.find((c) => c.variant === "ai");
    const pick = rule === "A" ? (best ?? ok.find((c) => c.variant === "game")) : best;
    return pick?.variant ?? null;
}

/** The problems the filters and the yellow rows name. */
export const PROBLEM_KINDS = ["glossary", "brokenTag", "noHighlight", "newlines", "longer", "address"] as const;
export type ProblemKind = (typeof PROBLEM_KINDS)[number];

/**
 * What is wrong with a line as the game shows it — its glossary forms, tags,
 * line breaks, ты / вы — and whether a proposal is noticeably longer. None
 * once the line is approved (it was checked).
 */
export function lineProblems(approved: boolean, checks: ReadonlyMap<string, IVariantCheck>): Set<ProblemKind> {
    const out = new Set<ProblemKind>();
    if (approved) return out;
    const game = checks.get("game");
    if (game) {
        if (game.glossary.length > 0) out.add("glossary");
        if (game.brokenTag || game.placeholders.length > 0) out.add("brokenTag");
        if (game.noHighlight) out.add("noHighlight");
        if (game.newlines) out.add("newlines");
        if (game.address) out.add("address");
    }
    for (const [variant, c] of checks) if (variant.startsWith("proposal:") && c.longer !== null) out.add("longer");
    return out;
}
