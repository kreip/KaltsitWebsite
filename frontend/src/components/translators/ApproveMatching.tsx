/**
 * "Approve every line whose variants equal the game's text" (a translator or
 * an admin): the entries where each translator's and the machine's variant is
 * the game's text again (typography aside) and nobody proposed anything —
 * approved as in the game, checked. The story offers this story or the whole
 * chapter; the Game sections a whole section. The dialog lists the lines and
 * their number first (`POST /translators/approvals/matching`).
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { CheckCheck } from "lucide-react";
import type React from "react";
import { useState } from "react";
import { Button } from "#/components/ui/button";
import { Dialog, DialogClose, DialogDescription, DialogFooter, DialogHeader, DialogPanel, DialogPopup, DialogTitle } from "#/components/ui/dialog";
import { LoadFailed } from "#/components/ui/load-failed";
import { Radio, RadioGroup } from "#/components/ui/radio-group";
import { Skeleton } from "#/components/ui/skeleton";
import { Spinner } from "#/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "#/components/ui/table";
import { toastManager } from "#/components/ui/toast";
import type { TypedT } from "#/lib/i18n/messages";
import { approveBatchFn, EDIT_QUERY_ROOTS, matchingLinesFn, TRANSLATORS_QUERY_ROOT } from "#/lib/translators/api";
import type { MatchingLine } from "#/types/generated/MatchingLine";
import { StoryText } from "./StoryText";
import { useStoryNames } from "./storyNames";
import type { messages } from "./translators.messages";

type T = TypedT<typeof messages>;

/** The batch route takes this many at once. */
const BATCH = 5_000;

const PROSE = "whitespace-normal py-1.5 align-top leading-snug";

/** Where to look: stories (one, or a chapter's), or a game section. */
export interface IMatchingScope {
    storyIds: string[];
    section: string | null;
}

function Where({ line, t }: { line: MatchingLine; t: T }): React.ReactElement {
    const nameOf = useStoryNames();
    const n = nameOf(line.place.storyId);
    return n ? (
        <Link to="/translators" search={{ group: n.groupId, story: line.place.storyId }} hash={`n-${line.ord + 1}`} className="underline-offset-4 hover:underline">
            {t("places.item", { group: n.group, story: n.story, n: line.ord + 1 })}
        </Link>
    ) : (
        <span>{t("places.itemBare", { story: line.place.storyId, n: line.ord + 1 })}</span>
    );
}

function MatchingDialog({ scopes, nickname, onClose, t }: { scopes: { label: string; scope: IMatchingScope }[]; nickname: string; onClose: () => void; t: T }): React.ReactElement {
    const [at, setAt] = useState(0);
    const scope = scopes[at]?.scope ?? scopes[0].scope;
    const queryClient = useQueryClient();
    const query = useQuery({
        queryKey: [...TRANSLATORS_QUERY_ROOT, "matching", scope],
        queryFn: () => matchingLinesFn({ data: scope }),
    });
    const approve = useMutation({
        mutationFn: async (lines: readonly MatchingLine[]) => {
            let approved = 0;
            let skipped = 0;
            for (let i = 0; i < lines.length; i += BATCH) {
                const r = await approveBatchFn({ data: { items: lines.slice(i, i + BATCH).map((l) => ({ place: l.place, variant: "game", proposalId: null, approve: true })) } });
                approved += r.approved;
                skipped += r.skipped;
            }
            return { approved, skipped };
        },
        onSuccess: async (r) => {
            toastManager.add({ title: t("bulk.done", r), type: "success" });
            await Promise.all([...EDIT_QUERY_ROOTS, [...TRANSLATORS_QUERY_ROOT, "matching"]].map((queryKey) => queryClient.invalidateQueries({ queryKey: [...queryKey] })));
            onClose();
        },
        onError: () => toastManager.add({ title: t("bulk.failed"), type: "error" }),
    });
    const lines = query.data?.lines ?? [];
    return (
        <Dialog open onOpenChange={(open) => !open && onClose()}>
            <DialogPopup className="max-w-4xl">
                <DialogHeader>
                    <DialogTitle>{query.data ? t("matching.title", { count: lines.length }) : t("matching.button")}</DialogTitle>
                    <DialogDescription>{t("matching.hint")}</DialogDescription>
                </DialogHeader>
                <DialogPanel className="flex flex-col gap-3">
                    {scopes.length > 1 ? (
                        <RadioGroup aria-label={t("matching.scope")} value={String(at)} onValueChange={(v) => setAt(Number(v))} className="flex flex-row flex-wrap gap-4">
                            {scopes.map((s, i) => (
                                // biome-ignore lint/a11y/noLabelWithoutControl: the kit Radio inside is the control
                                <label key={s.label} className="flex items-center gap-2 text-sm">
                                    <Radio value={String(i)} />
                                    {s.label}
                                </label>
                            ))}
                        </RadioGroup>
                    ) : null}
                    {query.isError ? (
                        <LoadFailed layout="inline" message={t("matching.load")} onRetry={() => query.refetch()} />
                    ) : !query.data ? (
                        <Skeleton className="h-40 w-full" />
                    ) : lines.length === 0 ? (
                        <p className="text-muted-foreground text-sm">{t("matching.none")}</p>
                    ) : (
                        <div className="max-h-[55vh] overflow-y-auto">
                            <Table aria-label={t("matching.table")} className="wrap-anywhere text-xs">
                                <TableHeader>
                                    <TableRow>
                                        <TableHead className="w-48">{t("edits.col.where")}</TableHead>
                                        <TableHead>{t("table.ru")}</TableHead>
                                        <TableHead>{t("table.en")}</TableHead>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {lines.map((l) => (
                                        <TableRow key={`${l.place.storyId}:${l.ord}`}>
                                            <TableCell className={PROSE}>
                                                <Where line={l} t={t} />
                                            </TableCell>
                                            <TableCell className={PROSE}>
                                                <StoryText text={l.ru} nickname={nickname} />
                                            </TableCell>
                                            <TableCell className={`${PROSE} text-muted-foreground`}>
                                                <StoryText text={l.en} nickname={nickname} />
                                            </TableCell>
                                        </TableRow>
                                    ))}
                                </TableBody>
                            </Table>
                        </div>
                    )}
                </DialogPanel>
                <DialogFooter>
                    <DialogClose render={<Button variant="ghost" />}>{t("editor.cancel")}</DialogClose>
                    <Button onClick={() => approve.mutate(lines)} disabled={approve.isPending || lines.length === 0}>
                        {approve.isPending ? <Spinner /> : <CheckCheck aria-hidden="true" />}
                        {t("matching.confirm", { count: lines.length })}
                    </Button>
                </DialogFooter>
            </DialogPopup>
        </Dialog>
    );
}

/** The button and its dialog (a translator or an admin: the caller checks). The first scope is the default. */
export function ApproveMatching({ scopes, nickname, t }: { scopes: { label: string; scope: IMatchingScope }[]; nickname: string; t: T }): React.ReactElement {
    const [open, setOpen] = useState(false);
    return (
        <>
            <Button variant="outline" size="sm" onClick={() => setOpen(true)}>
                <CheckCheck aria-hidden="true" />
                {t("matching.button")}
            </Button>
            {open ? <MatchingDialog scopes={scopes} nickname={nickname} onClose={() => setOpen(false)} t={t} /> : null}
        </>
    );
}
