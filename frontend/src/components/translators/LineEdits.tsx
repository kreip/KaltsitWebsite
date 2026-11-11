/**
 * Editing one story line: the approved variant,
 * the translators' proposals, remarks and translator's notes under the
 * Russian, the cell's actions on hover (propose, remark, note; a double click
 * on the Russian also proposes), the in-row editor and the save dialog: an
 * entry in more than one place is saved for all its places (with the list and
 * a confirmation) or for this place only (a context translation).
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Check, CheckCheck, History, MessageSquare, NotebookPen, PenLine, ThumbsUp, Trash2, Undo2 } from "lucide-react";
import type React from "react";
import { useId, useState } from "react";
import { AlertDialog, AlertDialogClose, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogPopup, AlertDialogTitle } from "#/components/ui/alert-dialog";
import { Badge } from "#/components/ui/badge";
import { Button } from "#/components/ui/button";
import { Checkbox } from "#/components/ui/checkbox";
import { Collapsible, CollapsiblePanel, CollapsibleTrigger } from "#/components/ui/collapsible";
import { LoadFailed } from "#/components/ui/load-failed";
import { Radio, RadioGroup } from "#/components/ui/radio-group";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "#/components/ui/select";
import { Skeleton } from "#/components/ui/skeleton";
import { Spinner } from "#/components/ui/spinner";
import { Textarea } from "#/components/ui/textarea";
import { toastManager } from "#/components/ui/toast";
import { useFormatters } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { type AgreeVariant, createCommentFn, createProposalFn, deleteCommentFn, deleteProposalFn, EDIT_QUERY_ROOTS, type ProposalSource, placesQueryOptions, resolveCommentFn, setAgreementFn, setApprovalFn } from "#/lib/translators/api";
import { cn } from "#/lib/utils";
import type { EditComment } from "#/types/generated/EditComment";
import type { EditPerson } from "#/types/generated/EditPerson";
import type { EditPlace } from "#/types/generated/EditPlace";
import type { KeyEdits } from "#/types/generated/KeyEdits";
import type { OutdatedEdit } from "#/types/generated/OutdatedEdit";
import type { Proposal } from "#/types/generated/Proposal";
import type { TranslatorLine } from "#/types/generated/TranslatorLine";
import type { TranslatorsMe } from "#/types/generated/TranslatorsMe";
import { StoryText } from "./StoryText";
import { useStoryNames } from "./storyNames";
import { TextDiff } from "./TextDiff";
import { TokenEditor } from "./TokenEditor";
import type { messages } from "./translators.messages";
import { gameWhere } from "./where";

type T = TypedT<typeof messages>;

/** More places than this: a proposal for all of them is confirmed with the list (the backend asks the same). */
export const CONFIRM_PLACES = 1;

/**
 * The tones of the variants and comments, from the project's tokens: the
 * proposals in the info blue (agreed), Sova in the physical-damage tan, the AI
 * in the arts violet, the dictionary under a plugin and outdated decisions in
 * the warning amber, approved in success green, remarks muted, translator's
 * notes in the healing mint with their own icon.
 */
export const TONE = {
    approved: "var(--success)",
    proposal: "var(--info)",
    sova: "var(--dmg-physic)",
    ai: "var(--dmg-magic)",
    dict: "var(--warning)",
    outdated: "var(--warning)",
    remark: "var(--muted-foreground)",
    note: "var(--dmg-heal)",
} as const;

/** The status dot's tone; `unchecked` shows none. */
const STATUS_TONE: Record<string, string | undefined> = {
    proposed: "var(--info)",
    approved: "var(--success)",
    outdated: "var(--warning)",
};

export function placeOf(storyId: string, line: TranslatorLine): EditPlace {
    return { storyId, line: line.line, part: line.part };
}

function samePlace(a: EditPlace, b: EditPlace): boolean {
    return a.storyId === b.storyId && a.line === b.line && a.part === b.part;
}

/** Line breaks as LF, trimmed: the way the backend stores a text. */
export function cleanText(s: string): string {
    return s.replaceAll("\r\n", "\n").replaceAll("\r", "\n").trim();
}

function hasRole(me: TranslatorsMe, role: string): boolean {
    return me.roles.includes(role) || me.roles.includes("admin");
}

/** The lead (editor): approves, sees everyone's edits, resolves any remark. */
export function isLead(me: TranslatorsMe): boolean {
    return hasRole(me, "editor");
}

/** Exports the approved entries for the game: the lead or the pack's maintainer. */
export function exportsForGame(me: TranslatorsMe): boolean {
    return hasRole(me, "editor") || hasRole(me, "maintainer");
}

/** Marks what went out in a pack version: the pack's maintainer. */
export function marksReleases(me: TranslatorsMe): boolean {
    return hasRole(me, "maintainer");
}

