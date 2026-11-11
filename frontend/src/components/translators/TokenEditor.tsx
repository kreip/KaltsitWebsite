/**
 * The proposal editor's text box: plain text where every substitution token
 * (`{@nickname}`, `{0}`, `{atk:0%}`) is one chip that cannot be split or typed
 * into: it is removed whole (Backspace, Delete, or its cross on hover) and
 * inserted again with its button under the box — a button per substitution
 * of the line's English, so nothing the original lacks can be put in. The
 * value is the text with the tokens as they are. Ctrl+Enter submits, Esc
 * cancels. It looks like the kit's field (border, focus ring).
 */
import { AtSign, Braces } from "lucide-react";
import type React from "react";
import { useEffect, useRef } from "react";
import { badgeVariants } from "#/components/ui/badge";
import { Button } from "#/components/ui/button";
import { cn } from "#/lib/utils";

/** A substitution: `{@nickname}`, `{0}`, `{atk:0%}`, `{-cost}`. */
const TOKEN = /\{[^{}\n]{1,60}\}/g;

const NICKNAME = "{@nickname}";

function isNickname(token: string): boolean {
    return token.toLowerCase() === NICKNAME;
}

/** The substitutions a text holds, each once, in order: what the editor's buttons can insert. */
export function paletteOf(text: string): string[] {
    const out: string[] = [];
    for (const token of text.match(TOKEN) ?? []) {
        const known = isNickname(token) ? NICKNAME : token;
        if (!out.includes(known)) out.push(known);
    }
    return out;
}

function chip(token: string, nickname: string, removeLabel: string): HTMLElement {
    const el = document.createElement("span");
    el.contentEditable = "false";
    el.dataset.token = token;
    el.className = cn(badgeVariants({ variant: "tint", size: "sm" }), "group/chip mx-0.5 select-none align-baseline");
    el.append(isNickname(token) ? nickname : token);
    // The cross: shown on hover (always on a touch screen); the editor's click handler removes the chip.
    const cross = document.createElement("button");
    cross.type = "button";
    cross.tabIndex = -1;
    cross.dataset.remove = "";
    cross.setAttribute("aria-label", removeLabel);
    cross.title = removeLabel;
    cross.className = "-me-0.5 ms-0.5 hidden cursor-pointer rounded-sm px-0.5 leading-none hover:bg-foreground/10 pointer-coarse:inline group-hover/chip:inline";
    cross.textContent = "×";
    el.append(cross);
    return el;
}

function fill(el: HTMLElement, text: string, nickname: string, removeLabel: string): void {
    const nodes: Node[] = [];
    let last = 0;
    for (const m of text.matchAll(TOKEN)) {
        const at = m.index ?? 0;
        if (at > last) nodes.push(document.createTextNode(text.slice(last, at)));
        nodes.push(chip(m[0], nickname, removeLabel));
        last = at + m[0].length;
    }
    if (last < text.length) nodes.push(document.createTextNode(text.slice(last)));
    el.replaceChildren(...nodes);
}

/** The text back from the box: chips as their tokens, line breaks as LF. */
function serialize(node: Node): string {
    let out = "";
    node.childNodes.forEach((child) => {
        if (child.nodeType === Node.TEXT_NODE) out += child.textContent ?? "";
        else if (child instanceof HTMLElement) {
            if (child.dataset.token) out += child.dataset.token;
            else if (child.tagName === "BR") out += "\n";
            else {
                // A browser may wrap a new line in a block.
                if ((child.tagName === "DIV" || child.tagName === "P") && out && !out.endsWith("\n")) out += "\n";
                out += serialize(child);
            }
        }
    });
    return out;
}

/** The chip right before (`back`) or after the caret, skipping empty text nodes; null when the caret is in a text. */
function chipAtCaret(el: HTMLElement, back: boolean): HTMLElement | null {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0 || !sel.isCollapsed) return null;
    const { startContainer: node, startOffset: offset } = sel.getRangeAt(0);
    if (!el.contains(node)) return null;
    let next: Node | null;
    if (node.nodeType === Node.TEXT_NODE) {
        const length = node.textContent?.length ?? 0;
        if (back ? offset > 0 : offset < length) return null;
        next = back ? node.previousSibling : node.nextSibling;
    } else {
        next = node.childNodes[back ? offset - 1 : offset] ?? null;
    }
    while (next && next.nodeType === Node.TEXT_NODE && !next.textContent) next = back ? next.previousSibling : next.nextSibling;
    return next instanceof HTMLElement && next.dataset.token ? next : null;
}

