/**
 * Whether a Russian text holds a glossary term's form — the page's copy of
 * the backend's rule (`glossary/matcher.rs`: `has_form`, `stem_match`,
 * `paradigm`), for the warning under the proposal box while one types.
 * Word by word in a row: the word is the form's stem with an ending (or an
 * adjective's suffix and ending: «урсусский»), or one of its singular case
 * forms by simple rules (Зоя: Зои, Зое, Зою, Зоей). «Доберманн» is not
 * «Доберман». `ё` = `е`, any case, quotes and «ООО» ignored.
 */
import type { LineTerm } from "#/types/generated/LineTerm";

const ENDINGS = ["ами", "ями", "ого", "его", "ому", "ему", "ыми", "ими", "ой", "ей", "ом", "ем", "ам", "ям", "ах", "ях", "ов", "ев", "ий", "ый", "ая", "яя", "ое", "ее", "ую", "юю", "ых", "их", "ые", "ие", "ым", "им", "ы", "и", "а", "я", "у", "ю", "е", "о", "ь", "й"];
const ADJECTIVE = ["овск", "евск", "инск", "ск", "ов", "ев", "ин"];

function fold(s: string): string {
    return s.toLowerCase().replaceAll("ё", "е");
}

function words(s: string): string[] {
    return fold(s)
        .split(/[^\p{L}\p{N}]+/u)
        .filter(Boolean);
}

/** The stem the glossary compares by: one ending cut, at least 3 letters left. */
export function stem(word: string): string {
    for (const e of ENDINGS) if (word.endsWith(e) && [...word.slice(0, word.length - e.length)].length >= 3) return word.slice(0, word.length - e.length);
    return word;
}

function stemMatch(word: string, st: string): boolean {
    if (!word.startsWith(st)) return false;
    const rest = word.slice(st.length);
    return rest === "" || ENDINGS.includes(rest) || ADJECTIVE.some((a) => rest.startsWith(a) && ENDINGS.includes(rest.slice(a.length)));
}

const HUSHING = "жшчщ";
const CONSONANT = "бвгджзклмнпрстфхцчшщ";

/** The singular endings of a word by the simple rules (stem, endings), or null. */
function paradigm(word: string): [string, readonly string[]] | null {
    const n = word.length;
    if (n < 2 || !/^\p{L}+$/u.test(word)) return null;
    const last = word[n - 1];
    const before = word[n - 2];
    const end2 = word.slice(n - 2);
    if (n >= 4) {
        if (end2 === "ый") return [word.slice(0, -2), ["ый", "ого", "ому", "ым", "ом"]];
        if (end2 === "ое") return [word.slice(0, -2), ["ое", "ого", "ому", "ым", "ом"]];
        if (end2 === "ая") return [word.slice(0, -2), HUSHING.includes(word[n - 3]) ? ["ая", "ей", "ой", "ую", "ею"] : ["ая", "ой", "ую", "ою"]];
    }
    if (last === "я") return [word.slice(0, -1), before === "и" ? ["я", "и", "ю", "ей", "ею"] : ["я", "и", "е", "ю", "ей", "ею"]];
    if (last === "а") return [word.slice(0, -1), HUSHING.includes(before) ? ["а", "и", "е", "у", "ей", "ею", "ой", "ою"] : "гкх".includes(before) ? ["а", "и", "е", "у", "ой", "ою"] : ["а", "ы", "е", "у", "ой", "ою"]];
    if (CONSONANT.includes(last)) return [word, HUSHING.includes(last) || last === "ц" ? ["", "а", "у", "ем", "ом", "е"] : ["", "а", "у", "ом", "е"]];
    return null;
}

function inflects(word: string, form: string): boolean {
    const p = paradigm(form);
    return p !== null && word.startsWith(p[0]) && p[1].includes(word.slice(p[0].length));
}

/** Whether `text` holds `form`, word by word in a row. */
export function hasForm(text: string, form: string): boolean {
    const forms = words(form.replace(/["«»„“”]/g, " ")).filter((w) => w !== "ооо");
    if (forms.length === 0) return false;
    const stems = forms.map(stem);
    const tw = words(text);
    for (let i = 0; i + forms.length <= tw.length; i++) if (forms.every((f, k) => stemMatch(tw[i + k], stems[k]) || inflects(tw[i + k], f))) return true;
    return false;
}

/** Whether `text` holds any of a term's forms (` / ` variants, the plural; a bracket is not looked for). */
export function termFound(text: string, term: Pick<LineTerm, "ru" | "ruPlural">): boolean {
    return [...term.ru.split(" / "), ...(term.ruPlural ? [term.ruPlural] : [])].some((f) => hasForm(text, f.replace(/\s*\(.*\)\s*$/, "")));
}

/** The line's glossary terms (not Sova's sheet) whose form `text` lacks, one per English found. */
export function missingTerms(text: string, terms: readonly LineTerm[]): { en: string; ru: string }[] {
    const byEn = new Map<string, LineTerm[]>();
    for (const term of terms) if (!term.fromSova) byEn.set(term.matched, [...(byEn.get(term.matched) ?? []), term]);
    return [...byEn.entries()].filter(([, ts]) => !ts.some((term) => termFound(text, term))).map(([en, ts]) => ({ en, ru: [...new Set(ts.map((term) => term.ru))].join(" / ") }));
}
