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
    /** Substitutions (`{0}`, `{@nickname}`) missing from the text or not in the English (the nickname written as a word aside). */
    placeholders: string[];
    /** The English has the player's nickname `{@nickname}`, the text a word instead («Доктор»). */
    noNickname: boolean;
    /** Line breaks: the English's count and the text's, when they differ. */
    newlines: { en: number; ru: number } | null;
    /** How much longer than the game's Russian (0.45 = 45 %), when noticeably. */
    longer: number | null;
    /** The text says "ты" where the speaker's neighbouring lines say "вы", or the reverse: the neighbours' form. */
    address: Address | null;
    /**
     * «Check the gender» (a mark to look at, not an error): the gender the line should have — the speaker's (an
     * operator) against «я сделал / я рада», or the Chinese 她 / 他 against «он / она» — when the text gives the other.
     */
    gender: { want: Gender; by: "speaker" | "cn" } | null;
    /** «ты / вы» by the Chinese (a mark to look at): 您 (polite) → «вы», when the text says «ты». */
    addressCn: Address | null;
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

export type Gender = "f" | "m";

/*
 * «Check the gender» and «ты / вы» by the Chinese — the backend's very rules (`texts/forms.rs`), the same cases in
 * the tests. Marks to look at, never errors: they can be wrong.
 */

/** Short adjectives and the like after «я»: the feminine and the masculine. */
const SHORT: readonly (readonly [string, string])[] = [
    ["рада", "рад"],
    ["готова", "готов"],
    ["уверена", "уверен"],
    ["должна", "должен"],
    ["согласна", "согласен"],
    ["свободна", "свободен"],
    ["виновата", "виноват"],
    ["права", "прав"],
    ["жива", "жив"],
    ["счастлива", "счастлив"],
    ["благодарна", "благодарен"],
    ["способна", "способен"],
    ["обязана", "обязан"],
    ["знакома", "знаком"],
    ["занята", "занят"],
    ["сама", "сам"],
    ["одна", "один"],
    ["удивлена", "удивлён"],
    ["удивлена", "удивлен"],
    ["тронута", "тронут"],
    ["рождена", "рождён"],
    ["рождена", "рожден"],
    ["польщена", "польщён"],
    ["польщена", "польщен"],
    ["спокойна", "спокоен"],
    ["больна", "болен"],
    ["здорова", "здоров"],
    ["честна", "честен"],
    ["нужна", "нужен"],
    ["влюблена", "влюблён"],
    ["влюблена", "влюблен"],
    ["измотана", "измотан"],
    ["ранена", "ранен"],
    ["заинтересована", "заинтересован"],
    ["вынуждена", "вынужден"],
    ["поражена", "поражён"],
    ["поражена", "поражен"],
    ["разочарована", "разочарован"],
    ["напугана", "напуган"],
    ["потрясена", "потрясён"],
    ["потрясена", "потрясен"],
];

/** Words skipped between «я» and the word that tells the gender. */
const FILLERS = new Set([
    "не",
    "уже",
    "же",
    "ж",
    "ведь",
    "тоже",
    "бы",
    "так",
    "очень",
    "все",
    "всё",
    "еще",
    "ещё",
    "просто",
    "только",
    "сейчас",
    "правда",
    "и",
    "ни",
    "вот",
    "даже",
    "лишь",
    "никогда",
    "всегда",
    "давно",
    "наконец",
    "точно",
    "действительно",
    "совсем",
    "вовсе",
    "сегодня",
    "тогда",
    "почти",
    "здесь",
    "тут",
    "там",
    "ли",
    "-",
]);

/** Words in «-л» / «-ла» that are no past verbs. */
const NOT_VERBS = new Set(["мол", "пол", "стол", "зал", "бал", "вол", "кол", "тела", "дела", "мила", "сила", "пила", "стола", "зала", "пола", "игла", "мгла", "скала", "стрела", "пчела", "смола", "метла", "ангел"]);