/** Run an edit, then refresh every story's lines and My edits; a failure is a toast in the section's words. */
export function useEditMutation<V, R = unknown>(fn: (v: V) => Promise<R>, failed: string) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: fn,
        onSuccess: () => Promise.all(EDIT_QUERY_ROOTS.map((queryKey) => queryClient.invalidateQueries({ queryKey: [...queryKey] }))),
        onError: () => toastManager.add({ title: failed, type: "error" }),
    });
}

/** The proposals that show at a place: those for every place, and those for this place only. */
export function proposalsAt(edits: KeyEdits | undefined, place: EditPlace): Proposal[] {
    return (edits?.proposals ?? []).filter((p) => !p.context || samePlace(p.place, place));
}

/** The comments that show at a place: remarks and notes on the entry (a note on a one-place proposal goes with it). */
function commentsAt(edits: KeyEdits | undefined, place: EditPlace): EditComment[] {
    const shown = new Set(proposalsAt(edits, place).map((p) => p.id));
    const all = new Set((edits?.proposals ?? []).map((p) => p.id));
    return (edits?.comments ?? []).filter((c) => c.variant !== "proposal" || c.proposalId === null || !all.has(c.proposalId) || shown.has(c.proposalId));
}

/** The quoted fragments of the open remarks and the notes at a place, to highlight (a resolved remark no longer is). */
export function quotesAt(edits: KeyEdits | undefined, place: EditPlace): string[] {
    return commentsAt(edits, place)
        .filter((c) => c.kind === "note" || !c.resolvedBy)
        .map((c) => c.quote)
        .filter((q): q is string => Boolean(q));
}

export function StatusDot({ status, t }: { status: string; t: T }): React.ReactElement | null {
    const tone = STATUS_TONE[status];
    if (!tone) return null;
    return (
        <span className="inline-flex items-center" title={t(`status.${status}` as "status.proposed")}>
            <span className="size-2 rounded-full bg-(--tone)" style={{ "--tone": tone } as React.CSSProperties} aria-hidden="true" />
            <span className="sr-only">{t(`status.${status}` as "status.proposed")}</span>
        </span>
    );
}

function names(people: readonly EditPerson[]): string {
    return people.map((p) => p.name).join(", ");
}

export function Toned({ tone, label, children, actions }: { tone: string; label: React.ReactNode; children?: React.ReactNode; actions?: React.ReactNode }): React.ReactElement {
    return (
        <div className="flex flex-col gap-0.5 border-(--tone) border-s-2 ps-2" style={{ "--tone": tone } as React.CSSProperties}>
            <span className="flex flex-wrap items-center gap-x-1.5 text-subtle-foreground text-xs">{label}</span>
            {children ? <div className="whitespace-pre-line">{children}</div> : null}
            {actions ? <div className="flex flex-wrap gap-1">{actions}</div> : null}
        </div>
    );
}

interface ICommentsProps {
    edits: KeyEdits | undefined;
    place: EditPlace;
    me: TranslatorsMe;
    t: T;
}

function CommentItem({ c, me, t }: { c: EditComment; me: TranslatorsMe; t: T }): React.ReactElement {
    const f = useFormatters();
    const mine = c.author.id === me.userId;
    const note = c.kind === "note";
    const resolve = useEditMutation((resolved: boolean) => resolveCommentFn({ data: { id: c.id, resolved } }), t("comment.failed"));
    const remove = useEditMutation(() => deleteCommentFn({ data: { id: c.id } }), t("comment.failed"));
    const canResolve = !note && (mine || isLead(me));
    return (
        <Toned
            tone={note ? TONE.note : TONE.remark}
            label={
                <>
                    {note ? <NotebookPen className="size-3" aria-hidden="true" /> : <MessageSquare className="size-3" aria-hidden="true" />}
                    <span>{note ? t("note.label") : t("remark.label")}</span>
                    <span>· {c.author.name}</span>
                    <span>· {f.date(c.createdAt * 1000, { dateStyle: "medium" })}</span>
                    {c.resolvedBy ? <span>· {t("comment.resolvedBy", { name: c.resolvedBy.name })}</span> : null}
                </>
            }
            actions={
                canResolve || mine ? (
                    <>
                        {canResolve ? (
                            <Button variant="ghost" size="xs" onClick={() => resolve.mutate(!c.resolvedBy)} disabled={resolve.isPending}>
                                {c.resolvedBy ? <Undo2 aria-hidden="true" /> : <Check aria-hidden="true" />}
                                {c.resolvedBy ? t("comment.reopen") : t("comment.resolve")}
                            </Button>
                        ) : null}
                        {mine ? (
                            <Button variant="ghost" size="xs" onClick={() => remove.mutate(undefined)} disabled={remove.isPending}>
                                <Trash2 aria-hidden="true" />
                                {t("comment.delete")}
                            </Button>
                        ) : null}
                    </>
                ) : undefined
            }
        >
            {c.quote ? <span className="block text-subtle-foreground text-xs">{t("comment.quote", { quote: c.quote })}</span> : null}
            <span className={note ? undefined : "text-muted-foreground"}>{c.text}</span>
        </Toned>
    );
}

