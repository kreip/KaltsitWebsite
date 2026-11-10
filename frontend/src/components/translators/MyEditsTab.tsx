/**
 * The My edits tab: my proposals (everyone's for the lead) as was (the
 * game's Russian now) / now (the proposal) / where, with its state, and
 * rewrite, delete and the history of its texts. Export to the XUnity
 * dictionaries: the Export… dialog.
 */
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { History, PenLine, Trash2 } from "lucide-react";
import type React from "react";
import { useId, useState } from "react";
import { Badge } from "#/components/ui/badge";
import { Button } from "#/components/ui/button";
import { Collapsible, CollapsiblePanel, CollapsibleTrigger } from "#/components/ui/collapsible";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "#/components/ui/empty";
import { LoadFailed } from "#/components/ui/load-failed";
import { Skeleton } from "#/components/ui/skeleton";
import { Spinner } from "#/components/ui/spinner";
import { Switch } from "#/components/ui/switch";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "#/components/ui/table";
import { useFormatters, useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { deleteProposalFn, editsQueryOptions, proposalVersionsQueryOptions, rewriteProposalFn, translatorsMeQueryOptions } from "#/lib/translators/api";
import type { EditRow } from "#/types/generated/EditRow";
import type { TranslatorsMe } from "#/types/generated/TranslatorsMe";
import { ExportDialog } from "./ExportDialog";
import { CONFIRM_PLACES, cleanText, exportsForGame, isLead, marksReleases, SaveDialog, useEditMutation } from "./LineEdits";
import { StoryText } from "./StoryText";
import { useStoryNames } from "./storyNames";
import { TextDiff } from "./TextDiff";
import { TokenEditor } from "./TokenEditor";
import type { messages } from "./translators.messages";

type T = TypedT<typeof messages>;

const PROSE = "whitespace-normal py-1.5 align-top leading-snug";

const STATE_VARIANT = {
    proposed: "info",
    approved: "success",
    otherApproved: "secondary",
    outdated: "warning",
} as const;

function Where({ row, t }: { row: EditRow; t: T }): React.ReactElement {
    const nameOf = useStoryNames();
    const { place } = row.proposal;
    const n = nameOf(place.storyId);
    const label = t("where.item", { group: n?.group ?? "", story: n?.story ?? place.storyId, n: row.ord === null ? "?" : row.ord + 1 });
    return (
        <span className="flex flex-col gap-0.5">
            {n ? (
                <Link to="/translators" search={{ group: n.groupId, story: place.storyId }} hash={row.ord === null ? undefined : `n-${row.ord + 1}`} className="underline-offset-4 hover:underline">
                    {label}
                </Link>
            ) : (
                <span>{label}</span>
            )}
            {row.places > 1 ? <span className="text-subtle-foreground text-xs">{t("where.places", { count: row.places })}</span> : null}
        </span>
    );
}

/** A proposal's texts, newest first, each compared by words with the one before it (the first with the game's Russian). */
function Versions({ id, ruGame, t }: { id: number; ruGame: string | null; t: T }): React.ReactElement {
    const f = useFormatters();
    const query = useQuery(proposalVersionsQueryOptions(id));
    if (query.isError) return <LoadFailed layout="inline" message={t("history.load")} onRetry={() => query.refetch()} />;
    if (!query.data) return <Skeleton className="h-10 w-full" />;
    const versions = query.data;
    const labels = { added: t("diff.added"), removed: t("diff.removed") };
    return (
        <ol className="flex flex-col gap-1 text-xs" aria-label={t("history.label")}>
            {versions.map((v, i) => {
                const before = versions[i + 1]?.text ?? ruGame;
                return (
                    <li key={v.id} className="flex flex-col border-s-2 ps-2">
                        <span className="text-subtle-foreground">
                            {f.date(v.createdAt * 1000, { dateStyle: "medium", timeStyle: "short" })} · {v.author.name}
                            {v.source !== "own" ? ` · ${t(`proposal.from.${v.source}` as "proposal.from.sova")}` : ""}
                            {i === 0 ? ` · ${t("history.current")}` : ""}
                        </span>
                        {before === null ? <span className="whitespace-pre-line">{v.text}</span> : <TextDiff before={before} after={v.text} labels={labels} />}
                    </li>
                );
            })}
        </ol>
    );
}

function Rewrite({ row, nickname, onClose, t }: { row: EditRow; nickname: string; onClose: () => void; t: T }): React.ReactElement {
    const id = useId();
    const [text, setText] = useState(row.proposal.text);
    const [asking, setAsking] = useState(false);
    const save = useEditMutation((confirmMany: boolean) => rewriteProposalFn({ data: { id: row.proposal.id, text, confirmMany } }), t("edit.failedSave"));
    const clean = cleanText(text);
    const sameAsGame = row.ruGame !== null && clean === cleanText(row.ruGame);
    const canSave = clean.length > 0 && !sameAsGame && clean !== row.proposal.text && !save.isPending;
    const several = !row.proposal.context && row.places > CONFIRM_PLACES;
    const submit = () => {
        if (!canSave) return;
        if (several) setAsking(true);
        else save.mutate(false, { onSuccess: onClose });
    };
    return (
        <div className="flex flex-col gap-1.5">
            <label htmlFor={id} className="text-subtle-foreground text-xs">
                {t("editor.rewrite")}
            </label>
            <TokenEditor id={id} initial={row.proposal.text} nickname={nickname} onChange={setText} onSubmit={submit} onCancel={onClose} insertLabel={t("editor.insertNickname")} />
            <span className="text-subtle-foreground text-xs">{t("save.proposalOnly")}</span>
            {sameAsGame ? <span className="text-subtle-foreground text-xs">{t("editor.sameAsGame")}</span> : null}
            {several ? <span className="text-warning-foreground text-xs">{t("places.note", { count: row.places - 1 })}</span> : null}
            <div className="flex flex-wrap items-center gap-2">
                <Button size="sm" onClick={submit} disabled={!canSave}>
                    {save.isPending ? <Spinner /> : null}
                    {t("editor.save")}
                </Button>
                <Button size="sm" variant="ghost" onClick={onClose}>
                    {t("editor.cancel")}
                </Button>
                <span className="text-subtle-foreground text-xs">{t("editor.keys")}</span>
            </div>
            {asking ? <SaveDialog open={asking} onOpenChange={setAsking} place={row.proposal.place} places={row.places} allowContext={false} pending={save.isPending} onSave={() => save.mutate(true, { onSuccess: onClose })} nickname={nickname} t={t} /> : null}
        </div>
    );
}

function Row({ row, me, all, nickname, t }: { row: EditRow; me: TranslatorsMe; all: boolean; nickname: string; t: T }): React.ReactElement {
    const f = useFormatters();
    const [rewriting, setRewriting] = useState(false);
    const remove = useEditMutation(() => deleteProposalFn({ data: { id: row.proposal.id } }), t("edit.failedDelete"));
    const p = row.proposal;
    const mine = p.author.id === me.userId;
    const state = row.state as keyof typeof STATE_VARIANT;
    return (
        <TableRow className="group/row">
            <TableCell className={PROSE}>
                <span className="flex flex-col gap-0.5">
                    {row.ruGame ? <StoryText text={row.ruGame} nickname={nickname} /> : <span className="text-subtle-foreground">{t("ru.missing")}</span>}
                    <span className="text-subtle-foreground text-xs">
                        <StoryText text={row.en} nickname={nickname} />
                    </span>
                </span>
            </TableCell>
            <TableCell className={PROSE}>
                <div className="flex flex-col gap-1">
                    {all ? <span className="text-subtle-foreground text-xs">{mine ? t("proposal.mine") : p.author.name}</span> : null}
                    {rewriting ? <Rewrite row={row} nickname={nickname} onClose={() => setRewriting(false)} t={t} /> : <StoryText text={p.text} nickname={nickname} />}
                    {p.context ? (
                        <span>
                            <Badge variant="info" size="sm">
                                {t("proposal.contextBadge")}
                            </Badge>
                        </span>
                    ) : null}
                    <Collapsible>
                        <div className="flex flex-wrap gap-1">
                            {mine && !rewriting ? (
                                <>
                                    <Button variant="ghost" size="sm" onClick={() => setRewriting(true)}>
                                        <PenLine aria-hidden="true" />
                                        {t("proposal.rewrite")}
                                    </Button>
                                    <Button variant="ghost" size="sm" onClick={() => remove.mutate(undefined)} disabled={remove.isPending}>
                                        <Trash2 aria-hidden="true" />
                                        {t("proposal.delete")}
                                    </Button>
                                </>
                            ) : null}
                            <CollapsibleTrigger render={<Button variant="ghost" size="sm" />}>
                                <History aria-hidden="true" />
                                {t("history.open", { count: p.versions })}
                            </CollapsibleTrigger>
                        </div>
                        <CollapsiblePanel className="pt-1">
                            <Versions id={p.id} ruGame={row.ruGame} t={t} />
                        </CollapsiblePanel>
                    </Collapsible>
                </div>
            </TableCell>
            <TableCell className={PROSE}>
                <Where row={row} t={t} />
            </TableCell>
            <TableCell className={PROSE}>
                <span className="flex flex-col items-start gap-0.5">
                    <Badge variant={STATE_VARIANT[state] ?? "secondary"} size="sm">
                        {t(`state.${state}` as "state.proposed")}
                    </Badge>
                    {p.agreements.length > 0 ? <span className="text-subtle-foreground text-xs">{t("proposal.agreed", { count: p.agreements.length })}</span> : null}
                </span>
            </TableCell>
            <TableCell className="py-1.5 align-top text-muted-foreground text-xs tabular-nums">{f.date((p.editedAt ?? p.createdAt) * 1000, { dateStyle: "medium", timeStyle: "short" })}</TableCell>
        </TableRow>
    );
}

export function MyEditsTab(): React.ReactElement {
    const t: T = useT("translators");
    const allId = useId();
    const me = useQuery(translatorsMeQueryOptions());
    const lead = me.data ? isLead(me.data) : false;
    const [all, setAll] = useState(false);
    const query = useQuery({ ...editsQueryOptions(all && lead), enabled: Boolean(me.data) });

    if (me.isError || query.isError) return <LoadFailed message={t("edits.load")} onRetry={() => (me.isError ? me.refetch() : query.refetch())} />;
    if (!me.data || !query.data) return <Skeleton className="h-64 w-full rounded-2xl" />;
    const rows = query.data.rows;

    return (
        <div className="flex flex-col gap-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
                <div className="flex flex-wrap items-center gap-4">
                    {lead ? (
                        <label htmlFor={allId} className="flex items-center gap-3">
                            <Switch id={allId} checked={all} onCheckedChange={setAll} />
                            <span className="font-medium text-sm">{t("edits.all")}</span>
                        </label>
                    ) : null}
                    <ExportDialog forGame={exportsForGame(me.data)} releases={marksReleases(me.data)} t={t} />
                </div>
            </div>
            {rows.length === 0 ? (
                <Empty>
                    <EmptyHeader>
                        <EmptyTitle>{t("edits.empty")}</EmptyTitle>
                        <EmptyDescription>{t("edits.emptyHint")}</EmptyDescription>
                    </EmptyHeader>
                </Empty>
            ) : (
                <>
                    <p className="text-subtle-foreground text-xs">{t("edits.count", { count: rows.length })}</p>
                    <Table aria-label={t("edits.tableLabel")}>
                        <TableHeader>
                            <TableRow>
                                <TableHead>{t("edits.col.was")}</TableHead>
                                <TableHead>{t("edits.col.now")}</TableHead>
                                <TableHead className="w-56">{t("edits.col.where")}</TableHead>
                                <TableHead className="w-32">{t("edits.col.state")}</TableHead>
                                <TableHead className="w-32">{t("edits.col.date")}</TableHead>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {rows.map((row) => (
                                <Row key={row.proposal.id} row={row} me={me.data} all={query.data.all} nickname={me.data.nickname ?? t("nickname.fallback")} t={t} />
                            ))}
                        </TableBody>
                    </Table>
                </>
            )}
        </div>
    );
}