function wordGender(w: string): Gender | null {
    for (const [f, m] of SHORT) {
        if (f === w) return "f";
        if (m === w) return "m";
    }
    if (NOT_VERBS.has(w) || !/^[а-яё]+$/.test(w)) return null;
    const n = [...w].length;
    if ((w.endsWith("лась") && n >= 6) || (w.endsWith("ла") && n >= 4)) return "f";
    if ((w.endsWith("лся") && n >= 5) || (w.endsWith("л") && n >= 4) || ["был", "шел", "шёл"].includes(w)) return "m";
    return null;
}

/** The text's words in lower case, tags and quoted speech left out (another's words: «…», “…”). */
function formWords(text: string): string[] {
    let out = "";
    let tag = false;
    let quote = 0;
    for (const c of text) {
        if (c === "<") tag = true;
        else if (c === ">" && tag) tag = false;
        else if (tag) continue;
        else if (c === "«" || c === "“") quote++;
        else if (c === "»" || c === "”") quote = Math.max(0, quote - 1);
        else if (quote === 0) out += c.toLowerCase();
    }
    return out.split(/[^\p{L}-]+/u).filter(Boolean);
}

/** The gender the Russian gives its speaker by «я …», when it gives one. */
export function genderOf(ru: string): Gender | null {
    const w = formWords(ru);
    let f = 0;
    let m = 0;
    w.forEach((word, i) => {
        if (word !== "я") return;
        const next = w.slice(i + 1, i + 5).find((x) => !FILLERS.has(x));
        const g = next ? wordGender(next) : null;
        if (g === "f") f++;
        else if (g === "m") m++;
    });
    return f > 0 && m === 0 ? "f" : m > 0 && f === 0 ? "m" : null;
}

const MASC = new Set(["он", "его", "него", "ему", "нему", "нем", "нём"]);
const FEM = new Set(["она", "ее", "её", "нее", "неё", "ей", "ней", "ею", "нею"]);
/** Chinese words holding 他 / 她 that are no "he" / "she". */
const CN_NOT_PERSON = ["他们", "她们", "其他", "他人", "吉他", "他乡", "他日", "他方", "他国", "利他", "他者", "无他", "排他"];

/** The gender the Chinese's 她 / 他 asks for, when the Russian has only the other's pronouns. */
export function pronounGender(ru: string, cn: string | null): Gender | null {
    if (!cn) return null;
    let rest = cn;
    for (const w of CN_NOT_PERSON) rest = rest.replaceAll(w, "");
    const want: Gender | null = rest.includes("她") && !rest.includes("他") ? "f" : rest.includes("他") && !rest.includes("她") ? "m" : null;
    if (!want) return null;
    const w = formWords(ru);
    const masc = w.some((x) => MASC.has(x));
    const fem = w.some((x) => FEM.has(x));
    return (want === "f" && masc && !fem) || (want === "m" && fem && !masc) ? want : null;
}

/** «ты / вы» by the Chinese: 您 (polite) → «вы», when the text says «ты» (你们 / 您们, plural, are left alone). */
export function cnAddress(ru: string, cn: string | null): Address | null {
    if (!cn || cn.includes("你们") || cn.includes("您们")) return null;
    // Only 您 asks for a form: 你 is the plain "you", and «вы» for it is the translator's choice.
    if (!cn.includes("您") || cn.includes("你")) return null;
    const want: Address = "вы";
    const own = addressOf(formWords(ru).join(" "));
    return own && own !== want ? want : null;
}

/** «Check the gender» of a text of a line: by its speaker, else by the Chinese. */
export function genderCheck(text: string, line: TranslatorLine): IVariantCheck["gender"] {
    const speaker = line.speakerGender === "f" || line.speakerGender === "m" ? line.speakerGender : null;
    const own = speaker ? genderOf(text) : null;
    if (speaker && own && own !== speaker) return { want: speaker, by: "speaker" };
    const pronoun = pronounGender(text, line.cn);
    return pronoun ? { want: pronoun, by: "cn" } : null;
}

