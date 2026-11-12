/**
 * What a member can do with a word or a phrase selected in any text of the
 * section's tables (a double click on a word, a drag over a phrase): a small
 * menu at the selection — find it everywhere (the Search tab), find it in
 * the glossary, or suggest it for the glossary (a note to the lead; the
 * team's sheet is never written from the site). A cell that can be selected
 * from is marked `data-sel` with its language; its row may name its place in
 * `data-place` (`story|line|part`).
 */
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { BookA, BookPlus, Search } from "lucide-react";
import type React from "react";
import { useEffect, useId, useState } from "react";
import { Button } from "#/components/ui/button";
import { Dialog, DialogClose, DialogDescription, DialogFooter, DialogHeader, DialogPanel, DialogPopup, DialogTitle } from "#/components/ui/dialog";
import { Input } from "#/components/ui/input";
import { Label } from "#/components/ui/label";
import { Spinner } from "#/components/ui/spinner";
import { Textarea } from "#/components/ui/textarea";
import { toastManager } from "#/components/ui/toast";
import { useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { createSuggestionFn, suggestionsQueryOptions } from "#/lib/translators/api";
import type { messages } from "./translators.messages";

type T = TypedT<typeof messages>;

/** The longest selection the menu answers to: a term or a phrase, not a paragraph. */
const MAX_SELECTED = 200;

interface ISelected {
    text: string;
    /** The language of the cell it was selected in. */
    lang: string;
    /** Where the menu stands, in the window. */
    left: number;
    top: number;
    /** The row's story line, when it is one. */
    place: { storyId: string; line: number; part: number } | null;
    /** The English of the row. */
    sourceEn: string | null;
}

function elementOf(node: Node | null): Element | null {
    return node instanceof Element ? node : (node?.parentElement ?? null);
}

/** The selection, when it is a short text inside one selectable cell. */
function readSelection(): ISelected | null {
    const sel = window.getSelection();
    if (!sel || sel.isCollapsed || sel.rangeCount === 0) return null;
    const text = sel.toString().replace(/\s+/g, " ").trim();
    if (!text || text.length > MAX_SELECTED) return null;
    const from = elementOf(sel.anchorNode);
    const cell = from?.closest("[data-sel]");
    if (!cell || !(cell instanceof HTMLElement) || elementOf(sel.focusNode)?.closest("[data-sel]") !== cell) return null;
    // Not while typing a proposal or a comment.
    if (from?.closest("[contenteditable], [role=textbox], input, textarea")) return null;
    const rect = sel.getRangeAt(0).getBoundingClientRect();
    if (rect.width === 0 && rect.height === 0) return null;
    const row = cell.closest("[data-place]");
    const [storyId, line, part] = (row instanceof HTMLElement ? (row.dataset.place ?? "") : "").split("|");
    const en = (row ?? cell.closest("tr"))?.querySelector('[data-sel="en"]')?.textContent?.trim();
    // Under the selection; above it when there is no room below.
    const below = rect.bottom + 6;
    return {
        text,
        lang: cell.dataset.sel ?? "ru",
        left: Math.max(8, Math.min(rect.left, window.innerWidth - 440)),
        top: below + 40 > window.innerHeight ? Math.max(8, rect.top - 38) : below,
        place: storyId && line && part && !storyId.startsWith("@") ? { storyId, line: Number(line), part: Number(part) } : null,
        sourceEn: en ? en.slice(0, 400) : null,
    };
}

/** The dialog of "Suggest for the glossary": the English, the Russian, a note. */
function SuggestDialog({ selected, onClose, t }: { selected: ISelected; onClose: () => void; t: T }): React.ReactElement {
    const ids = { en: useId(), ru: useId(), note: useId() };
    const queryClient = useQueryClient();
    const [en, setEn] = useState(selected.lang === "en" ? selected.text : "");
    const [ru, setRu] = useState(selected.lang === "ru" ? selected.text : "");
    const [note, setNote] = useState(selected.lang === "cn" ? selected.text : "");
    const save = useMutation({
        mutationFn: () => createSuggestionFn({ data: { en, ru, note: note.trim() || null, storyId: selected.place?.storyId ?? null, line: selected.place?.line ?? null, part: selected.place?.part ?? null, sourceEn: selected.sourceEn } }),
        onSuccess: () => {
            toastManager.add({ title: t("suggest.sent"), type: "success" });
            queryClient.invalidateQueries({ queryKey: suggestionsQueryOptions().queryKey });
            onClose();
        },
        onError: () => toastManager.add({ title: t("suggest.failed"), type: "error" }),
    });
    const ready = en.trim().length > 0 && ru.trim().length > 0 && !save.isPending;
    return (
        <Dialog open onOpenChange={(open: boolean) => !open && onClose()}>
            <DialogPopup className="max-w-lg">
                <DialogHeader>
                    <DialogTitle>{t("suggest.title")}</DialogTitle>
                    <DialogDescription>{t("suggest.description")}</DialogDescription>
                </DialogHeader>
                <DialogPanel className="flex flex-col gap-3">
                    <div className="flex flex-col gap-1.5">
                        <Label htmlFor={ids.en}>{t("suggest.en")}</Label>
                        <Input id={ids.en} value={en} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setEn(e.target.value)} autoFocus={selected.lang !== "en"} />
                    </div>
                    <div className="flex flex-col gap-1.5">
                        <Label htmlFor={ids.ru}>{t("suggest.ru")}</Label>
                        <Input id={ids.ru} value={ru} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setRu(e.target.value)} autoFocus={selected.lang === "en"} />
                    </div>
                    <div className="flex flex-col gap-1.5">
                        <Label htmlFor={ids.note}>{t("suggest.note")}</Label>
                        <Textarea id={ids.note} size="sm" value={note} onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => setNote(e.target.value)} />
                    </div>
                    {selected.sourceEn ? <p className="text-subtle-foreground text-xs">{t("suggest.from", { en: selected.sourceEn })}</p> : null}
                </DialogPanel>
                <DialogFooter>
                    <DialogClose render={<Button variant="ghost" />}>{t("editor.cancel")}</DialogClose>
                    <Button onClick={() => save.mutate()} disabled={!ready}>
                        {save.isPending ? <Spinner /> : null}
                        {t("suggest.send")}
                    </Button>
                </DialogFooter>
            </DialogPopup>
        </Dialog>
    );
}

