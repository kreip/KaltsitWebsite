/**
 * A long text (an operator's file, a module's story) folds to a few lines in
 * a table; a click on it, or the row's button, unfolds the whole row. Short
 * rows are left alone.
 */
import type React from "react";
import { useCallback, useState } from "react";

/** From this many characters or line breaks a text is long: more than the few lines a folded row shows. */
const LONG_CHARS = 600;
const LONG_BREAKS = 8;

export function isLong(...texts: readonly (string | null | undefined)[]): boolean {
    return texts.some((text) => text != null && (text.length > LONG_CHARS || text.split("\n").length > LONG_BREAKS));
}

/** Which rows of a table are unfolded, by the row's id. */
export function useUnfolded(): { has: (id: string) => boolean; toggle: (id: string) => void } {
    const [open, setOpen] = useState<ReadonlySet<string>>(new Set());
    const toggle = useCallback((id: string) => {
        setOpen((before) => {
            const next = new Set(before);
            if (!next.delete(id)) next.add(id);
            return next;
        });
    }, []);
    return { has: (id) => open.has(id), toggle };
}

/** A cell's content, cut to a few lines while `folded`; a plain click on it (not a selection, not a button) unfolds. */
export function Folded({ folded, onOpen, children }: { folded: boolean; onOpen: () => void; children: React.ReactNode }): React.ReactElement {
    if (!folded) return <>{children}</>;
    return (
        // biome-ignore lint/a11y/noStaticElementInteractions: a shortcut; the row's Show all button is the accessible way
        // biome-ignore lint/a11y/useKeyWithClickEvents: as above
        <div
            className="max-h-24 cursor-pointer overflow-hidden"
            onClick={(e) => {
                const onControl = e.target instanceof Element && e.target.closest("button, a, [role=textbox]");
                if (!onControl && window.getSelection()?.isCollapsed !== false) onOpen();
            }}
        >
            {children}
        </div>
    );
}
