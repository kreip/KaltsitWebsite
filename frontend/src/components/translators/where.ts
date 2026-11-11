/**
 * Where a text stands in the game data, in words: «Operator › Amiya ›
 * Skill 2 › description», «Item › Orirock › how to get», «Interface and
 * other › Medals» (a table of the game data by what it holds, never by its
 * name). The field names are the import's (`texts/gamedata.rs`).
 */
import type { TypedT } from "#/lib/i18n/messages";
import type { messages } from "./translators.messages";

type T = TypedT<typeof messages>;

const PARTS = new Set(["talent", "skill", "module", "base", "skin", "file", "record", "voice", "zone"]);
const FIELDS = new Set(["name", "appellation", "class", "subclass", "tag", "trait", "description", "potential", "mission", "effect", "story", "group", "usage", "dialog", "title", "text", "ability", "obtain", "first", "second", "third", "locked"]);
const SECTIONS = new Set(["story", "operators", "enemies", "stages", "items", "archives", "other"]);

/** `skill.description` with 2 -> «Skill 2 › description»; `potential` with 3 -> «potential 3»; the `name` of an operator or an enemy -> «name», of anything else -> «title». */
export function fieldWords(field: string, idx: number, t: T, separator = " › ", section = ""): string {
    const n = idx > 0 ? ` ${idx}` : "";
    const leaf = (f: string) => (FIELDS.has(f) ? t(`at.field.${f}` as "at.field.name") : f);
    const dot = field.indexOf(".");
    // An operator and an enemy have a name; a stage, an item, a skill have a title.
    if (field === "name" && (section === "operators" || section === "enemies")) return t("at.field.who");
    if (dot < 0) return `${leaf(field)}${n}`;
    const part = field.slice(0, dot);
    const head = PARTS.has(part) ? t(`at.part.${part}` as "at.part.skill") : part;
    return `${head}${n}${separator}${leaf(field.slice(dot + 1))}`;
}

/** A field as a card's row names it: «Skill 2 · description», «Name». */
export function fieldLabel(section: string, field: string, idx: number, t: T): string {
    const words = fieldWords(field, idx, t, " · ", section);
    return words.charAt(0).toLocaleUpperCase() + words.slice(1);
}

/** The words of a note's codes that carry a number, and of those that stand alone. */
const NUMBERED: Record<string, string> = { E: "elite", L: "level", P: "potential", S: "skillLevel", M: "mastery", T: "moduleStage", N: "number", V: "voiceSet" };
const NAMED: Record<string, string> = { C: "challenge", DE: "easy", DN: "normal", DT: "tough" };

/** One code in words; several numbers of one kind as a range («skill level 1–7»). */
function codeWords(code: string, upTo: number | null, t: T): string {
    if (code.startsWith("#")) return code.slice(1);
    const named = NAMED[code];
    if (named) return t(`at.note.${named}` as "at.note.challenge");
    const kind = NUMBERED[code.charAt(0)];
    const n = code.slice(1);
    if (!kind || !/^\d+$/.test(n)) return code;
    return t(`at.note.${kind}` as "at.note.elite", { n: upTo === null ? n : `${n}–${upTo}` });
}

/**
 * What tells a text from the others at its field, in words (the backend's
 * `ObjectLine.note`): «Elite 2 · potential 6», «skill level 1–7, mastery
 * 1–3», «Challenge Mode». Variants are joined by `,`, the parts of one by `+`.
 */
export function noteWords(note: string, t: T): string {
    // `@…` names a story to read (`noteStories`), not a difference.
    const variants = note.split(",").filter((v) => v && !v.startsWith("@"));
    const out: string[] = [];
    for (let i = 0; i < variants.length; i++) {
        const v = variants[i];
        // A run of one kind's consecutive numbers reads as a range.
        const m = /^([A-Z])(\d+)$/.exec(v);
        if (m) {
            let last = Number(m[2]);
            let j = i;
            while (variants[j + 1] === `${m[1]}${last + 1}`) {
                j++;
                last++;
            }
            out.push(codeWords(v, j > i ? last : null, t));
            i = j;
            continue;
        }
        out.push(
            v
                .split("+")
                .map((code) => codeWords(code, null, t))
                .join(" · "),
        );
    }
    return out.join(", ");
}

