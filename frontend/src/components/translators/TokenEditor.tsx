/**
 * The proposal editor's text box: the text as the game will show it, never
 * with its markup spelled out.
 *
 * A substitution (`{@nickname}`, `{0}`, `{atk:0%}`) is one chip that cannot
 * be split or typed into: it is removed whole (Backspace, Delete, or its
 * cross on hover) and inserted again with its button under the box.
 *
 * A tag with an opener and a closer (`<@ba.vup>…</>`, `<i>…</i>`) is a span
 * that looks as in the game and is typed into like any text: select a word
 * and press the tag's button to wrap it; put the cursor inside a span and
 * "Remove the tag" unwraps it — the text stays, both halves of the tag go
 * together, so a tag is never left half open.
 *
 * The buttons are the substitutions and the tags of the line's English, so
 * nothing the original lacks can be put in. The value is the text with its
 * tokens and tags as the game writes them. Ctrl+Enter submits, Esc cancels.
 * It looks like the kit's field (border, focus ring).
 */
import { AtSign, Braces, Eraser } from "lucide-react";
import type React from "react";
import { useEffect, useRef, useState } from "react";
import { badgeVariants } from "#/components/ui/badge";
import { Button } from "#/components/ui/button";
import { cn } from "#/lib/utils";
import { type MarkNode, parseMarkup } from "./markup";
import type { ISpanLook } from "./tags";

const NICKNAME = "{@nickname}";

function isNickname(token: string): boolean {
    return token.toLowerCase() === NICKNAME;
}

/** Stands beside a chip or a span at the edge of the box so the cursor can be put outside it; never part of the value. */
const GUARD = "​";

/** A tag the editor can wrap a selection in. */
export interface ITagButton {
    /** The opener and the closer as the game writes them. */
    open: string;
    close: string;
    /** In words («value raised», «Stun»). */
    label: string;
    look: ISpanLook;
}

/** What the box needs to know about a span it draws: its look and its name in words. */
export type TagLookup = (open: string) => { look: ISpanLook; label: string };

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

/** A wrapped span: typed into, shown as in the game, on a faint ground so its edges are seen. */
function tagSpan(open: string, close: string, lookup: TagLookup): HTMLElement {
    const el = document.createElement("span");
    const { look, label } = lookup(open);
    el.dataset.open = open;
    el.dataset.close = close;
    el.title = label;
    el.className = cn("rounded-sm bg-foreground/5", look.className);
    if (look.color) el.style.color = look.color;
    return el;
}

interface IBuild {
    nickname: string;
    removeLabel: string;
    lookup: TagLookup;
}

function build(nodes: readonly MarkNode[], b: IBuild): Node[] {
    return nodes.map((n) => {
        if (n.kind === "text") return document.createTextNode(n.value);
        if (n.kind === "token") return chip(n.raw, b.nickname, b.removeLabel);
        const el = tagSpan(n.open, n.close, b.lookup);
        el.append(...build(n.children, b));
        return el;
    });
}

function isBoxed(node: Node | null): node is HTMLElement {
    return node instanceof HTMLElement && (node.dataset.token !== undefined || node.dataset.open !== undefined);
}

/** Keep a place for the cursor before a chip or span that starts the box and after one that ends it. */
function guard(el: HTMLElement): void {
    if (isBoxed(el.firstChild)) el.prepend(document.createTextNode(GUARD));
    if (isBoxed(el.lastChild)) el.append(document.createTextNode(GUARD));
}