/** The translator's notes about one variant, under it. */
export function VariantNotes({ edits, place, me, variant, proposalId, t }: ICommentsProps & { variant: "game" | "sova" | "ai" | "proposal"; proposalId?: number }): React.ReactElement | null {
    const list = commentsAt(edits, place).filter((c) => c.kind === "note" && c.variant === variant && (variant !== "proposal" || c.proposalId === proposalId));
    if (list.length === 0) return null;
    return (
        <div className="ms-3 flex flex-col gap-1">
            {list.map((c) => (
                <CommentItem key={c.id} c={c} me={me} t={t} />
            ))}
        </div>
    );
}

/**
 * The line's notes (on the line, or on a variant that is gone: shown with
 * its text as it was) and remarks, the resolved ones folded under one line.
 * Nothing here is ever removed by an approval, a new variant or outdating.
 */
export function LineComments({ edits, place, me, t }: ICommentsProps): React.ReactElement | null {
    const proposals = new Set(proposalsAt(edits, place).map((p) => p.id));
    const all = commentsAt(edits, place);
    const orphan = (c: EditComment) => c.variant === "proposal" && (c.proposalId === null || !proposals.has(c.proposalId));
    const notes = all.filter((c) => c.kind === "note" && (c.variant === null || orphan(c)));
    const remarks = all.filter((c) => c.kind !== "note");
    const open = remarks.filter((c) => !c.resolvedBy);
    const resolved = remarks.filter((c) => c.resolvedBy);
    if (notes.length + remarks.length === 0) return null;
    return (
        <>
            {notes.map((c) => (
                <div key={c.id} className="flex flex-col gap-0.5">
                    {orphan(c) && c.variantText ? <span className="text-subtle-foreground text-xs">{t("note.forGone", { text: c.variantText })}</span> : null}
                    <CommentItem c={c} me={me} t={t} />
                </div>
            ))}
            {open.map((c) => (
                <CommentItem key={c.id} c={c} me={me} t={t} />
            ))}
            {resolved.length > 0 ? (
                <Collapsible>
                    <CollapsibleTrigger render={<Button variant="ghost" size="xs" className="text-muted-foreground" />}>{t("comment.resolvedCount", { count: resolved.length })}</CollapsibleTrigger>
                    <CollapsiblePanel className="flex flex-col gap-1 pt-1">
                        {resolved.map((c) => (
                            <CommentItem key={c.id} c={c} me={me} t={t} />
                        ))}
                    </CollapsiblePanel>
                </Collapsible>
            ) : null}
        </>
    );
}

/** Who agrees with Sova's or the AI's text at this place. */
export function placeVotes(edits: KeyEdits | undefined, place: EditPlace, variant: "sova" | "ai"): EditPerson[] {
    return edits?.placeAgreements.find((a) => a.variant === variant && samePlace(a.place, place))?.people ?? [];
}

interface IAgreeProps {
    place: EditPlace;
    variant: AgreeVariant;
    proposalId?: number;
    people: readonly EditPerson[];
    me: TranslatorsMe;
    /** One's own proposal: the count only. */
    own?: boolean;
    t: T;
}

/**
 * "Agree" with one variant: a thumb with the count, who in its tooltip; mine
 * pressed. With no votes it shows on the cell's hover and keyboard focus only
 * (always on a touch screen), so the dense table stays quiet.
 */
export function AgreeButton({ place, variant, proposalId, people, me, own = false, t }: IAgreeProps): React.ReactElement | null {
    const f = useFormatters();
    const mine = people.some((p) => p.id === me.userId);
    const count = people.length;
    const vote = useEditMutation((agree: boolean) => setAgreementFn({ data: { place, variant, proposalId: proposalId ?? null, agree } }), t("agree.failed"));
    if (own && count === 0) return null;
    const title = count > 0 ? t("agree.who", { count, names: names(people) }) : t("agree.hint");
    return (
        <Button
            variant="ghost"
            size="xs"
            className={cn("h-auto gap-0.5 px-1 py-0 align-baseline text-xs", count === 0 && "opacity-0 pointer-coarse:opacity-100 focus-visible:opacity-100 group-hover/cell:opacity-100")}
            aria-pressed={mine}
            data-pressed={mine ? "" : undefined}
            disabled={own || vote.isPending}
            title={title}
            onClick={() => vote.mutate(!mine)}
        >
            <ThumbsUp className={cn("size-3", mine && "fill-current")} aria-hidden="true" />
            {count > 0 ? f.number(count) : null}
            <span className="sr-only">{own ? title : t("agree.label")}</span>
        </Button>
    );
}

