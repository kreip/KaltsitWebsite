/**
 * Two versions of a translation compared by words (diff.ts): the new text
 * with what it added in the success green and what it dropped, where it
 * stood, struck through in the destructive red; a word changed by a few
 * letters shows those letters. The game's tags (`<i>`, `<@ba.vup>`, `</>`…)
 * are never shown as letters: they colour the text as the tables do (a
 * dropped text's tags change nothing), a substitution is a chip, and a
 * change of the tags alone says so.
 */
import type React from "react";
import { useMemo } from "react";
import { diffWords } from "./diff";
import { NicknameChip, TokenChip } from "./StoryText";
import { spanLook, tagOf } from "./tags";

/** The game's and Unity's tags: left out of the comparison. */
const TAG = /<\/?(?:[@$][^<>\n]*|i|b|u|color(?:=[^<>\n]*)?|size(?:=[^<>\n]*)?|p(?:=[^<>\n]*)?)?>/gi;

/** Line breaks written as `\n` read as line breaks, the way the story text shows them; no tags. */
function prepare(text: string): string {
    return text.replace(/\\n/g, "\n").replace(TAG, "");
}

/** The same with the tags kept: they colour the text as the tables do, never as letters. */
function prepareTagged(text: string): string {
    return text.replace(/\\n/g, "\n");
}

/** One whole tag (TAG is global: its own state must not leak between tests). */
const IS_TAG = new RegExp(`^(?:${TAG.source})$`, "i");

/** A part's text as tags, substitutions and plain runs. */
const PIECE = /(<[^<>\n]*>|\{[^{}\n]*\})/;

interface IStyle {
    color?: string;
    className?: string;
}

/**
 * A part's text with the game's look: a tag of the new text (kept or added) opens or closes a style for what
 * follows (a dropped text's tags change nothing), a substitution is a chip, a nickname its chip.
 */
function renderPart(text: string, kind: "same" | "add" | "del", stack: IStyle[], nickname: string, key: string): React.ReactNode[] {
    const out: React.ReactNode[] = [];
    // Keyed by where each piece starts in the part.
    let at = 0;
    for (const piece of text.split(PIECE)) {
        const i = at;
        at += piece.length;
        if (!piece) continue;
        if (IS_TAG.test(piece)) {
            if (kind === "del") continue;
            if (piece.startsWith("</")) stack.pop();
            else {
                const look = spanLook(tagOf(piece));
                stack.push({ color: look.color, className: look.className });
            }
            continue;
        }
        if (/^\{[^{}\n]*\}$/.test(piece)) {
            out.push(nickname && /^\{@nickname\}$/i.test(piece) ? <NicknameChip key={`${key}.${i}`} nickname={nickname} /> : <TokenChip key={`${key}.${i}`} raw={piece} />);
            continue;
        }
        const color = [...stack].reverse().find((st) => st.color)?.color;
        const className = stack
            .map((st) => st.className)
            .filter(Boolean)
            .join(" ");
        out.push(
            color || className ? (
                <span key={`${key}.${i}`} className={className || undefined} style={color ? { color } : undefined}>
                    {piece}
                </span>
            ) : (
                piece
            ),
        );
    }
    return out;
}

export interface ITextDiffProps {
    /** The text one step back. */
    before: string;
    after: string;
    /** Screen-reader words for an added and a dropped fragment; the note for a change of the tags alone. */
    labels: { added: string; removed: string; tags?: string };
    /** Shown for `{@nickname}`. */
    nickname?: string;
}

/** The parts in order, the style of the tags carried from one part to the next. */
function renderParts(parts: readonly { kind: "same" | "add" | "del"; text: string; key: string }[], nickname: string, labels: ITextDiffProps["labels"]): React.ReactNode[] {
    const stack: IStyle[] = [];
    return parts.map((p) => {
        const content = renderPart(p.text, p.kind, stack, nickname, p.key);
        if (p.kind === "same") return <span key={p.key}>{content}</span>;
        // A part of tags only (a tag added or dropped) shows nothing.
        if (content.length === 0) return null;
        return p.kind === "add" ? (
            <ins key={p.key} className="rounded-sm bg-success/8 text-success-foreground no-underline">
                <span className="sr-only">{labels.added} </span>
                {content}
            </ins>
        ) : (
            <del key={p.key} className="rounded-sm bg-destructive/8 text-destructive-foreground line-through">
                <span className="sr-only">{labels.removed} </span>
                {content}
            </del>
        );
    });
}

export function TextDiff({ before, after, labels, nickname = "" }: ITextDiffProps): React.ReactElement {
    const parts = useMemo(() => {
        // Keyed by where each part stands in the comparison.
        let at = 0;
        return diffWords(prepareTagged(before), prepareTagged(after)).map((p) => {
            const key = `${p.kind}${at}`;
            at += p.text.length;
            return { ...p, key };
        });
    }, [before, after]);
    const tagsOnly = before !== after && prepare(before) === prepare(after);
    return (
        <span className="whitespace-pre-line">
            {tagsOnly && labels.tags ? <span className="me-1 text-subtle-foreground text-xs">({labels.tags})</span> : null}
            {renderParts(parts, nickname, labels)}
        </span>
    );
}