/** The text back from the box: chips as their tokens, spans inside their tags, line breaks as LF. */
function serialize(node: Node): string {
    let out = "";
    node.childNodes.forEach((child) => {
        if (child.nodeType === Node.TEXT_NODE) out += (child.textContent ?? "").replaceAll(GUARD, "");
        else if (child instanceof HTMLElement) {
            if (child.dataset.token) out += child.dataset.token;
            else if (child.dataset.open !== undefined) {
                // A span emptied by deleting its text leaves no tag behind.
                const inner = serialize(child);
                if (inner) out += child.dataset.open + inner + (child.dataset.close ?? "");
            } else if (child.tagName === "BR") out += "\n";
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
        const text = node.textContent ?? "";
        // The guard beside a chip does not count as text.
        const before = text.slice(0, offset).replaceAll(GUARD, "");
        const after = text.slice(offset).replaceAll(GUARD, "");
        if (back ? before.length > 0 : after.length > 0) return null;
        next = back ? node.previousSibling : node.nextSibling;
    } else {
        next = node.childNodes[back ? offset - 1 : offset] ?? null;
    }
    while (next && next.nodeType === Node.TEXT_NODE && !(next.textContent ?? "").replaceAll(GUARD, "")) next = back ? next.previousSibling : next.nextSibling;
    return next instanceof HTMLElement && next.dataset.token ? next : null;
}

/** The wrapped span the caret (or the selection's start) is in. */
function spanAtCaret(el: HTMLElement): HTMLElement | null {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return null;
    const node = sel.anchorNode;
    if (!node || !el.contains(node)) return null;
    const from = node instanceof Element ? node : node.parentElement;
    const span = from?.closest("[data-open]");
    return span instanceof HTMLElement && el.contains(span) ? span : null;
}

/** The editor's words. */
export interface ITokenLabels {
    /** Name of the "insert nickname" button. */
    insertNickname: string;
    /** Tooltip of another substitution's button, given the token. */
    insertToken: (token: string) => string;
    /** Accessible name of a chip's cross. */
    remove: string;
    /** Tooltip of a tag's button: wraps the selection, given the tag in words. */
    wrap: (tag: string) => string;
    /** The button that unwraps the span the cursor is in, given the tag in words. */
    unwrap: (tag: string) => string;
    /** Shown beside the tags' buttons: how to use them. */
    wrapHint: string;
}

export interface ITokenEditorProps {
    id: string;
    initial: string;
    nickname: string;
    /** The substitutions that can be inserted (the line's English's): a button each. */
    palette: readonly string[];
    /** The tags a selection can be wrapped in (the line's English's): a button each. */
    tags: readonly ITagButton[];
    /** The look and the name of any tag the text may already hold. */
    lookup: TagLookup;
    onChange: (text: string) => void;
    onSubmit: () => void;
    onCancel: () => void;
    labels: ITokenLabels;
    describedBy?: string;
}

export function TokenEditor({ id, initial, nickname, palette, tags, lookup, onChange, onSubmit, onCancel, labels, describedBy }: ITokenEditorProps): React.ReactElement {
    const ref = useRef<HTMLDivElement>(null);
    // The wrapped span the cursor is in: "Remove the tag" is offered for it.
    const [inside, setInside] = useState<HTMLElement | null>(null);
    // Filled once: React does not own the box's content, so typing never jumps the caret.
    // biome-ignore lint/correctness/useExhaustiveDependencies: the initial text only
    useEffect(() => {
        const el = ref.current;
        if (!el) return;
        el.replaceChildren(...build(parseMarkup(initial), { nickname, removeLabel: labels.remove, lookup }));
        guard(el);
        el.focus();
        const range = document.createRange();
        range.selectNodeContents(el);
        range.collapse(false);
        window.getSelection()?.removeAllRanges();
        window.getSelection()?.addRange(range);
    }, []);

    // Where the cursor stands, to offer "Remove the tag".
    useEffect(() => {
        const track = () => {
            const el = ref.current;
            setInside(el ? spanAtCaret(el) : null);
        };
        document.addEventListener("selectionchange", track);
        return () => document.removeEventListener("selectionchange", track);
    }, []);

    const changed = () => {
        if (ref.current) onChange(serialize(ref.current));
    };

    /** The selection inside the box, else the end of the box. */
    const rangeIn = (el: HTMLElement): Range => {
        const sel = window.getSelection();
        const range = sel && sel.rangeCount > 0 ? sel.getRangeAt(0) : null;
        if (range && el.contains(range.startContainer) && el.contains(range.endContainer)) return range;
        const end = document.createRange();
        end.selectNodeContents(el);
        end.collapse(false);
        return end;
    };

    const caretAfter = (el: HTMLElement, node: Node) => {
        guard(el);
        const range = document.createRange();
        range.setStartAfter(node);
        range.collapse(true);
        window.getSelection()?.removeAllRanges();
        window.getSelection()?.addRange(range);
        el.focus();
        changed();
    };

    const insert = (token: string) => {
        const el = ref.current;
        if (!el) return;
        const range = rangeIn(el);
        const node = chip(token, nickname, labels.remove);
        range.deleteContents();
        range.insertNode(node);
        caretAfter(el, node);
    };

    const wrap = (tag: ITagButton) => {
        const el = ref.current;
        if (!el) return;
        const range = rangeIn(el);
        // Nothing selected: there is nothing to wrap (the hint beside the buttons says so).
        if (range.collapsed) return;
        const span = tagSpan(tag.open, tag.close, lookup);
        span.append(range.extractContents());
        range.insertNode(span);
        el.normalize();
        caretAfter(el, span);
    };

    const unwrap = (span: HTMLElement) => {
        const el = ref.current;
        if (!el) return;
        const last = span.lastChild;
        span.replaceWith(...span.childNodes);
        setInside(null);
        if (last) caretAfter(el, last);
        else changed();
        el.normalize();
    };

    const remove = (node: HTMLElement) => {
        const el = ref.current;
        if (!el) return;
        const range = document.createRange();
        range.setStartBefore(node);
        range.collapse(true);
        node.remove();
        el.normalize();
        guard(el);
        window.getSelection()?.removeAllRanges();
        window.getSelection()?.addRange(range);
        el.focus();
        changed();
    };

    return (
        <div className="flex flex-col gap-1">
            {/* biome-ignore lint/a11y/useSemanticElements: a textarea cannot hold the token chips and the tag spans */}
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
            {palette.length + tags.length > 0 || inside ? (
                // The buttons keep the box's selection: a press must not move the cursor out of it.
                // biome-ignore lint/a11y/noStaticElementInteractions: only keeps the selection for its buttons
                <div className="flex flex-wrap items-center gap-1" onMouseDown={(e) => e.preventDefault()}>
                    {palette.map((token) => (
                        <Button key={token} variant="ghost" size="xs" onClick={() => insert(token)} title={isNickname(token) ? undefined : labels.insertToken(token)}>
                            {isNickname(token) ? <AtSign aria-hidden="true" /> : <Braces aria-hidden="true" />}
                            {isNickname(token) ? labels.insertNickname : token}
                        </Button>
                    ))}
                    {tags.map((tag) => (
                        <Button key={tag.open} variant="outline" size="xs" onClick={() => wrap(tag)} title={labels.wrap(tag.label)}>
                            <span className={tag.look.className} style={tag.look.color ? { color: tag.look.color } : undefined}>
                                {tag.label}
                            </span>
                        </Button>
                    ))}
                    {inside ? (
                        <Button variant="ghost" size="xs" onClick={() => unwrap(inside)}>
                            <Eraser aria-hidden="true" />
                            {labels.unwrap(lookup(inside.dataset.open ?? "").label)}
                        </Button>
                    ) : null}
                    {tags.length > 0 && !inside ? <span className="text-subtle-foreground text-xs">{labels.wrapHint}</span> : null}
                </div>
            ) : null}
        </div>
    );
}
