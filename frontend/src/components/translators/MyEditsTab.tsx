/**
 * The Edits tab: my proposals (everyone's for a translator) as was (the
 * game's Russian now) / now (the proposal) / where, with its state, and
 * rewrite, delete and the history of its texts. A translator decides right
 * in the list: a click on the state approves or rejects (with a reason the
 * author sees), and the rows ticked (Shift for a range) are decided at once
 * from the "N selected" bar. Export to the XUnity dictionaries: the Export…
 * dialog.
 */
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Check, ChevronDown, History, PenLine, RotateCcw, Trash2, Undo2, X } from "lucide-react";
import type React from "react";
import { useId, useMemo, useRef, useState } from "react";
import { Badge } from "#/components/ui/badge";
import { Button } from "#/components/ui/button";
import { Checkbox } from "#/components/ui/checkbox";
import { Collapsible, CollapsiblePanel, CollapsibleTrigger } from "#/components/ui/collapsible";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "#/components/ui/empty";
import { LoadFailed } from "#/components/ui/load-failed";
import { Menu, MenuItem, MenuPopup, MenuTrigger } from "#/components/ui/menu";
import { Skeleton } from "#/components/ui/skeleton";
import { Spinner } from "#/components/ui/spinner";
import { Switch } from "#/components/ui/switch";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "#/components/ui/table";
import { useFormatters, useLocale, useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { deleteProposalFn, editsQueryOptions, type ProposalDecision, proposalVersionsQueryOptions, rewriteProposalFn, translatorsMeQueryOptions } from "#/lib/translators/api";
import type { EditRow } from "#/types/generated/EditRow";
import type { TranslatorsMe } from "#/types/generated/TranslatorsMe";
import { ExportDialog } from "./ExportDialog";
import { CONFIRM_PLACES, cleanText, exportsForGame, isLead, marksReleases, RejectDialog, SaveDialog, TagsWarning, useDecide, useEditMutation, useEditorTools } from "./LineEdits";
import { StoryText } from "./StoryText";
import { useStoryNames } from "./storyNames";
import { TextDiff } from "./TextDiff";
import { TokenEditor } from "./TokenEditor";
import type { messages } from "./translators.messages";
import { fromWords } from "./translatorVariants";
import { plainText, TEXT_PLACE } from "./where";

type T = TypedT<typeof messages>;

const PROSE = "whitespace-normal py-1.5 align-top leading-snug";

const STATE_VARIANT = {
    proposed: "info",
    approved: "success",
    otherApproved: "secondary",
    outdated: "warning",
    rejected: "error",
} as const;

/** The state of a row; for a translator a menu: approve / reject a proposal, withdraw an approval, reopen a rejected one. */
function StateCell({ row, lead, t }: { row: EditRow; lead: boolean; t: T }): React.ReactElement {
    const [rejecting, setRejecting] = useState(false);
    const decide = useDecide(t);
    const p = row.proposal;
    const state = row.state as keyof typeof STATE_VARIANT;
    const badge = (
        <Badge variant={STATE_VARIANT[state] ?? "secondary"} size="sm">
            {t(`state.${state}` as "state.proposed")}
        </Badge>
    );
    const actions: { key: string; icon: React.ReactNode; label: string; run: () => void; destructive?: boolean }[] = [];
    const run = (decision: ProposalDecision) => () => decide.mutate({ ids: [p.id], decision, reason: "" });
    if (state === "proposed" || state === "otherApproved") {
        actions.push({ key: "approve", icon: <Check aria-hidden="true" />, label: t("decide.approve"), run: run("approve") });
        actions.push({ key: "reject", icon: <X aria-hidden="true" />, label: t("decide.reject"), run: () => setRejecting(true), destructive: true });
    } else if (state === "approved") {
        actions.push({ key: "withdraw", icon: <Undo2 aria-hidden="true" />, label: t("approve.withdraw"), run: run("withdraw") });
    } else if (state === "rejected") {
        actions.push({ key: "reopen", icon: <RotateCcw aria-hidden="true" />, label: t("decide.reopen"), run: run("reopen") });
    }
    return (
        <span className="flex flex-col items-start gap-0.5">
            {lead && actions.length > 0 ? (
                <Menu>
                    <MenuTrigger render={<Button variant="ghost" size="xs" className="h-auto gap-0.5 px-0.5 py-0.5" />} aria-label={t("decide.menu")} disabled={decide.isPending}>
                        {badge}
                        <ChevronDown className="size-3" aria-hidden="true" />
                    </MenuTrigger>
                    <MenuPopup align="start">
                        {actions.map((a) => (
                            <MenuItem key={a.key} onClick={a.run} variant={a.destructive ? "destructive" : "default"}>
                                {a.icon}
                                {a.label}
                            </MenuItem>
                        ))}
                    </MenuPopup>
                </Menu>
            ) : (
                badge
            )}
            {p.rejectedBy ? <span className="text-subtle-foreground text-xs">{t("decide.rejectedBy", { name: p.rejectedBy.name })}</span> : null}
            {p.rejectReason ? <span className="text-xs">{t("decide.reasonShown", { reason: p.rejectReason })}</span> : null}
            {p.agreements.length > 0 ? <span className="text-subtle-foreground text-xs">{t("proposal.agreed", { count: p.agreements.length })}</span> : null}
            {rejecting ? <RejectDialog open={rejecting} onOpenChange={setRejecting} count={1} pending={decide.isPending} onReject={(reason) => decide.mutate({ ids: [p.id], decision: "reject", reason }, { onSuccess: () => setRejecting(false) })} t={t} /> : null}
        </span>
    );
}

/** A proposal a translator may decide in the list: waiting for a decision. */
function decidable(row: EditRow): boolean {
    return row.state === "proposed" || row.state === "otherApproved";
}

/** The bar over the table while rows are ticked: "N selected · Approve · Reject". */
function SelectionBar({ ids, onDone, t }: { ids: number[]; onDone: () => void; t: T }): React.ReactElement | null {
    const [rejecting, setRejecting] = useState(false);
    const decide = useDecide(t);
    if (ids.length === 0) return null;
    return (
        <div className="sticky top-(--site-header-height) z-20 flex flex-wrap items-center gap-2 rounded-lg border bg-popover px-3 py-1.5 shadow-xs">
            <span className="font-medium text-sm">{t("decide.selected", { count: ids.length })}</span>
            <Button size="sm" onClick={() => decide.mutate({ ids, decision: "approve", reason: "" }, { onSuccess: onDone })} disabled={decide.isPending}>
                {decide.isPending ? <Spinner /> : <Check aria-hidden="true" />}
                {t("decide.approve")}
            </Button>
            <Button size="sm" variant="destructive-outline" onClick={() => setRejecting(true)} disabled={decide.isPending}>
                <X aria-hidden="true" />
                {t("decide.reject")}
            </Button>
            <Button size="sm" variant="ghost" onClick={onDone}>
                {t("decide.clear")}
            </Button>
            {rejecting ? (
                <RejectDialog
                    open={rejecting}
                    onOpenChange={setRejecting}
                    count={ids.length}
                    pending={decide.isPending}
                    onReject={(reason) =>
                        decide.mutate(
                            { ids, decision: "reject", reason },
                            {
                                onSuccess: () => {
                                    setRejecting(false);
                                    onDone();
                                },
                            },
                        )
                    }
                    t={t}
                />
            ) : null}
        </div>
    );
}

function Where({ row, t }: { row: EditRow; t: T }): React.ReactElement {
    const nameOf = useStoryNames();
    const { place } = row.proposal;
    // A text outside the story: it is found again in Search.
    if (place.storyId === TEXT_PLACE)
        return (
            <span className="flex flex-col gap-0.5">
                <Link to="/translators" search={{ tab: "search", q: plainText(row.proposal.editKey) }} className="underline-offset-4 hover:underline">
                    {t("where.text")}
                </Link>
                {row.places > 1 ? <span className="text-subtle-foreground text-xs">{t("where.places", { count: row.places })}</span> : null}
            </span>
        );
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
    const locale = useLocale();
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
                            {v.source !== "own" ? ` · ${fromWords(v.source, t, locale)}` : ""}
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
    const tools = useEditorTools(row.en, t);
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
            <TokenEditor id={id} initial={row.proposal.text} nickname={nickname} palette={tools.palette} tags={tools.tags} lookup={tools.lookup} onChange={setText} onSubmit={submit} onCancel={onClose} labels={tools.labels} />
            <TagsWarning en={row.en} text={text} words={tools.words} t={t} />
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

interface IRowProps {
    row: EditRow;
    me: TranslatorsMe;
    all: boolean;
    nickname: string;
    /** A translator: the tick box and the state's menu. */
    lead: boolean;
    selected: boolean;
    onSelect: (shift: boolean) => void;
    t: T;
}

function Row({ row, me, all, nickname, lead, selected, onSelect, t }: IRowProps): React.ReactElement {
    const f = useFormatters();
    const [rewriting, setRewriting] = useState(false);
    const remove = useEditMutation(() => deleteProposalFn({ data: { id: row.proposal.id } }), t("edit.failedDelete"));
    const p = row.proposal;
    const mine = p.author.id === me.userId;
    return (
        <TableRow className="group/row" data-state={selected ? "selected" : undefined}>
            {lead ? <TableCell className="py-1.5 align-top">{decidable(row) ? <Checkbox checked={selected} onCheckedChange={() => undefined} onClick={(e: React.MouseEvent) => onSelect(e.shiftKey)} aria-label={t("decide.tick")} /> : null}</TableCell> : null}
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
                    {p.here ? (
                        <span>
                            <Badge variant="warning" size="sm">
                                {t("proposal.onlyHere")}
                            </Badge>
                        </span>
                    ) : null}
                    {p.context && !p.here ? (
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
                <StateCell row={row} lead={lead} t={t} />
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
    const [selected, setSelected] = useState<ReadonlySet<number>>(new Set());
    // The last row ticked or unticked: Shift extends from it.
    const anchor = useRef<number | null>(null);
    const rows = query.data?.rows ?? [];
    const open = useMemo(() => rows.filter(decidable).map((r) => r.proposal.id), [rows]);
    // Only what is still waiting stays ticked (after a decision or a reload).
    const ticked = open.filter((id) => selected.has(id));

    if (me.isError || query.isError) return <LoadFailed message={t("edits.load")} onRetry={() => (me.isError ? me.refetch() : query.refetch())} />;
    if (!me.data || !query.data) return <Skeleton className="h-64 w-full rounded-2xl" />;

    const select = (index: number, shift: boolean) => {
        const id = rows[index].proposal.id;
        const next = new Set(selected);
        const on = !next.has(id);
        if (shift && anchor.current !== null) {
            const [from, to] = anchor.current < index ? [anchor.current, index] : [index, anchor.current];
            for (const r of rows.slice(from, to + 1)) if (decidable(r)) on ? next.add(r.proposal.id) : next.delete(r.proposal.id);
        } else if (on) next.add(id);
        else next.delete(id);
        anchor.current = index;
        setSelected(next);
    };
    const allTicked = open.length > 0 && ticked.length === open.length;

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
                    <p className="text-subtle-foreground text-xs">
                        {t("edits.count", { count: rows.length })}
                        {lead && open.length > 0 ? ` · ${t("decide.hint")}` : ""}
                    </p>
                    {lead ? <SelectionBar ids={ticked} onDone={() => setSelected(new Set())} t={t} /> : null}
                    <Table aria-label={t("edits.tableLabel")}>
                        <TableHeader>
                            <TableRow>
                                {lead ? <TableHead className="w-8">{open.length > 0 ? <Checkbox checked={allTicked} indeterminate={ticked.length > 0 && !allTicked} onCheckedChange={() => setSelected(allTicked ? new Set() : new Set(open))} aria-label={t("decide.all")} /> : null}</TableHead> : null}
                                <TableHead>{t("edits.col.was")}</TableHead>
                                <TableHead>{t("edits.col.now")}</TableHead>
                                <TableHead className="w-56">{t("edits.col.where")}</TableHead>
                                <TableHead className="w-40">{t("edits.col.state")}</TableHead>
                                <TableHead className="w-32">{t("edits.col.date")}</TableHead>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {rows.map((row, i) => (
                                <Row key={row.proposal.id} row={row} me={me.data} all={query.data.all} nickname={me.data.nickname ?? t("nickname.fallback")} lead={lead} selected={selected.has(row.proposal.id)} onSelect={(shift) => select(i, shift)} t={t} />
                            ))}
                        </TableBody>
                    </Table>
                </>
            )}
        </div>
    );
}