/** The editor's words. */
export interface ITokenLabels {
    /** Name of the "insert nickname" button. */
    insertNickname: string;
    /** Tooltip of another substitution's button, given the token. */
    insertToken: (token: string) => string;
    /** Accessible name of a chip's cross. */
    remove: string;
}

export interface ITokenEditorProps {
    id: string;
    initial: string;
    nickname: string;
    /** The substitutions that can be inserted (`paletteOf` the line's English): a button each. */
    palette: readonly string[];
    onChange: (text: string) => void;
    onSubmit: () => void;
    onCancel: () => void;
    labels: ITokenLabels;
    describedBy?: string;
}

export function TokenEditor({ id, initial, nickname, palette, onChange, onSubmit, onCancel, labels, describedBy }: ITokenEditorProps): React.ReactElement {
    const ref = useRef<HTMLDivElement>(null);
    // Filled once: React does not own the box's content, so typing never jumps the caret.
    // biome-ignore lint/correctness/useExhaustiveDependencies: the initial text only
    useEffect(() => {
        const el = ref.current;
        if (!el) return;
        fill(el, initial, nickname, labels.remove);
        el.focus();
        const range = document.createRange();
        range.selectNodeContents(el);
        range.collapse(false);
        window.getSelection()?.removeAllRanges();
        window.getSelection()?.addRange(range);
    }, []);

    const changed = () => {
        if (ref.current) onChange(serialize(ref.current));
    };

    const insert = (token: string) => {
        const el = ref.current;
        if (!el) return;
        const sel = window.getSelection();
        let range = sel && sel.rangeCount > 0 ? sel.getRangeAt(0) : null;
        if (!range || !el.contains(range.startContainer)) {
            range = document.createRange();
            range.selectNodeContents(el);
            range.collapse(false);
        }
        const node = chip(token, nickname, labels.remove);
        range.deleteContents();
        range.insertNode(node);
        range.setStartAfter(node);
        range.collapse(true);
        sel?.removeAllRanges();
        sel?.addRange(range);
        el.focus();
        changed();
    };

    const remove = (node: HTMLElement) => {
        const el = ref.current;
        if (!el) return;
        const range = document.createRange();
        range.setStartBefore(node);
        range.collapse(true);
        node.remove();
        el.normalize();
        window.getSelection()?.removeAllRanges();
        window.getSelection()?.addRange(range);
        el.focus();
        changed();
    };

    return (
        <div className="flex flex-col gap-1">
            {/* biome-ignore lint/a11y/useSemanticElements: a textarea cannot hold the token chips */}
            <div
                ref={ref}
                id={id}
                role="textbox"
                aria-multiline="true"
                aria-describedby={describedBy}
                tabIndex={0}
                contentEditable="plaintext-only"
                suppressContentEditableWarning
                onInput={changed}
                onClick={(e) => {
                    // A chip's cross.
                    const cross = e.target instanceof HTMLElement ? e.target.closest("[data-remove]") : null;
                    const owner = cross?.closest("[data-token]");
                    if (owner instanceof HTMLElement) remove(owner);
                }}
                onPaste={(e) => {
                    // Plain text only, also where `plaintext-only` is not supported.
                    e.preventDefault();
                    document.execCommand("insertText", false, e.clipboardData.getData("text/plain"));
                }}
                onKeyDown={(e) => {
                    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
                        e.preventDefault();
                        onSubmit();
                    } else if (e.key === "Escape") {
                        e.preventDefault();
                        onCancel();
                    } else if ((e.key === "Backspace" || e.key === "Delete") && ref.current) {
                        // A chip is not text: the browser may refuse to step over it.
                        const node = chipAtCaret(ref.current, e.key === "Backspace");
                        if (node) {
                            e.preventDefault();
                            remove(node);
                        }
                    }
                }}
                className="max-h-96 min-h-16 w-full overflow-y-auto whitespace-pre-wrap rounded-lg border border-input bg-background px-3 py-1.5 text-base shadow-xs/5 outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring sm:text-sm dark:bg-input/32"
            />
            {palette.length > 0 ? (
                <div className="flex flex-wrap gap-1">
                    {palette.map((token) => (
                        <Button key={token} variant="ghost" size="xs" onClick={() => insert(token)} onMouseDown={(e) => e.preventDefault()} title={isNickname(token) ? undefined : labels.insertToken(token)}>
                            {isNickname(token) ? <AtSign aria-hidden="true" /> : <Braces aria-hidden="true" />}
                            {isNickname(token) ? labels.insertNickname : token}
                        </Button>
                    ))}
                </div>
            ) : null}
        </div>
    );
}