/** The approved variant: above everything else in the cell. */
interface IApproveProps {
    place: EditPlace;
    variant: AgreeVariant;
    proposalId?: number;
    me: TranslatorsMe;
    /** This variant is the approved one: the button withdraws. */
    approved: boolean;
    t: T;
}

/**
 * The lead's "Approve" on a variant (shown on the cell's hover and keyboard
 * focus), or "Withdraw" on the approved one. Nothing for other members.
 */
export function ApproveButton({ place, variant, proposalId, me, approved, t }: IApproveProps): React.ReactElement | null {
    const set = useEditMutation((approve: boolean) => setApprovalFn({ data: { place, variant, proposalId: proposalId ?? null, approve } }), t("approve.failed"));
    if (!isLead(me)) return null;
    return (
        <Button variant="ghost" size="xs" className={cn("h-auto gap-0.5 px-1 py-0 align-baseline text-xs", !approved && "opacity-0 pointer-coarse:opacity-100 focus-visible:opacity-100 group-hover/cell:opacity-100")} disabled={set.isPending} onClick={() => set.mutate(!approved)}>
            {approved ? <Undo2 className="size-3" aria-hidden="true" /> : <CheckCheck className="size-3" aria-hidden="true" />}
            {approved ? t("approve.withdraw") : t("approve.button")}
        </Button>
    );
}

/** The approved variant of the entry, if any: `game`, `sova`, `ai` or `proposal:<id>`. */
export function approvedVariant(edits: KeyEdits | undefined): string | null {
    const a = edits?.approval;
    if (!a) return null;
    return a.variant === "proposal" ? `proposal:${a.proposalId}` : a.variant;
}

/** The screen-reader words of a comparison. */
export function diffLabels(t: T): { added: string; removed: string } {
    return { added: t("diff.added"), removed: t("diff.removed") };
}

interface IApprovedProps {
    edits: KeyEdits | undefined;
    place: EditPlace;
    me: TranslatorsMe;
    nickname: string;
    marks?: React.ReactNode;
    /** Compare is on: the approved text as its changes against this (the game's Russian). */
    compareWith?: string | null;
    t: T;
}

export function ApprovedVariant({ edits, place, me, nickname, marks, compareWith, t }: IApprovedProps): React.ReactElement | null {
    const f = useFormatters();
    const a = edits?.approval;
    if (!a) return null;
    const when = f.date(a.at * 1000, { dateStyle: "medium" });
    return (
        <Toned
            tone={TONE.approved}
            label={
                <>
                    {a.variant === "game" ? t("approved.game", { name: a.by.name, date: when }) : t("approved.label", { name: a.by.name, date: when })}
                    {marks}
                    {a.releasedIn ? (
                        <Badge variant="success" size="sm">
                            {t("release.in", { version: a.releasedIn })}
                        </Badge>
                    ) : null}
                    <ApproveButton place={place} variant={a.variant as AgreeVariant} proposalId={a.proposalId ?? undefined} me={me} approved t={t} />
                </>
            }
        >
            {compareWith && a.variant !== "game" ? <TextDiff before={compareWith} after={a.text} labels={diffLabels(t)} /> : <StoryText text={a.text} nickname={nickname} />}
        </Toned>
    );
}

interface IProposalsProps extends ICommentsProps {
    nickname: string;
    onEdit: (p: Proposal) => void;
    /** Compare is on: each proposal as its changes against the text one step back. */
    compare?: boolean;
    /** The game's Russian: one step back for a proposal never rewritten. */
    ruGame?: string | null;
    /** The proposal's automatic marks, shown in its label line. */
    marks?: (p: Proposal) => React.ReactNode;
}

