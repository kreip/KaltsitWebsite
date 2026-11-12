/**
 * The game's tags in words and in looks, for the tables and the editor: a
 * term by the name the game shows for it («Stun»), a style by what it marks
 * («value raised», «keyword»), Unity's spans by what they do, a substitution
 * by what it stands for. The terms' names and explanations and the styles'
 * colours come from the import (`GET /translators/tags`).
 */
import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { inkForTag } from "#/lib/gamedata/richtext";
import type { TypedT } from "#/lib/i18n/messages";
import { gameTagsQueryOptions } from "#/lib/translators/api";
import type { GameTag } from "#/types/generated/GameTag";
import { parseMarkup, type SpanType } from "./markup";
import type { messages } from "./translators.messages";

type T = TypedT<typeof messages>;

export type GameTags = ReadonlyMap<string, GameTag>;

const NONE: GameTags = new Map();

/** The game's tags by name (`@ba.vup`, `$ba.stun`); empty until loaded. */
export function useGameTags(): GameTags {
    const query = useQuery(gameTagsQueryOptions());
    return useMemo(() => (query.data ? new Map(query.data.map((tag) => [tag.tag, tag])) : NONE), [query.data]);
}

/** What a style marks, by the last part of its name (`ba.vup`, `cc.vup` -> `vup`). */
const STYLES: Record<string, string> = {
    vup: "vup",
    vdown: "vdown",
    rem: "rem",
    acrem: "rem",
    kw: "kw",
    key: "kw",
    imp: "kw",
    talpu: "talpu",
    item: "item",
    muitem: "item",
    fs: "condition",
    ez: "condition",
    get: "get",
    lose: "lose",
};

interface INamed {
    type: SpanType;
    name: string;
    open: string;
}

/** A tag in words: «Stun», «value raised», «italic», «colour #FF0000». */
export function tagWords(tag: INamed, tags: GameTags, t: T, locale: string): string {
    if (tag.type === "term") {
        const info = tags.get(tag.name);
        const name = (locale === "ru" ? info?.nameRu : null) ?? info?.nameEn;
        return name ? t("tag.term", { name }) : t("tag.termCode", { code: tag.name.slice(1) });
    }
    if (tag.type === "style") {
        const known = STYLES[tag.name.slice(tag.name.lastIndexOf(".") + 1)];
        return known ? t(`tag.style.${known}` as "tag.style.vup") : t("tag.style.other", { code: tag.name.slice(1) });
    }
    if (tag.type === "inline") return t(`tag.inline.${tag.name === "b" ? "b" : tag.name === "u" ? "u" : "i"}` as "tag.inline.i");
    if (tag.type === "color") return t("tag.color", { color: colorOf(tag.open) ?? "" });
    if (tag.type === "size") return t("tag.size");
    return t("tag.paragraph");
}

/** What the game explains about a term, to show over its span: «Stun — Unable to act». */
export function termHint(name: string, tags: GameTags, locale: string): string | undefined {
    const info = tags.get(name);
    if (!info) return undefined;
    const title = (locale === "ru" ? info.nameRu : null) ?? info.nameEn;
    const about = (locale === "ru" ? info.descriptionRu : null) ?? info.descriptionEn;
    return [title, about].filter(Boolean).join(" — ") || undefined;
}

/** A substitution in words: «the Doctor's name», «value atk». */
export function tokenWords(token: string, t: T): string {
    if (token.toLowerCase() === "{@nickname}") return t("tag.token.nickname");
    return t("tag.token.value", { name: token.slice(1, -1) });
}

/** The colour a `<color=…>` opener names. */
export function colorOf(open: string): string | null {
    return /=\s*["']?([#A-Za-z0-9]+)/.exec(open)?.[1] ?? null;
}

/** How a span looks, in the tables and in the editor alike: the ink of a game style, a term's dotted underline, Unity's own. */
export interface ISpanLook {
    color?: string;
    /** Tailwind classes. */
    className?: string;
}

export function spanLook(tag: INamed): ISpanLook {
    if (tag.type === "style") return { color: inkForTag(tag.name) };
    if (tag.type === "term") return { color: inkForTag(tag.name), className: "underline decoration-dotted underline-offset-2" };
    if (tag.type === "color") return { color: colorOf(tag.open) ?? undefined };
    if (tag.type === "inline") return { className: tag.name === "b" ? "font-semibold" : tag.name === "u" ? "underline" : "italic" };
    return {};
}

/** An opener (`<@ba.vup>`, `<color=#fff>`) as the tag it starts. */
export function tagOf(open: string): INamed {
    const first = parseMarkup(open)[0];
    return first?.kind === "span" ? { type: first.type, name: first.name, open } : { type: "style", name: open, open };
}
