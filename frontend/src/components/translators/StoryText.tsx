/**
 * A text of the game in the translators' section, shown the way the game
 * shows it, never with its tags spelled out: a game style in its ink, a term
 * underlined with the game's explanation on hover, italics, bold, colour, a
 * story's paragraphs; `{@nickname}` as a chip with the signed-in member's
 * nickname (the saved text keeps the token as it is). The fragments quoted
 * by comments and what the search found are highlighted.
 */
import type React from "react";
import { Badge } from "#/components/ui/badge";
import { useLocale, useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { type MarkNode, parseMarkup } from "./markup";
import { type GameTags, spanLook, tagWords, termHint, useGameTags } from "./tags";
import type { messages } from "./translators.messages";

type T = TypedT<typeof messages>;

/** A private-use character standing for `{@nickname}` while the markup is parsed. */
const NICK = String.fromCharCode(0xe000);
const NICK_TOKEN = /\{@nickname\}/gi;

/** The text with the story's substitutions, the nickname kept as a marker. */
function prepare(text: string): string {
    return text
        .replace(NICK_TOKEN, NICK)
        .replace(/\{@nbs\}/g, String.fromCharCode(0xa0))
        .replace(/\\n/g, "\n");
}

export function NicknameChip({ nickname }: { nickname: string }): React.ReactElement {
    return (
        <Badge variant="tint" size="sm" className="mx-0.5 align-baseline" data-token="{@nickname}">
            {nickname}
        </Badge>
    );
}

/** What the search looks for: any case, ё = е, whole words only when `whole`. */
export interface IFind {
    q: string;
    whole: boolean;
}

function foldText(s: string): string {
    return s.toLowerCase().replaceAll("ё", "е");
}

const WORD = /[\p{L}\p{N}_]/u;

/** Where the search's text first stands in a run, as [start, end); null when it does not. */
function findIn(value: string, find: IFind | null | undefined): [number, number] | null {
    const q = find ? foldText(find.q.trim()) : "";
    if (!q || !find) return null;
    const folded = foldText(value);
    // Lower case of a few letters changes the length; then the positions would lie.
    if (folded.length !== value.length) return null;
    for (let at = folded.indexOf(q); at >= 0; at = folded.indexOf(q, at + 1)) {
        const end = at + q.length;
        if (!find.whole || (!WORD.test(value[at - 1] ?? "") && !WORD.test(value[end] ?? ""))) return [at, end];
    }
    return null;
}

const MARK = "rounded-sm bg-warning/20 text-foreground";

/** Split a plain run by the quotes and the search's text (each marked) and the nickname marker (a chip). */
function renderRun(value: string, nickname: string, quotes: readonly string[], key: string, find?: IFind | null): React.ReactNode[] {
    const out: React.ReactNode[] = [];
    const found = findIn(value, find);
    if (found) {
        const [at, end] = found;
        out.push(...renderRun(value.slice(0, at), nickname, quotes, `${key}c`, find));
        out.push(
            <mark key={`${key}f`} className={MARK}>
                {renderRun(value.slice(at, end), nickname, [], `${key}g`)}
            </mark>,
        );
        out.push(...renderRun(value.slice(end), nickname, quotes, `${key}d`, find));
        return out;
    }
    const quote = quotes.find((q) => q && value.includes(q));
    if (quote) {
        const at = value.indexOf(quote);
        out.push(...renderRun(value.slice(0, at), nickname, quotes, `${key}a`));
        out.push(
            <mark key={`${key}m`} className={MARK}>
                {renderRun(quote, nickname, [], `${key}q`)}
            </mark>,
        );
        out.push(...renderRun(value.slice(at + quote.length), nickname, quotes, `${key}b`));
        return out;
    }
    // Keyed by where each chip stands in the run.
    let at = 0;
    for (const part of value.split(NICK)) {
        if (at > 0) out.push(<NicknameChip key={`${key}n${at}`} nickname={nickname} />);
        if (part) out.push(part);
        at += part.length + 1;
    }
    return out;
}

interface IRender {
    nickname: string;
    quotes: readonly string[];
    find?: IFind | null;
    tags: GameTags;
    locale: string;
    t: T;
}

function renderNodes(nodes: readonly MarkNode[], r: IRender, prefix: string): React.ReactNode[] {
    return nodes.map((n, i) => {
        const key = `${prefix}${i}`;
        if (n.kind === "text") return <span key={key}>{renderRun(n.value, r.nickname, r.quotes, key, r.find)}</span>;
        // A substitution the game fills in (a number of a skill's level): as written.
        if (n.kind === "token") return <span key={key}>{n.raw}</span>;
        const children = renderNodes(n.children, r, `${key}.`);
        if (n.type === "paragraph")
            return (
                <span key={key} className="block">
                    {children}
                </span>
            );
        const look = spanLook(n);
        // A term carries what the game explains about it; any other span says what it is.
        const title = n.type === "term" ? (termHint(n.name, r.tags, r.locale) ?? tagWords(n, r.tags, r.t, r.locale)) : n.type === "style" ? tagWords(n, r.tags, r.t, r.locale) : undefined;
        return (
            <span key={key} className={look.className} style={look.color ? { color: look.color } : undefined} title={title} data-tag={n.name}>
                {children}
            </span>
        );
    });
}

export interface IStoryTextProps {
    text: string;
    /** Shown for `{@nickname}`. */
    nickname: string;
    /** Fragments to highlight (comments' quotes). */
    quotes?: readonly string[];
    /** The search's text, highlighted where it stands. */
    find?: IFind | null;
}

export function StoryText({ text, nickname, quotes = [], find = null }: IStoryTextProps): React.ReactElement {
    const t: T = useT("translators");
    const tags = useGameTags();
    const locale = useLocale();
    // A quote is the selected text, where the chip read as the nickname.
    const marks = quotes.map((q) => q.replaceAll(nickname, NICK));
    return <span className="whitespace-pre-line">{renderNodes(parseMarkup(prepare(text)), { nickname, quotes: marks, find, tags, locale, t }, "t")}</span>;
}