export function Proposals({ edits, place, me, nickname, onEdit, marks, compare = false, ruGame = null, t }: IProposalsProps): React.ReactElement | null {
    const f = useFormatters();
    const remove = useEditMutation((id: number) => deleteProposalFn({ data: { id } }), t("edit.failedDelete"));
    const list = proposalsAt(edits, place);
    if (list.length === 0) return null;
    return (
        <>
            {list.map((p) => {
                const mine = p.author.id === me.userId;
                // One step back: its previous version, else the game's Russian.
                const before = compare ? (p.previousText ?? ruGame) : null;
                return (
                    <div key={p.id} className="flex flex-col gap-1">
                        <Toned
                            tone={TONE.proposal}
                            label={
                                <>
                                    <span>{mine ? t("proposal.mine") : t("proposal.by", { name: p.author.name })}</span>
                                    <span>· {f.date((p.editedAt ?? p.createdAt) * 1000, { dateStyle: "medium" })}</span>
                                    {p.editedAt !== null ? <span>· {t("proposal.edited")}</span> : null}
                                    {p.source !== "own" ? <span>· {t(`proposal.from.${p.source}` as "proposal.from.sova")}</span> : null}
                                    {p.context ? (
                                        <Badge variant="info" size="sm">
                                            {t("proposal.contextBadge")}
                                        </Badge>
                                    ) : null}
                                    <AgreeButton place={place} variant="proposal" proposalId={p.id} people={p.agreements} me={me} own={mine} t={t} />
                                    {p.context && p.approvedAt !== null ? (
                                        <Badge variant="success" size="sm">
                                            {p.releasedIn ? t("release.in", { version: p.releasedIn }) : t("approved.here")}
                                        </Badge>
                                    ) : null}
                                    <ApproveButton place={place} variant="proposal" proposalId={p.id} me={me} approved={p.context ? p.approvedAt !== null : edits?.approval?.proposalId === p.id} t={t} />
                                    {marks?.(p)}
                                    {before ? <span>· {p.previousText ? t("diff.vsPrevious") : t("diff.vsGame")}</span> : null}
                                </>
                            }
                            actions={
                                mine ? (
                                    <>
                                        <Button variant="ghost" size="xs" onClick={() => onEdit(p)}>
                                            <PenLine aria-hidden="true" />
                                            {t("proposal.edit")}
                                        </Button>
                                        <Button variant="ghost" size="xs" onClick={() => remove.mutate(p.id)} disabled={remove.isPending}>
                                            <Trash2 aria-hidden="true" />
                                            {t("proposal.delete")}
                                        </Button>
                                    </>
                                ) : undefined
                            }
                        >
                            {before ? <TextDiff before={before} after={p.text} labels={diffLabels(t)} /> : <StoryText text={p.text} nickname={nickname} />}
                        </Toned>
                        <VariantNotes edits={edits} place={place} me={me} variant="proposal" proposalId={p.id} t={t} />
                    </div>
                );
            })}
        </>
    );
}

/** Decisions left at this place for an English that has changed since. Shown always, never removed. */
export function OutdatedHere({ items, nickname, t }: { items: readonly OutdatedEdit[]; nickname: string; t: T }): React.ReactElement | null {
    if (items.length === 0) return null;
    return (
        <>
            {items.map((o) => (
                <Toned
                    key={`${o.kind}-${o.editKey}-${o.author.id}`}
                    tone={TONE.outdated}
                    label={
                        <>
                            <Badge variant="warning" size="sm">
                                {t("status.outdated")}
                            </Badge>
                            <span>{t(o.kind === "approval" ? "outdated.approval" : "outdated.proposal", { name: o.author.name })}</span>
                        </>
                    }
                >
                    <StoryText text={o.text} nickname={nickname} />
                    <span className="block text-subtle-foreground text-xs">{t("outdated.was", { en: o.editKey })}</span>
                </Toned>
            ))}
        </>
    );
}

/** What the in-row editor is open for. */
export type EditorState = { kind: "proposal"; text: string; source: ProposalSource; context: boolean } | { kind: "remark" | "note"; quote: string | null };

/** The text selected inside `within`, if any: a comment quotes it. */
function selectedIn(within: Element | null): string | null {
    const sel = typeof window === "undefined" ? null : window.getSelection();
    if (!sel || sel.isCollapsed || !within || !sel.anchorNode || !within.contains(sel.anchorNode)) return null;
    const text = sel.toString().trim();
    return text ? text : null;
}

/**
 * The Russian cell's actions, at its top right on hover or keyboard focus
 * (always on a touch screen): agree with and (the lead) approve the game's
 * text, propose, remark, note. The game's text's buttons live here, not at
 * the end of the text, which the panel would cover on a long line. A remark or note quotes
 * the text selected in the row (read before the click can clear it). History
 * opens the history of the line's translation (LineHistory.tsx).
 */
