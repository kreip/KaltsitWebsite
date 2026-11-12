/**
 * The translators whose variants the import brings beside the game's
 * Russian, each a source with its author's name. Sova is the first; another
 * translator is another entry here (and another column of the import). In
 * the interface a variant, a proposal started from it and a comment carry
 * the author's NAME — "translator" is the kind, never a person.
 */
import type { TypedT } from "#/lib/i18n/messages";
import type { messages } from "./translators.messages";

type T = TypedT<typeof messages>;

interface ITranslator {
    /** The author's name, by the section's language. */
    name: { ru: string; en: string };
}

/** By the source's key in the data (`TranslatorLine.sova`, a proposal's `source`, a vote's `variant`). */
export const TRANSLATORS = {
    sova: { name: { ru: "Сова", en: "Sova" } },
} as const satisfies Record<string, ITranslator>;

export type TranslatorSource = keyof typeof TRANSLATORS;

/** The translators' sources, in the order their variants show. */
export const TRANSLATOR_SOURCES = Object.keys(TRANSLATORS) as TranslatorSource[];

export function isTranslator(source: string): source is TranslatorSource {
    return source in TRANSLATORS;
}

/** A translator's name in the section's language. */
export function translatorName(source: TranslatorSource, locale: string): string {
    return TRANSLATORS[source].name[locale === "ru" ? "ru" : "en"];
}

/** "from the variant of …", for a proposal that started from a variant (a translator's, the machine's). */
export function fromWords(source: string, t: T, locale: string): string {
    return isTranslator(source) ? t("proposal.from.translator", { name: translatorName(source, locale) }) : t("proposal.from.ai");
}

/** The editor's label when it started from a variant. */
export function editorLabel(source: string, t: T, locale: string): string {
    if (source === "own") return t("editor.label");
    return isTranslator(source) ? t("editor.label.translator", { name: translatorName(source, locale) }) : t("editor.label.ai");
}
