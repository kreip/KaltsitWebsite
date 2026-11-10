/**
 * Two versions of a translation compared by words (diff.ts): the new text
 * with what it added in the success green and what it dropped, where it
 * stood, struck through in the destructive red. The texts are shown as
 * written (tags and `{@nickname}` as they are), so a changed tag is seen too.
 */
import type React from "react";
import { useMemo } from "react";
import { diffWords } from "./diff";

/** Line breaks written as `\n` read as line breaks, the way the story text shows them. */
function prepare(text: string): string {
    return text.replace(/\\n/g, "\n");
}

export interface ITextDiffProps {
    /** The text one step back. */
    before: string;
    after: string;
    /** Screen-reader words for an added and a dropped fragment. */
    labels: { added: string; removed: string };
}

export function TextDiff({ before, after, labels }: ITextDiffProps): React.ReactElement {
    const parts = useMemo(() => {
        // Keyed by where each part stands in the comparison.
        let at = 0;
        return diffWords(prepare(before), prepare(after)).map((p) => {
            const key = `${p.kind}${at}`;
            at += p.text.length;
            return { ...p, key };
        });
    }, [before, after]);
    return (
        <span className="whitespace-pre-line">
            {parts.map((p) =>
                p.kind === "same" ? (
                    <span key={p.key}>{p.text}</span>
                ) : p.kind === "add" ? (
                    <ins key={p.key} className="rounded-sm bg-success/8 text-success-foreground no-underline">
                        <span className="sr-only">{labels.added} </span>
                        {p.text}
                    </ins>
                ) : (
                    <del key={p.key} className="rounded-sm bg-destructive/8 text-destructive-foreground line-through">
                        <span className="sr-only">{labels.removed} </span>
                        {p.text}
                    </del>
                ),
            )}
        </span>
    );
}