export function CellActions({ line, onOpen, onHistory, game, t }: { line: TranslatorLine; onOpen: (state: EditorState) => void; onHistory: () => void; game?: React.ReactNode; t: T }): React.ReactElement {
    const comment = (kind: "remark" | "note", from: HTMLElement) => onOpen({ kind, quote: selectedIn(from.closest("tr")) });
    const keys = (kind: "remark" | "note") => (e: React.KeyboardEvent<HTMLButtonElement>) => {
        if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            comment(kind, e.currentTarget);
        }
    };
    return (
        <div className="absolute end-0 top-0 z-10 flex items-center gap-0.5 rounded-lg border bg-popover p-0.5 opacity-0 pointer-coarse:opacity-100 shadow-xs transition-opacity focus-within:opacity-100 group-hover/cell:opacity-100">
            {game}
            {line.editable ? (
                <Button variant="ghost" size="xs" onClick={() => onOpen({ kind: "proposal", text: line.ruGame ?? "", source: "own", context: false })}>
                    <PenLine aria-hidden="true" />
                    {t("action.propose")}
                </Button>
            ) : null}
            <Button
                variant="ghost"
                size="xs"
                onPointerDown={(e) => {
                    e.preventDefault();
                    comment("remark", e.currentTarget);
                }}
                onKeyDown={keys("remark")}
            >
                <MessageSquare aria-hidden="true" />
                {t("action.remark")}
            </Button>
            <Button
                variant="ghost"
                size="xs"
                onPointerDown={(e) => {
                    e.preventDefault();
                    comment("note", e.currentTarget);
                }}
                onKeyDown={keys("note")}
            >
                <NotebookPen aria-hidden="true" />
                {t("action.note")}
            </Button>
            <Button variant="ghost" size="xs" onClick={onHistory}>
                <History aria-hidden="true" />
                {t("action.history")}
            </Button>
        </div>
    );
}

/** Every place of the entry, with the lines before and after, as links. */
export function PlacesList({ place, nickname, t }: { place: EditPlace; nickname: string; t: T }): React.ReactElement {
    const query = useQuery(placesQueryOptions(place));
    const nameOf = useStoryNames();
    if (query.isError) return <LoadFailed layout="inline" message={t("places.load")} onRetry={() => query.refetch()} />;
    if (!query.data) return <Skeleton className="h-16 w-full" />;
    const { places, total } = query.data;
    return (
        <div className="flex flex-col gap-1">
            <ul className="flex max-h-72 flex-col gap-2 overflow-y-auto text-xs">
                {places.map((p) => {
                    // A field of the game data: where it stands, in words.
                    if (p.section !== "story")
                        return (
                            <li key={`${p.section}-${p.objectEn}-${p.field}-${p.idx}`} className="flex flex-col gap-0.5 border-s-2 ps-2">
                                <span className="font-medium">{gameWhere(p.section, p.objectEn, p.field, p.idx, t)}</span>
                            </li>
                        );
                    const n = nameOf(p.storyId);
                    const here = samePlace(p, place);
                    return (
                        <li key={`${p.storyId}-${p.line}-${p.part}`} className="flex flex-col gap-0.5 border-s-2 ps-2">
                            <span className="font-medium">
                                {n ? (
                                    <Link to="/translators" search={{ group: n.groupId, story: p.storyId }} hash={`n-${p.ord + 1}`} className="underline-offset-4 hover:underline">
                                        {t("places.item", { group: n.group, story: n.story, n: p.ord + 1 })}
                                    </Link>
                                ) : (
                                    t("places.itemBare", { story: p.storyId, n: p.ord + 1 })
                                )}
                                {here ? <span className="font-normal text-subtle-foreground"> · {t("places.here")}</span> : null}
                            </span>
                            {p.before ? (
                                <span className="text-subtle-foreground">
                                    {t("places.before")} <StoryText text={p.before} nickname={nickname} />
                                </span>
                            ) : null}
                            {p.after ? (
                                <span className="text-subtle-foreground">
                                    {t("places.after")} <StoryText text={p.after} nickname={nickname} />
                                </span>
                            ) : null}
                        </li>
                    );
                })}
            </ul>
            {total > places.length ? <span className="text-subtle-foreground text-xs">{t("places.more", { count: total - places.length })}</span> : null}
        </div>
    );
}

interface ISaveDialogProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    place: EditPlace;
    places: number;
    /** Offer "only this place" when the line allows it (`contextAllowed`: every story line); a rewrite keeps its scope. Else the "changes everywhere" note. */
    allowContext: boolean;
    pending: boolean;
    onSave: (context: boolean) => void;
    nickname: string;
    t: T;
}

