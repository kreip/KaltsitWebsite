/**
 * "Replace the word in other lines" (Kolooo: "I change Бибек to Байбик and
 * the site doesn't offer to change it in the other lines"). A saved proposal
 * that changed one word for another (wordSwap.ts) asks the backend which
 * other texts hold the old word; when some do, a toast offers "Replace
 * «Бибек» → «Байбик» in N more lines?", which opens the same dialog as the
 * glossary's replace (ReplaceDialog.tsx: was / will be in the same case,
 * tick boxes). When the glossary has the old word, the toast and the dialog
 * say the glossary may need fixing too.
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import type React from "react";
import { createContext, useCallback, useState } from "react";
import { Alert, AlertDescription } from "#/components/ui/alert";
import { Dialog, DialogPopup } from "#/components/ui/dialog";
import { toastManager } from "#/components/ui/toast";
import { useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { EDIT_QUERY_ROOTS, replaceWordFn, replaceWordQueryOptions, translatorsMeQueryOptions } from "#/lib/translators/api";
import type { LinePick } from "#/types/generated/LinePick";
import type { WordGlossaryTerm } from "#/types/generated/WordGlossaryTerm";
import { ReplaceDialogBody } from "./ReplaceDialog";
import type { messages } from "./translators.messages";

type T = TypedT<typeof messages>;

export interface IWordOffer {
    from: string;
    to: string;
    /** The entry the proposal was made for: not listed. */
    skip: string | null;
}

/** Offer the replacement after a proposal was saved; null outside the section's page. */
export const WordReplaceOffer = createContext<((offer: IWordOffer) => void) | null>(null);

function GlossaryNote({ terms, t }: { terms: readonly WordGlossaryTerm[]; t: T }): React.ReactElement | null {
    if (terms.length === 0) return null;
    return (
        <Alert variant="warning">
            <AlertDescription>
                <span>{t("replaceWord.glossary")}</span>
                <span className="flex flex-wrap gap-x-3">
                    {terms.map((term) => (
                        <Link key={`${term.en}-${term.ru}`} to="/translators" search={{ tab: "glossary", gq: term.en }} className="underline underline-offset-4">
                            {term.en} → {term.ru}
                        </Link>
                    ))}
                </span>
            </AlertDescription>
        </Alert>
    );
}

function WordReplaceDialog({ offer, onDone, t }: { offer: IWordOffer; onDone: () => void; t: T }): React.ReactElement {
    const queryClient = useQueryClient();
    const me = useQuery(translatorsMeQueryOptions());
    const nickname = me.data?.nickname ?? t("nickname.fallback");
    const preview = useQuery(replaceWordQueryOptions(offer.from, offer.to, offer.skip));
    const apply = useMutation({
        mutationFn: (picked: LinePick[]) => replaceWordFn({ data: { ...offer, apply: true, picked } }),
        onSuccess: (data) => {
            toastManager.add({ title: t("glossary.fix.done", { count: data.created }), type: "success" });
            for (const root of EDIT_QUERY_ROOTS) queryClient.invalidateQueries({ queryKey: root });
            onDone();
        },
        onError: () => toastManager.add({ title: t("glossary.fix.failed"), type: "error" }),
    });
    const more = preview.data?.more ? <p className="text-subtle-foreground text-xs">{t("replaceWord.more", { count: preview.data.lines.length })}</p> : null;
    return (
        <ReplaceDialogBody
            title={t("glossary.fix.title", { from: offer.from, to: offer.to })}
            description={t("replaceWord.description")}
            note={
                <>
                    <GlossaryNote terms={preview.data?.glossary ?? []} t={t} />
                    {more}
                </>
            }
            data={preview.data}
            isError={preview.isError}
            onRetry={() => preview.refetch()}
            pending={apply.isPending}
            onApply={(picked) => apply.mutate(picked)}
            nickname={nickname}
            t={t}
        />
    );
}

/** Provides the offer to every table under it and shows its dialog. */
export function WordReplaceHost({ children }: { children: React.ReactNode }): React.ReactElement {
    const t: T = useT("translators");
    const queryClient = useQueryClient();
    const [open, setOpen] = useState<IWordOffer | null>(null);
    const ask = useCallback(
        (offer: IWordOffer) => {
            queryClient
                .fetchQuery(replaceWordQueryOptions(offer.from, offer.to, offer.skip))
                .then((data) => {
                    const count = data.lines.length;
                    const glossary = data.glossary.map((g) => `${g.en} → ${g.ru}`).join(", ");
                    if (count === 0 && !glossary) return;
                    const id = toastManager.add({
                        title: count > 0 ? t("replaceWord.offer", { from: offer.from, to: offer.to, count }) : t("replaceWord.glossaryOnly", { terms: glossary }),
                        description: count > 0 && glossary ? t("replaceWord.glossaryToo", { terms: glossary }) : undefined,
                        type: "info",
                        timeout: 20_000,
                        actionProps:
                            count > 0
                                ? {
                                      children: t("replaceWord.show"),
                                      onClick: () => {
                                          toastManager.close(id);
                                          setOpen(offer);
                                      },
                                  }
                                : undefined,
                    });
                })
                // The proposal is saved; the offer is a convenience.
                .catch(() => undefined);
        },
        [queryClient, t],
    );
    return (
        <WordReplaceOffer.Provider value={ask}>
            {children}
            <Dialog open={open !== null} onOpenChange={(o) => !o && setOpen(null)}>
                <DialogPopup className="max-w-5xl">{open ? <WordReplaceDialog offer={open} onDone={() => setOpen(null)} t={t} /> : null}</DialogPopup>
            </Dialog>
        </WordReplaceOffer.Provider>
    );
}