export interface ISelectionMenuProps {
    /** Open the Search tab on a text. */
    onFind: (text: string) => void;
    /** Open the Glossary tab filtered by a text. */
    onGlossary: (text: string) => void;
}

export function SelectionMenu({ onFind, onGlossary }: ISelectionMenuProps): React.ReactElement | null {
    const t: T = useT("translators");
    const [selected, setSelected] = useState<ISelected | null>(null);
    const [suggesting, setSuggesting] = useState<ISelected | null>(null);

    useEffect(() => {
        // Once the browser has settled the selection (a double click selects on mouse up).
        const read = () => window.setTimeout(() => setSelected(readSelection()), 0);
        const keys = (e: KeyboardEvent) => {
            if (e.key === "Escape") setSelected(null);
            else if (e.shiftKey || e.key === "Shift") read();
        };
        const hide = () => setSelected(null);
        document.addEventListener("mouseup", read);
        document.addEventListener("keyup", keys);
        window.addEventListener("scroll", hide, true);
        window.addEventListener("resize", hide);
        return () => {
            document.removeEventListener("mouseup", read);
            document.removeEventListener("keyup", keys);
            window.removeEventListener("scroll", hide, true);
            window.removeEventListener("resize", hide);
        };
    }, []);

    const act = (run: (s: ISelected) => void) => () => {
        if (!selected) return;
        const s = selected;
        window.getSelection()?.removeAllRanges();
        setSelected(null);
        run(s);
    };

    return (
        <>
            {selected ? (
                // The buttons keep the selection: a press must not clear it before the click reads it.
                <div role="toolbar" aria-label={t("selection.label")} className="fixed z-50 flex items-center gap-0.5 rounded-lg border bg-popover p-0.5 shadow-xs" style={{ left: selected.left, top: selected.top }} onMouseDown={(e) => e.preventDefault()}>
                    <Button variant="ghost" size="xs" onClick={act((s) => onFind(s.text))}>
                        <Search aria-hidden="true" />
                        {t("selection.find")}
                    </Button>
                    <Button variant="ghost" size="xs" onClick={act((s) => onGlossary(s.text))}>
                        <BookA aria-hidden="true" />
                        {t("selection.glossary")}
                    </Button>
                    <Button variant="ghost" size="xs" onClick={act(setSuggesting)}>
                        <BookPlus aria-hidden="true" />
                        {t("selection.suggest")}
                    </Button>
                </div>
            ) : null}
            {suggesting ? <SuggestDialog selected={suggesting} onClose={() => setSuggesting(null)} t={t} /> : null}
        </>
    );
}