/** Saving a proposal for an entry in several places: for all of them (with the list and a confirmation) or for this place only. */
export function SaveDialog({ open, onOpenChange, place, places, allowContext, pending, onSave, nickname, t }: ISaveDialogProps): React.ReactElement {
    const scopeId = useId();
    const [context, setContext] = useState(false);
    const [understood, setUnderstood] = useState(false);
    const forAll = !context;
    return (
        <AlertDialog open={open} onOpenChange={onOpenChange}>
            <AlertDialogPopup className="max-w-2xl">
                <AlertDialogHeader>
                    <AlertDialogTitle>{t("save.title", { count: places })}</AlertDialogTitle>
                    <AlertDialogDescription>{t("save.proposalOnly")}</AlertDialogDescription>
                    {allowContext ? (
                        <RadioGroup aria-labelledby={scopeId} value={context ? "here" : "all"} onValueChange={(v) => setContext(v === "here")} className="gap-2 pt-2">
                            <span id={scopeId} className="sr-only">
                                {t("save.scope")}
                            </span>
                            {/* biome-ignore lint/a11y/noLabelWithoutControl: the kit Radio inside is the control */}
                            <label className="flex items-start gap-2 text-sm">
                                <Radio value="all" />
                                <span className="flex flex-col">
                                    <span className="font-medium">{t("save.all", { count: places })}</span>
                                    <span className="text-muted-foreground text-xs">{t("save.allHint")}</span>
                                </span>
                            </label>
                            {/* biome-ignore lint/a11y/noLabelWithoutControl: the kit Radio inside is the control */}
                            <label className="flex items-start gap-2 text-sm">
                                <Radio value="here" />
                                <span className="flex flex-col">
                                    <span className="font-medium">{t("save.here")}</span>
                                    <span className="text-muted-foreground text-xs">{t("save.hereHint")}</span>
                                </span>
                            </label>
                        </RadioGroup>
                    ) : (
                        <p className="pt-2 font-medium text-sm">{t("save.hereShort", { count: places })}</p>
                    )}
                    {forAll ? (
                        <div className="flex flex-col gap-2 pt-2">
                            <PlacesList place={place} nickname={nickname} t={t} />
                            {/* biome-ignore lint/a11y/noLabelWithoutControl: the kit Checkbox inside is the control */}
                            <label className="flex items-center gap-2 font-medium text-sm">
                                <Checkbox checked={understood} onCheckedChange={(v) => setUnderstood(v === true)} />
                                {t("save.understand", { count: places })}
                            </label>
                        </div>
                    ) : null}
                </AlertDialogHeader>
                <AlertDialogFooter>
                    <AlertDialogClose render={<Button variant="ghost" />}>{t("editor.cancel")}</AlertDialogClose>
                    <Button onClick={() => onSave(context)} disabled={pending || (forAll && !understood)}>
                        {pending ? <Spinner /> : null}
                        {forAll ? t("save.saveAll", { count: places }) : t("save.saveHere")}
                    </Button>
                </AlertDialogFooter>
            </AlertDialogPopup>
        </AlertDialog>
    );
}

/** "Also in N other places" under the editor, with the list on demand. */
function PlacesNote({ line, place, nickname, t }: { line: TranslatorLine; place: EditPlace; nickname: string; t: T }): React.ReactElement | null {
    if (line.places < 2) return null;
    return (
        <Collapsible>
            <span className="flex flex-wrap items-center gap-1 text-warning-foreground text-xs">
                {t("places.note", { count: line.places - 1 })}
                <CollapsibleTrigger render={<Button variant="link" size="xs" className="h-auto px-0 text-xs" />}>{t("places.show")}</CollapsibleTrigger>
            </span>
            <CollapsiblePanel className="pt-1">
                <PlacesList place={place} nickname={nickname} t={t} />
            </CollapsiblePanel>
        </Collapsible>
    );
}

interface IEditorProps {
    line: TranslatorLine;
    place: EditPlace;
    initial: string;
    source: ProposalSource;
    /** Editing a one-place proposal: it stays one. */
    context: boolean;
    nickname: string;
    onClose: () => void;
    t: T;
}

/** The in-row editor of my proposal: Ctrl+Enter saves, Esc cancels; `{@nickname}` is one chip. */
export function ProposalEditor({ line, place, initial, source, context, nickname, onClose, t }: IEditorProps): React.ReactElement {
    const id = useId();
    const hintId = useId();
    const [text, setText] = useState(initial);
    const [asking, setAsking] = useState(false);
    const save = useEditMutation((c: { context: boolean; confirmMany: boolean }) => createProposalFn({ data: { place, text, source, context: c.context, confirmMany: c.confirmMany } }), t("edit.failedSave"));
    const clean = cleanText(text);
    const sameAsGame = line.ruGame !== null && clean === cleanText(line.ruGame);
    const canSave = clean.length > 0 && !sameAsGame && !save.isPending;
    const several = line.places > CONFIRM_PLACES;

    const done = { onSuccess: onClose };
    const submit = () => {
        if (!canSave) return;
        if (several && !context) setAsking(true);
        else save.mutate({ context, confirmMany: false }, done);
    };

    return (
        <div className="flex flex-col gap-1.5">
            <label htmlFor={id} className="text-subtle-foreground text-xs">
                {source === "own" ? t("editor.label") : t(`editor.label.${source}` as "editor.label.sova")}
            </label>
            <TokenEditor id={id} initial={initial} nickname={nickname} onChange={setText} onSubmit={submit} onCancel={onClose} insertLabel={t("editor.insertNickname")} describedBy={hintId} />
            <span id={hintId} className="text-subtle-foreground text-xs">
                {t("save.proposalOnly")}
            </span>
            {line.ruGameMatch === "lines" || line.en.includes("\n") ? <span className="text-subtle-foreground text-xs">{t("editor.wholeEntry")}</span> : null}
            {sameAsGame ? <span className="text-subtle-foreground text-xs">{t("editor.sameAsGame")}</span> : null}
            {context ? <span className="text-subtle-foreground text-xs">{t("editor.contextOnly")}</span> : <PlacesNote line={line} place={place} nickname={nickname} t={t} />}
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
            {asking ? <SaveDialog open={asking} onOpenChange={setAsking} place={place} places={line.places} allowContext={line.contextAllowed} pending={save.isPending} onSave={(c) => save.mutate({ context: c, confirmMany: !c }, done)} nickname={nickname} t={t} /> : null}
        </div>
    );
}