/** The stories a note leads to (an operator record's introduction: the record's story in the Story tab). */
export function noteStories(note: string): string[] {
    return note
        .split(",")
        .filter((v) => v.startsWith("@"))
        .map((v) => v.slice(1));
}

/** The game sections whose texts stand at objects: a card per object on the Game sections tab. */
export const CARD_SECTIONS = ["operators", "enemies", "stages", "items", "archives"] as const;
export type CardSection = (typeof CARD_SECTIONS)[number];

export function isCardSection(section: string): section is CardSection {
    return (CARD_SECTIONS as readonly string[]).includes(section);
}

/**
 * What names an object in the address (the backend's `GameObject.id`): an
 * operator's id in the game data (for its card and for its archives); the
 * English name for the rest, where several records under one name are one card.
 */
export function objectKey(section: string, objectId: string | null, objectEn: string | null): string | null {
    return section === "operators" || section === "archives" ? objectId : objectEn;
}

export function sectionWord(section: string, t: T): string {
    return SECTIONS.has(section) ? t(`at.${section}` as "at.story") : section;
}

/** The game's tables behind «Interface and other», by what the game shows from them. */
const TABLES: Record<string, string> = {
    roguelike_topic_table: "rogue",
    roguelike_table: "rogue",
    activity_table: "events",
    building_data: "base",
    medal_table: "medals",
    sandbox_perm_table: "sandbox",
    sandbox_table: "sandbox",
    main_text: "ui",
    init_text: "ui",
    meta_ui_table: "ui",
    gamedata_const: "ui",
    stage_table: "stages",
    zone_table: "stages",
    chapter_table: "stages",
    handbook_info_table: "files",
    character_table: "operators",
    char_meta_table: "operators",
    char_patch_table: "operators",
    char_master_table: "operators",
    special_operator_table: "operators",
    token_table: "operators",
    charword_table: "voice",
    display_meta_table: "display",
    player_avatar_table: "display",
    story_review_meta_table: "review",
    story_review_table: "review",
    story_table: "review",
    retro_table: "review",
    shop_client_table: "shop",
    mission_table: "missions",
    open_server_table: "missions",
    checkin_table: "missions",
    gacha_table: "gacha",
    climb_tower_table: "tower",
    skin_table: "skins",
    charm_table: "charms",
    audio_data: "audio",
    crisis_v2_table: "crisis",
    crisis_table: "crisis",
    tip_table: "tips",
    uniequip_table: "modules",
    uniequip_data: "modules",
    battle_equip_table: "modules",
    enemy_handbook_table: "enemies",
    handbook_team_table: "teams",
    skill_table: "skills",
    clue_data: "clues",
    campaign_table: "campaign",
    item_table: "items",
};

/** A table of the game data in words («Medals»); its name when the section does not know it. */
export function tableWord(table: string, t: T): string {
    const known = TABLES[table];
    return known ? t(`at.table.${known}` as "at.table.ui") : table;
}

const FILES: Record<string, string> = { "11_ui.txt": "ui", "12_story.txt": "story", "13_names.txt": "names", "14_new.txt": "new", "_AutoGeneratedTranslations.txt": "auto" };

/** Where an entry tied to no place of the game data is kept, in words: a dictionary of the pack, or the plugin's catalog. */
export function fileWord(file: string, t: T): string {
    if (file.startsWith("plugin:")) return t("at.file.plugin");
    const known = FILES[file];
    return known ? t(`at.file.${known}` as "at.file.ui") : file;
}

/** A field of a game object (`object` is a table's name for `other`, shown in words; its field says nothing more). */
export function gameWhere(section: string, object: string | null, field: string | null, idx: number | null, t: T): string {
    const parts = [sectionWord(section, t)];
    if (object) parts.push(section === "other" ? tableWord(object, t) : object);
    if (field && !(section === "other" && field === "text")) parts.push(fieldWords(field, idx ?? 0, t, " › ", section));
    return parts.join(" › ");
}

/** The story id of a text outside the story (the backend's `TEXT_PLACE`): an edit there holds everywhere. */
export const TEXT_PLACE = "@text";

/** The English without its tags, as the search reads it. */
export function plainText(s: string): string {
    return s.replace(/<[^>]*>/g, "").trim();
}