/** Every mark of one variant's text against its line. `variant` names it as the backend does (`game`, `sova`, `proposal:12`). */
const NICKNAME = /\{@nickname\}/gi;

export function checkVariant(text: string, variant: string, line: TranslatorLine, neighbours: Address | null): IVariantCheck {
    // The nickname written as a word: its own mark, not a broken tag.
    const noNickname = /\{@nickname\}/i.test(line.en) && !/\{@nickname\}/i.test(text);
    const en = noNickname ? line.en.replace(NICKNAME, "Nickname") : line.en;
    const enTokens = tokens(en);
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
    const tags = tagHealth(en, text);
    return {
        glossary: line.variantGlossary.find((v) => v.variant === variant)?.missing ?? [],
        brokenTag: tags.broken,
        noHighlight: tags.noHighlight,
        placeholders: [...difference(enTokens, ruTokens), ...difference(ruTokens, enTokens)],
        noNickname,
        newlines: enBreaks === ruBreaks ? null : { en: enBreaks, ru: ruBreaks },
        longer,
        address: own && neighbours && own !== neighbours ? neighbours : null,
        gender: genderCheck(text, line),
        addressCn: cnAddress(text, line.cn),
    };
}

/**
 * How many errors a text has (each glossary term counts; length is no error, nor «no highlight»: the text is
 * whole and the plugin may colour it by its own rules): "Approve" asks first when any.
 */
export function problemCount(c: IVariantCheck | undefined): number {
    if (!c) return 0;
    return c.glossary.length + Number(c.brokenTag || c.placeholders.length > 0) + Number(c.noNickname) + Number(c.newlines !== null) + Number(c.address !== null);
}

/** Whether a variant passes every check («no highlight» is no error). */
export function passes(c: IVariantCheck): boolean {
    return c.glossary.length === 0 && !c.brokenTag && c.placeholders.length === 0 && !c.noNickname && c.newlines === null && c.longer === null && c.address === null;
}

/** The problems that are no errors — «no highlight» (the text is whole), the marks to look at (they can be wrong). */
export const NOT_ERRORS: ReadonlySet<ProblemKind> = new Set(["noHighlight", "gender", "addressCn"]);

/** Whether a line's problems hold an error — «Status: every error»; the others are only their own filter items. */
export function hasError(problems: ReadonlySet<ProblemKind>): boolean {
    for (const p of problems) if (!NOT_ERRORS.has(p)) return true;
    return false;
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
export const PROBLEM_KINDS = ["glossary", "brokenTag", "nickname", "noHighlight", "newlines", "longer", "address", "gender", "addressCn"] as const;
export type ProblemKind = (typeof PROBLEM_KINDS)[number];

/**
 * What is wrong with the text a line stands with — what its folded row shows:
 * the approved text once approved, else the game's — its glossary forms,
 * tags, line breaks, ты / вы; and, until approved, whether a proposal is
 * noticeably longer. For an approved text the glossary counts only when the
 * line's terms say a form is missing (a term changed after the approval):
 * the approval checked the rest.
 */
export function lineProblems(shown: IVariantCheck | undefined, settled: boolean, glossaryMissing: boolean, checks: ReadonlyMap<string, IVariantCheck>): Set<ProblemKind> {
    const out = new Set<ProblemKind>();
    if (shown) {
        if (settled ? glossaryMissing : shown.glossary.length > 0) out.add("glossary");
        if (shown.brokenTag || shown.placeholders.length > 0) out.add("brokenTag");
        if (shown.noNickname) out.add("nickname");
        if (shown.noHighlight) out.add("noHighlight");
        if (shown.newlines) out.add("newlines");
        if (shown.address) out.add("address");
        if (shown.gender) out.add("gender");
        if (shown.addressCn) out.add("addressCn");
    }
    if (settled) return out;
    for (const [variant, c] of checks) if (variant.startsWith("proposal:") && c.longer !== null) out.add("longer");
    return out;
}
