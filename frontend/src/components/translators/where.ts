/**
 * Where a text stands in the game data, in words: «Operator › Amiya ›
 * Skill 2 › description», «Item › Orirock › how to get», «Interface and
 * other › <table>». The field names are the import's (`texts/gamedata.rs`).
 */
import type { TypedT } from "#/lib/i18n/messages";
import type { messages } from "./translators.messages";

type T = TypedT<typeof messages>;

const PARTS = new Set(["talent", "skill", "module", "base", "skin", "file", "voice", "zone"]);
const FIELDS = new Set(["name", "appellation", "subclass", "tag", "trait", "description", "potential", "mission", "effect", "story", "group", "usage", "dialog", "title", "text", "ability", "obtain", "first", "second", "third", "locked"]);
const SECTIONS = new Set(["story", "operators", "enemies", "stages", "items", "archives", "other"]);

/** `skill.description` with 2 -> «Skill 2 › description»; `potential` with 3 -> «potential 3». */
export function fieldWords(field: string, idx: number, t: T): string {
    const n = idx > 0 ? ` ${idx}` : "";
    const leaf = (f: string) => (FIELDS.has(f) ? t(`at.field.${f}` as "at.field.name") : f);
    const dot = field.indexOf(".");
    if (dot < 0) return `${leaf(field)}${n}`;
    const part = field.slice(0, dot);
    const head = PARTS.has(part) ? t(`at.part.${part}` as "at.part.skill") : part;
    return `${head}${n} › ${leaf(field.slice(dot + 1))}`;
}

export function sectionWord(section: string, t: T): string {
    return SECTIONS.has(section) ? t(`at.${section}` as "at.story") : section;
}

/** A field of a game object (`object` is a table's name for `other`, whose field says nothing more). */
export function gameWhere(section: string, object: string | null, field: string | null, idx: number | null, t: T): string {
    const parts = [sectionWord(section, t)];
    if (object) parts.push(object);
    if (field && !(section === "other" && field === "text")) parts.push(fieldWords(field, idx ?? 0, t));
    return parts.join(" › ");
}

/** The story id of a text outside the story (the backend's `TEXT_PLACE`): an edit there holds everywhere. */
export const TEXT_PLACE = "@text";

/** The English without its tags, as the search reads it. */
export function plainText(s: string): string {
    return s.replace(/<[^>]*>/g, "").trim();
}