interface ICommentEditorProps {
    line: TranslatorLine;
    place: EditPlace;
    edits: KeyEdits | undefined;
    kind: "remark" | "note";
    quote: string | null;
    onClose: () => void;
    t: T;
}

const LINE = "__line__";

/** The in-row box for a remark or a translator's note (a note may be about one variant). */
export function CommentEditor({ line, place, edits, kind, quote: initialQuote, onClose, t }: ICommentEditorProps): React.ReactElement {
    const id = useId();
    const variantId = useId();
    const [text, setText] = useState("");
    const [quote, setQuote] = useState(initialQuote);
    const [variant, setVariant] = useState<string>(LINE);
    const options: { value: string; label: string }[] = [{ value: LINE, label: t("note.aboutLine") }];
    if (line.ruGame) options.push({ value: "game", label: t("note.aboutGame") });
    for (const p of proposalsAt(edits, place)) options.push({ value: `proposal:${p.id}`, label: t("note.aboutProposal", { name: p.author.name }) });
    if (line.sova) options.push({ value: "sova", label: t("note.aboutSova") });
    if (line.ai) options.push({ value: "ai", label: t("note.aboutAi") });
    const save = useEditMutation(() => {
        const [v, pid] = variant.split(":");
        return createCommentFn({ data: { place, text, quote, kind, variant: variant === LINE ? null : v, proposalId: pid ? Number(pid) : null } });
    }, t("comment.failed"));
    const submit = () => {
        if (cleanText(text).length === 0 || save.isPending) return;
        save.mutate(undefined, { onSuccess: onClose });
    };
    return (
        <div className="flex flex-col gap-1.5">
            <label htmlFor={id} className="text-subtle-foreground text-xs">
                {kind === "note" ? t("note.editorLabel") : t("remark.editorLabel")}
            </label>
            {quote ? (
                <span className="flex flex-wrap items-center gap-1 text-xs">
                    <mark className="rounded-sm bg-warning/20 text-foreground">{quote}</mark>
                    <Button variant="ghost" size="xs" onClick={() => setQuote(null)}>
                        {t("comment.dropQuote")}
                    </Button>
                </span>
            ) : (
                <span className="text-subtle-foreground text-xs">{t("comment.quoteHint")}</span>
            )}
            {kind === "note" && options.length > 1 ? (
                <div className="flex flex-col gap-1 sm:w-80">
                    <label htmlFor={variantId} className="text-subtle-foreground text-xs">
                        {t("note.about")}
                    </label>
                    <Select value={variant} onValueChange={(v: string | null) => setVariant(v ?? LINE)}>
                        <SelectTrigger id={variantId} size="sm">
                            <SelectValue>{() => options.find((o) => o.value === variant)?.label}</SelectValue>
                        </SelectTrigger>
                        <SelectContent>
                            {options.map((o) => (
                                <SelectItem key={o.value} value={o.value}>
                                    {o.label}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                </div>
            ) : null}
            <Textarea
                id={id}
                size="sm"
                value={text}
                onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => setText(e.target.value)}
                onKeyDown={(e: React.KeyboardEvent<HTMLTextAreaElement>) => {
                    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
                        e.preventDefault();
                        submit();
                    } else if (e.key === "Escape") {
                        e.preventDefault();
                        onClose();
                    }
                }}
                autoFocus
            />
            <div className="flex flex-wrap items-center gap-2">
                <Button size="sm" onClick={submit} disabled={cleanText(text).length === 0 || save.isPending}>
                    {save.isPending ? <Spinner /> : null}
                    {kind === "note" ? t("note.save") : t("remark.save")}
                </Button>
                <Button size="sm" variant="ghost" onClick={onClose}>
                    {t("editor.cancel")}
                </Button>
                <span className="text-subtle-foreground text-xs">{t("editor.keys")}</span>
            </div>
        </div>
    );
}
