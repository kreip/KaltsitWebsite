/**
 * A line of the tables is quiet until opened (Kolooo: "no need to see all
 * that until I click the line"): folded, it shows the text it stands with
 * (the approved one, else the game's), the English and what is wrong; a
 * click on the row (not on a button, a link, a box, not a selection, not a
 * double click that selects a word) opens it — the variants, the proposals,
 * the history and every button — and a second click or Esc folds it again.
 * The chevron in the № cell does the same from the keyboard. A player's
 * choices open and fold together with the line they answer (the story's
 * table). "Open every line" in the View menu (`open=1`) shows every row
 * opened, as before.
 */
import { ChevronDown, ChevronRight } from "lucide-react";
import type React from "react";
import { createContext, useCallback, useEffect, useRef, useState } from "react";
import { Button } from "#/components/ui/button";

/** "Open every line" (the View menu): every row shows everything, the buttons on hover as before. */
export const OpenAll = createContext(false);

/** A click on these never opens or folds the row. */
const CONTROLS = "button, a, input, textarea, select, label, [role=textbox], [role=menuitem], [role=checkbox], [contenteditable=true], [data-no-toggle]";

/** Long enough to tell a single click from the first of a double click (which selects a word). */
const CLICK_DELAY = 220;

export interface IOpenRows {
    isOpen: (id: string) => boolean;
    toggle: (id: string) => void;
    /** The row's click (to open or fold it) and its look while open; `locked` (an editor is open in it) keeps it open. */
    rowProps: (id: string, locked?: boolean) => { onClick: (e: React.MouseEvent<HTMLElement>) => void; "data-state"?: "selected"; "data-open"?: "" };
}

/**
 * Which rows of a table are open, by the row's id; `initial` (the row the
 * address names) starts open. `groupOf` names rows that open and fold
 * together (a player's choices with the line they answer).
 */
export function useOpenRows(initial: string | null = null, groupOf?: (id: string) => readonly string[] | undefined): IOpenRows {
    const [open, setOpen] = useState<ReadonlySet<string>>(() => new Set(initial ? [initial] : []));
    const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
    const toggle = useCallback(
        (id: string) => {
            const ids = groupOf?.(id) ?? [id];
            setOpen((before) => {
                const next = new Set(before);
                const opening = !before.has(id);
                for (const x of ids) {
                    if (opening) next.add(x);
                    else next.delete(x);
                }
                return next;
            });
        },
        [groupOf],
    );
    // The row the address names, once it changes (a link to another line of the same table).
    useEffect(() => {
        if (initial) setOpen((before) => (before.has(initial) ? before : new Set([...before, initial])));
    }, [initial]);
    // Esc folds the open rows, unless it is for a box being typed in, a dialog or a menu.
    const any = open.size > 0;
    useEffect(() => {
        if (!any) return;
        const onKey = (e: KeyboardEvent) => {
            if (e.key !== "Escape" || e.defaultPrevented) return;
            const target = e.target instanceof Element ? e.target : null;
            if (target?.closest("input, textarea, [contenteditable=true], [role=dialog], [role=alertdialog], [role=menu], [role=listbox]")) return;
            setOpen(new Set());
        };
        document.addEventListener("keydown", onKey);
        return () => document.removeEventListener("keydown", onKey);
    }, [any]);
    useEffect(
        () => () => {
            if (timer.current) clearTimeout(timer.current);
        },
        [],
    );
    const rowProps = (id: string, locked = false) => ({
        onClick: (e: React.MouseEvent<HTMLElement>) => {
            if (e.target instanceof Element && e.target.closest(CONTROLS)) return;
            if (timer.current) clearTimeout(timer.current);
            timer.current = null;
            // The second click of a double click selects a word (the selection menu): leave the row as it is.
            if (e.detail > 1) return;
            timer.current = setTimeout(() => {
                timer.current = null;
                if (window.getSelection()?.isCollapsed === false) return;
                if (locked && open.has(id)) return;
                toggle(id);
            }, CLICK_DELAY);
        },
        ...(open.has(id) ? { "data-state": "selected" as const, "data-open": "" as const } : {}),
    });
    return { isOpen: (id) => open.has(id), toggle, rowProps };
}

/** The № cell's chevron: opens or folds the row (the keyboard's way; the click on the row is a shortcut). */
export function RowToggle({ open, onToggle, label }: { open: boolean; onToggle: () => void; label: string }): React.ReactElement {
    return (
        <Button variant="ghost" size="xs" className="-ms-1 h-4 px-0.5 text-subtle-foreground" aria-expanded={open} aria-label={label} title={label} onClick={onToggle}>
            {open ? <ChevronDown className="size-3" aria-hidden="true" /> : <ChevronRight className="size-3" aria-hidden="true" />}
        </Button>
    );
}
