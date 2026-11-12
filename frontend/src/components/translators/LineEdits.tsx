/**
 * Editing one story line: the approved variant,
 * the translators' proposals, remarks and translator's notes under the
 * Russian, the cell's actions on hover (propose, remark, note, history, and
 * for the lead approve or withdraw), the in-row editor and the save dialog: an
 * entry in more than one place is saved for all its places (with the list and
 * a confirmation) or for this place only (a context translation).
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Check, CircleHelp, History, MessageSquare, NotebookPen, PenLine, ThumbsUp, Trash2, Undo2, X } from "lucide-react";
import type React from "react";
import { useId, useState } from "react";
import { AlertDialog, AlertDialogClose, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogPopup, AlertDialogTitle } from "#/components/ui/alert-dialog";
import { Badge } from "#/components/ui/badge";
import { Button } from "#/components/ui/button";
import { Checkbox } from "#/components/ui/checkbox";
import { Collapsible, CollapsiblePanel, CollapsibleTrigger } from "#/components/ui/collapsible";
import { Dialog, DialogHeader, DialogPanel, DialogPopup, DialogTitle } from "#/components/ui/dialog";
import { InfoHint } from "#/components/ui/info-hint";
import { LoadFailed } from "#/components/ui/load-failed";
import { Popover, PopoverPopup, PopoverTrigger } from "#/components/ui/popover";
import { Radio, RadioGroup } from "#/components/ui/radio-group";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "#/components/ui/select";
import { Separator } from "#/components/ui/separator";
import { Skeleton } from "#/components/ui/skeleton";
import { Spinner } from "#/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "#/components/ui/table";
import { Textarea } from "#/components/ui/textarea";
import { toastManager } from "#/components/ui/toast";
import { useFormatters, useLocale } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { type AgreeVariant, createCommentFn, createProposalFn, decideProposalsFn, deleteCommentFn, deleteProposalFn, EDIT_QUERY_ROOTS, type ProposalDecision, type ProposalSource, placesQueryOptions, resolveCommentFn, setAgreementFn, setApprovalFn } from "#/lib/translators/api";
import { cn } from "#/lib/utils";
import type { EditComment } from "#/types/generated/EditComment";
import type { EditPerson } from "#/types/generated/EditPerson";
import type { EditPlace } from "#/types/generated/EditPlace";
import type { GameHere } from "#/types/generated/GameHere";
import type { KeyEdits } from "#/types/generated/KeyEdits";
import type { OutdatedEdit } from "#/types/generated/OutdatedEdit";
import type { Proposal } from "#/types/generated/Proposal";
import type { TranslatorLine } from "#/types/generated/TranslatorLine";
import type { TranslatorsMe } from "#/types/generated/TranslatorsMe";
import { tagDiff, tagPairs, tokensIn } from "./markup";
import { GamePlace } from "./PlaceWords";
import { StoryText } from "./StoryText";
import { useStoryNames } from "./storyNames";
import { TextDiff } from "./TextDiff";
import { type ITagButton, type ITokenLabels, type TagLookup, TokenEditor } from "./TokenEditor";
import { spanLook, tagOf, tagWords, useGameTags } from "./tags";
import type { messages } from "./translators.messages";
import { editorLabel, fromWords, TRANSLATOR_SOURCES, translatorName } from "./translatorVariants";

type T = TypedT<typeof messages>;

/** More places than this: a proposal for all of them is confirmed with the list (the backend asks the same). */
export const CONFIRM_PLACES = 1;

/**
 * The tones of the variants and comments, from the project's tokens: the
 * proposals in the info blue (agreed), a translator's variant in the physical-damage tan, the AI
 * in the arts violet, the dictionary under a plugin and outdated decisions in
 * the warning amber, approved in success green, remarks muted, translator's
 * notes in the healing mint with their own icon.
 */
export const TONE = {
    approved: "var(--success)",
    proposal: "var(--info)",
    translator: "var(--dmg-physic)",
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

/** A translator (or an admin): approves and rejects, sees everyone's edits, resolves any remark. */
export function isLead(me: TranslatorsMe): boolean {
    return hasRole(me, "translator");
}

/** Exports the approved entries for the game: a translator or an admin. */
export function exportsForGame(me: TranslatorsMe): boolean {
    return hasRole(me, "translator");
}

/** Marks what went out in a pack version: an admin. */
export function marksReleases(me: TranslatorsMe): boolean {
    return hasRole(me, "admin");
}

/** The admin panel: an admin. */
export function isAdmin(me: TranslatorsMe): boolean {
    return me.roles.includes("admin");
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

/** Who agrees with a translator's or the AI's text at this place. */
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
    /** In an action bar that shows on hover itself: never hidden on its own. */
    inBar?: boolean;
    t: T;
}

/**
 * "Agree" with one variant: a thumb with the count, who in its tooltip; mine
 * pressed. With no votes it shows on the cell's hover and keyboard focus only
 * (always on a touch screen), so the dense table stays quiet.
 */
export function AgreeButton({ place, variant, proposalId, people, me, own = false, inBar = false, t }: IAgreeProps): React.ReactElement | null {
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
            className={cn("h-auto gap-0.5 px-1 py-0.5 align-baseline text-muted-foreground text-xs", count === 0 && !inBar && "opacity-0 pointer-coarse:opacity-100 focus-visible:opacity-100 group-hover/cell:opacity-100")}
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

interface IApproveProps {
    place: EditPlace;
    variant: AgreeVariant;
    proposalId?: number;
    me: TranslatorsMe;
    /** This variant is the approved one: the button withdraws. */
    approved: boolean;
    /** The game's text with proposals: "Approve" (keep as in the game) asks first. */
    confirmProposals?: number;
    t: T;
}

/** One of the main decisions of a block: its icon and its word. */
function MainButton({ icon, label, onClick, disabled, destructive = false }: { icon: React.ReactNode; label: string; onClick: () => void; disabled?: boolean; destructive?: boolean }): React.ReactElement {
    return (
        <Button variant="ghost" size="xs" className={cn("h-auto gap-1 px-1.5 py-0.5 text-xs", destructive && "text-destructive-foreground")} disabled={disabled} onClick={onClick}>
            {icon}
            {label}
        </Button>
    );
}

/**
 * A translator's "Approve" (✓) on a variant, or "Withdraw" on the approved
 * one. On the game's text it means "keep as in the game" and asks first when
 * the line has proposals. Nothing for a user.
 */
export function ApproveButton({ place, variant, proposalId, me, approved, confirmProposals = 0, t }: IApproveProps): React.ReactElement | null {
    const [asking, setAsking] = useState(false);
    const set = useEditMutation((approve: boolean) => setApprovalFn({ data: { place, variant, proposalId: proposalId ?? null, approve } }), t("approve.failed"));
    if (!isLead(me)) return null;
    if (approved) return <MainButton icon={<Undo2 className="size-3" aria-hidden="true" />} label={t("approve.withdraw")} disabled={set.isPending} onClick={() => set.mutate(false)} />;
    return (
        <>
            <MainButton icon={<Check className="size-3" aria-hidden="true" />} label={t("approve.button")} disabled={set.isPending} onClick={() => (confirmProposals > 0 ? setAsking(true) : set.mutate(true))} />
            {asking ? (
                <AlertDialog open={asking} onOpenChange={setAsking}>
                    <AlertDialogPopup>
                        <AlertDialogHeader>
                            <AlertDialogTitle>{t("approve.gameTitle")}</AlertDialogTitle>
                            <AlertDialogDescription>{t("approve.gameHint", { count: confirmProposals })}</AlertDialogDescription>
                        </AlertDialogHeader>
                        <AlertDialogFooter>
                            <AlertDialogClose render={<Button variant="ghost" />}>{t("editor.cancel")}</AlertDialogClose>
                            <Button onClick={() => set.mutate(true, { onSuccess: () => setAsking(false) })} disabled={set.isPending}>
                                {set.isPending ? <Spinner /> : <Check aria-hidden="true" />}
                                {t("approve.gameKeep")}
                            </Button>
                        </AlertDialogFooter>
                    </AlertDialogPopup>
                </AlertDialog>
            ) : null}
        </>
    );
}

/** A decision on proposals by id (the Edits list, a proposal's Reject); the toast says how many were skipped. */
export function useDecide(t: T) {
    return useEditMutation(async (v: { ids: number[]; decision: ProposalDecision; reason: string }) => {
        const r = await decideProposalsFn({ data: v });
        if (r.skipped > 0) toastManager.add({ title: t("decide.skipped", { done: r.approved, skipped: r.skipped }), type: "info" });
        return r;
    }, t("decide.failed"));
}

/** "Reject" with an optional reason its author sees: for one proposal or the ones ticked. */
export function RejectDialog({ open, onOpenChange, count, pending, onReject, t }: { open: boolean; onOpenChange: (open: boolean) => void; count: number; pending: boolean; onReject: (reason: string) => void; t: T }): React.ReactElement {
    const id = useId();
    const [reason, setReason] = useState("");
    return (
        <AlertDialog open={open} onOpenChange={onOpenChange}>
            <AlertDialogPopup>
                <AlertDialogHeader>
                    <AlertDialogTitle>{t("decide.rejectTitle", { count })}</AlertDialogTitle>
                    <AlertDialogDescription>{t("decide.rejectHint")}</AlertDialogDescription>
                    <label htmlFor={id} className="pt-2 text-subtle-foreground text-xs">
                        {t("decide.reason")}
                    </label>
                    <Textarea id={id} size="sm" value={reason} onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => setReason(e.target.value)} autoFocus />
                </AlertDialogHeader>
                <AlertDialogFooter>
                    <AlertDialogClose render={<Button variant="ghost" />}>{t("editor.cancel")}</AlertDialogClose>
                    <Button variant="destructive" onClick={() => onReject(reason.trim())} disabled={pending}>
                        {pending ? <Spinner /> : <X aria-hidden="true" />}
                        {t("decide.reject")}
                    </Button>
                </AlertDialogFooter>
            </AlertDialogPopup>
        </AlertDialog>
    );
}

/** A translator's "Reject" (✗) on a proposal, with an optional reason. */
export function RejectButton({ proposalId, me, t }: { proposalId: number; me: TranslatorsMe; t: T }): React.ReactElement | null {
    const [asking, setAsking] = useState(false);
    const decide = useDecide(t);
    if (!isLead(me)) return null;
    return (
        <>
            <MainButton icon={<X className="size-3" aria-hidden="true" />} label={t("decide.reject")} destructive disabled={decide.isPending} onClick={() => setAsking(true)} />
            {asking ? <RejectDialog open={asking} onOpenChange={setAsking} count={1} pending={decide.isPending} onReject={(reason) => decide.mutate({ ids: [proposalId], decision: "reject", reason }, { onSuccess: () => setAsking(false) })} t={t} /> : null}
        </>
    );
}

/** A secondary action: an icon only, gray, its words on hover and for a screen reader. */
export function IconAction({ icon, label, onClick, onPointerDown, onKeyDown }: { icon: React.ReactNode; label: string; onClick?: () => void; onPointerDown?: (e: React.PointerEvent<HTMLButtonElement>) => void; onKeyDown?: (e: React.KeyboardEvent<HTMLButtonElement>) => void }): React.ReactElement {
    return (
        <Button variant="ghost" size="icon-xs" className="size-6 text-muted-foreground hover:text-foreground" title={label} aria-label={label} onClick={onClick} onPointerDown={onPointerDown} onKeyDown={onKeyDown}>
            {icon}
        </Button>
    );
}

/** "?": what each icon of the actions means, on hover. */
export function ActionsHelp({ t }: { t: T }): React.ReactElement {
    const items: [React.ReactNode, string][] = [
        [<Check key="a" className="size-3.5" aria-hidden="true" />, t("help.approve")],
        [<X key="r" className="size-3.5" aria-hidden="true" />, t("help.reject")],
        [<PenLine key="p" className="size-3.5" aria-hidden="true" />, t("help.propose")],
        [<MessageSquare key="m" className="size-3.5" aria-hidden="true" />, t("help.remark")],
        [<NotebookPen key="n" className="size-3.5" aria-hidden="true" />, t("help.note")],
        [<History key="h" className="size-3.5" aria-hidden="true" />, t("help.history")],
        [<ThumbsUp key="g" className="size-3.5" aria-hidden="true" />, t("help.agree")],
    ];
    return (
        <Popover>
            <PopoverTrigger openOnHover delay={120} render={<Button variant="ghost" size="icon-xs" className="size-6 cursor-help text-muted-foreground hover:text-foreground" aria-label={t("help.label")} />}>
                <CircleHelp className="size-3.5" aria-hidden="true" />
            </PopoverTrigger>
            <PopoverPopup className="w-[min(320px,calc(100vw-2rem))]">
                <ul className="m-0 flex flex-col gap-1.5 p-0 text-[12.5px] text-foreground leading-normal">
                    {items.map(([icon, words]) => (
                        <li key={words} className="flex items-start gap-2">
                            <span className="mt-0.5 text-muted-foreground">{icon}</span>
                            <span>{words}</span>
                        </li>
                    ))}
                </ul>
            </PopoverPopup>
        </Popover>
    );
}

/** A block's actions: the main decisions with their words, a rule, then the secondary icons. */
export function ActionRow({ main, secondary, className }: { main?: React.ReactNode; secondary?: React.ReactNode; className?: string }): React.ReactElement | null {
    const hasMain = Boolean(main);
    if (!hasMain && !secondary) return null;
    return (
        <span className={cn("inline-flex flex-wrap items-center gap-0.5 align-middle", className)}>
            {main}
            {hasMain && secondary ? <Separator orientation="vertical" className="mx-1 h-4" /> : null}
            {secondary}
        </span>
    );
}

/** Shown on the hover of the cell (and keyboard focus; always on a touch screen). */
export const ON_HOVER = "opacity-0 pointer-coarse:opacity-100 focus-within:opacity-100 group-hover/cell:opacity-100";

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
    const locale = useLocale();
    const a = edits?.approval;
    if (!a) return null;
    const when = f.date(a.at * 1000, { dateStyle: "medium" });
    const author = a.variant === "proposal" ? (edits?.proposals.find((p) => p.id === a.proposalId)?.author.name ?? null) : null;
    const what =
        a.variant === "game"
            ? t("approved.headGame")
            : a.variant === "ai"
              ? t("approved.headAi")
              : a.variant === "proposal"
                ? author
                    ? t("approved.headVariant", { name: author })
                    : t("approved.headProposal")
                : t("approved.headVariant", { name: translatorName(a.variant as (typeof TRANSLATOR_SOURCES)[number], locale) });
    // A proposal written (or rewritten) after the approval.
    const newer = proposalsAt(edits, place).some((p) => p.id !== a.proposalId && (p.editedAt ?? p.createdAt) > a.at);
    return (
        <Toned
            tone={TONE.approved}
            label={
                <>
                    <span className="font-medium text-success-foreground">{what}</span>
                    <span>· {t("approved.byDate", { name: a.by.name, date: when })}</span>
                    {marks}
                    {a.releasedIn ? (
                        <Badge variant="success" size="sm">
                            {t("release.in", { version: a.releasedIn })}
                        </Badge>
                    ) : null}
                    {newer ? (
                        <Badge variant="info" size="sm">
                            {t("approved.newer")}
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
    const locale = useLocale();
    const remove = useEditMutation((id: number) => deleteProposalFn({ data: { id } }), t("edit.failedDelete"));
    const list = proposalsAt(edits, place);
    if (list.length === 0) return null;
    return (
        <>
            {list.map((p) => {
                const mine = p.author.id === me.userId;
                // One step back: its previous version, else the game's Russian.
                const before = compare ? (p.previousText ?? ruGame) : null;
                const approvedHere = p.context ? p.approvedAt !== null : edits?.approval?.proposalId === p.id;
                return (
                    <div key={p.id} className="flex flex-col gap-1">
                        <Toned
                            tone={TONE.proposal}
                            label={
                                <>
                                    <span>{mine ? t("proposal.mine") : t("proposal.by", { name: p.author.name })}</span>
                                    <span>· {f.date((p.editedAt ?? p.createdAt) * 1000, { dateStyle: "medium" })}</span>
                                    {p.editedAt !== null ? <span>· {t("proposal.edited")}</span> : null}
                                    {p.source !== "own" ? <span>· {fromWords(p.source, t, locale)}</span> : null}
                                    {p.context && !p.here ? (
                                        <Badge variant="info" size="sm">
                                            {t("proposal.contextBadge")}
                                        </Badge>
                                    ) : null}
                                    {p.here ? (
                                        // Outside the story the game cannot tell places apart: a request to the pack's maintainer.
                                        <>
                                            <Badge variant="warning" size="sm">
                                                {t("proposal.onlyHere")}
                                            </Badge>
                                            <GamePlace section={p.here.section} objectId={p.here.objectId} objectEn={p.here.objectEn ?? null} field={p.here.field} idx={p.here.idx} t={t} />
                                        </>
                                    ) : null}
                                    {p.context && p.approvedAt !== null ? (
                                        <Badge variant="success" size="sm">
                                            {p.releasedIn ? t("release.in", { version: p.releasedIn }) : t("approved.here")}
                                        </Badge>
                                    ) : null}
                                    {marks?.(p)}
                                    {before ? <span>· {p.previousText ? t("diff.vsPrevious") : t("diff.vsGame")}</span> : null}
                                    <ActionRow
                                        className={approvedHere || p.agreements.length > 0 ? undefined : ON_HOVER}
                                        main={
                                            isLead(me) ? (
                                                approvedHere ? (
                                                    <ApproveButton place={place} variant="proposal" proposalId={p.id} me={me} approved t={t} />
                                                ) : (
                                                    <>
                                                        <ApproveButton place={place} variant="proposal" proposalId={p.id} me={me} approved={false} t={t} />
                                                        <RejectButton proposalId={p.id} me={me} t={t} />
                                                    </>
                                                )
                                            ) : undefined
                                        }
                                        secondary={
                                            <>
                                                <AgreeButton place={place} variant="proposal" proposalId={p.id} people={p.agreements} me={me} own={mine} inBar t={t} />
                                                {mine ? (
                                                    <>
                                                        <IconAction icon={<PenLine aria-hidden="true" />} label={t("proposal.edit")} onClick={() => onEdit(p)} />
                                                        <IconAction icon={<Trash2 aria-hidden="true" />} label={t("proposal.delete")} onClick={() => remove.mutate(p.id)} />
                                                    </>
                                                ) : null}
                                            </>
                                        }
                                    />
                                </>
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
export type EditorState = { kind: "proposal"; text: string; source: ProposalSource; context: boolean; here?: GameHere | null } | { kind: "remark" | "note"; quote: string | null };

/** The text selected inside `within`, if any: a comment quotes it. */
function selectedIn(within: Element | null): string | null {
    const sel = typeof window === "undefined" ? null : window.getSelection();
    if (!sel || sel.isCollapsed || !within || !sel.anchorNode || !within.contains(sel.anchorNode)) return null;
    const text = sel.toString().trim();
    return text ? text : null;
}

/**
 * The game's text's actions, at the top right of it on hover or keyboard
 * focus (always on a touch screen): a translator's "Approve" (keep as in the
 * game), then, as gray icons after a rule, propose, remark, note, history and
 * agree, and "?" with what each icon means. The other variants carry their
 * own buttons, so the panel never repeats them. A remark or note quotes the
 * text selected in the row (read before the click can clear it). History
 * opens the history of the line's translation (LineHistory.tsx).
 */
export function CellActions({ line, onOpen, onHistory, main, agree, t }: { line: TranslatorLine; onOpen: (state: EditorState) => void; onHistory: () => void; main?: React.ReactNode; agree?: React.ReactNode; t: T }): React.ReactElement {
    const comment = (kind: "remark" | "note", from: HTMLElement) => onOpen({ kind, quote: selectedIn(from.closest("tr")) });
    const keys = (kind: "remark" | "note") => (e: React.KeyboardEvent<HTMLButtonElement>) => {
        if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            comment(kind, e.currentTarget);
        }
    };
    const down = (kind: "remark" | "note") => (e: React.PointerEvent<HTMLButtonElement>) => {
        e.preventDefault();
        comment(kind, e.currentTarget);
    };
    return (
        <div className={cn("absolute end-0 top-0 z-10 flex items-center rounded-lg border bg-popover p-0.5 shadow-xs transition-opacity", ON_HOVER)}>
            <ActionRow
                main={main}
                secondary={
                    <>
                        {line.editable ? <IconAction icon={<PenLine aria-hidden="true" />} label={t("action.propose")} onClick={() => onOpen({ kind: "proposal", text: line.ruGame ?? "", source: "own", context: false })} /> : null}
                        <IconAction icon={<MessageSquare aria-hidden="true" />} label={t("action.remark")} onPointerDown={down("remark")} onKeyDown={keys("remark")} />
                        <IconAction icon={<NotebookPen aria-hidden="true" />} label={t("action.note")} onPointerDown={down("note")} onKeyDown={keys("note")} />
                        <IconAction icon={<History aria-hidden="true" />} label={t("action.history")} onClick={onHistory} />
                        {agree}
                        <ActionsHelp t={t} />
                    </>
                }
            />
        </div>
    );
}

/** Every place of the entry as links, with the lines before and after (`compact`: the places only). */
export function PlacesList({ place, nickname, compact = false, t }: { place: EditPlace; nickname: string; compact?: boolean; t: T }): React.ReactElement {
    const query = useQuery(placesQueryOptions(place));
    const nameOf = useStoryNames();
    if (query.isError) return <LoadFailed layout="inline" message={t("places.load")} onRetry={() => query.refetch()} />;
    if (!query.data) return <Skeleton className="h-16 w-full" />;
    const { places, total } = query.data;
    return (
        <div className="flex flex-col gap-1">
            <ul className={cn("flex max-h-72 flex-col overflow-y-auto text-xs", compact ? "gap-1" : "gap-2")}>
                {places.map((p) => {
                    // A field of the game data: where it stands, in words.
                    if (p.section !== "story")
                        return (
                            <li key={`${p.section}-${p.objectId}-${p.field}-${p.idx}`} className="flex flex-col gap-0.5 border-s-2 ps-2">
                                <span className={compact ? undefined : "font-medium"}>
                                    <GamePlace section={p.section} objectId={p.objectId} objectEn={p.objectEn} field={p.field} idx={p.idx} t={t} />
                                </span>
                            </li>
                        );
                    const n = nameOf(p.storyId);
                    const here = samePlace(p, place);
                    return (
                        <li key={`${p.storyId}-${p.line}-${p.part}`} className="flex flex-col gap-0.5 border-s-2 ps-2">
                            <span className={compact ? undefined : "font-medium"}>
                                {n ? (
                                    <Link to="/translators" search={{ group: n.groupId, story: p.storyId }} hash={`n-${p.ord + 1}`} className="underline-offset-4 hover:underline">
                                        {t("places.item", { group: n.group, story: n.story, n: p.ord + 1 })}
                                    </Link>
                                ) : (
                                    t("places.itemBare", { story: p.storyId, n: p.ord + 1 })
                                )}
                                {here ? <span className="font-normal text-subtle-foreground"> · {t("places.here")}</span> : null}
                            </span>
                            {!compact && p.before ? (
                                <span className="text-subtle-foreground">
                                    {t("places.before")} <StoryText text={p.before} nickname={nickname} />
                                </span>
                            ) : null}
                            {!compact && p.after ? (
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

/**
 * Every place of the entry as a table: № | where | was (the game's Russian
 * there) | now (`after`: the proposal) | English | Chinese (when any place
 * has it). Story places link to their line; game data places to their card.
 */
export function PlacesPreview({ place, after, nickname, t }: { place: EditPlace; after: string | null; nickname: string; t: T }): React.ReactElement {
    const query = useQuery(placesQueryOptions(place));
    const nameOf = useStoryNames();
    if (query.isError) return <LoadFailed layout="inline" message={t("places.load")} onRetry={() => query.refetch()} />;
    if (!query.data) return <Skeleton className="h-24 w-full" />;
    const { places, total } = query.data;
    const cn = places.some((p) => p.cn);
    const prose = "whitespace-normal py-1.5 align-top leading-snug";
    return (
        <div className="flex flex-col gap-1">
            <div className="max-h-[60vh] overflow-y-auto">
                <Table aria-label={t("places.table")} className="wrap-anywhere text-xs">
                    <TableHeader>
                        <TableRow>
                            <TableHead className="w-10">{t("table.line")}</TableHead>
                            <TableHead className="w-48">{t("edits.col.where")}</TableHead>
                            <TableHead>{t("edits.col.was")}</TableHead>
                            {after !== null ? <TableHead>{t("edits.col.now")}</TableHead> : null}
                            <TableHead>{t("table.en")}</TableHead>
                            {cn ? <TableHead>{t("table.cn")}</TableHead> : null}
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        {places.map((p, i) => {
                            const story = p.section === "story";
                            const n = story ? nameOf(p.storyId) : null;
                            const here = story && samePlace(p, place);
                            return (
                                <TableRow key={story ? `${p.storyId}-${p.line}-${p.part}` : `${p.section}-${p.objectId}-${p.field}-${p.idx}`}>
                                    <TableCell className="py-1.5 align-top text-subtle-foreground tabular-nums">{i + 1}</TableCell>
                                    <TableCell className={prose}>
                                        {!story ? (
                                            <GamePlace section={p.section} objectId={p.objectId} objectEn={p.objectEn} field={p.field} idx={p.idx} t={t} />
                                        ) : n ? (
                                            <Link to="/translators" search={{ group: n.groupId, story: p.storyId }} hash={`n-${p.ord + 1}`} className="underline-offset-4 hover:underline">
                                                {t("places.item", { group: n.group, story: n.story, n: p.ord + 1 })}
                                            </Link>
                                        ) : (
                                            t("places.itemBare", { story: p.storyId, n: p.ord + 1 })
                                        )}
                                        {here ? <span className="text-subtle-foreground"> · {t("places.here")}</span> : null}
                                    </TableCell>
                                    <TableCell className={prose}>{p.ruGame ? <StoryText text={p.ruGame} nickname={nickname} /> : <span className="text-subtle-foreground">{t("ru.missing")}</span>}</TableCell>
                                    {after !== null ? <TableCell className={prose}>{p.ruGame ? <TextDiff before={p.ruGame} after={after} labels={diffLabels(t)} /> : <StoryText text={after} nickname={nickname} />}</TableCell> : null}
                                    <TableCell className={prose}>{p.en ? <StoryText text={p.en} nickname={nickname} /> : null}</TableCell>
                                    {cn ? <TableCell className={`${prose} whitespace-pre-line`}>{p.cn ?? ""}</TableCell> : null}
                                </TableRow>
                            );
                        })}
                    </TableBody>
                </Table>
            </div>
            {total > places.length ? <span className="text-subtle-foreground text-xs">{t("places.more", { count: total - places.length })}</span> : null}
        </div>
    );
}

/** "Places: N" in the Edits list: opens every place of the entry with was / now. */
export function PlacesDialog({ place, count, after, nickname, t }: { place: EditPlace; count: number; after: string; nickname: string; t: T }): React.ReactElement {
    const [open, setOpen] = useState(false);
    return (
        <>
            <Button variant="link" size="xs" className="h-auto justify-start px-0 text-subtle-foreground text-xs" onClick={() => setOpen(true)}>
                {t("where.places", { count })}
            </Button>
            <Dialog open={open} onOpenChange={setOpen}>
                <DialogPopup className="max-w-6xl">
                    <DialogHeader>
                        <DialogTitle>{t("save.title", { count })}</DialogTitle>
                    </DialogHeader>
                    <DialogPanel>{open ? <PlacesPreview place={place} after={after} nickname={nickname} t={t} /> : null}</DialogPanel>
                </DialogPopup>
            </Dialog>
        </>
    );
}

interface ISaveDialogProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    place: EditPlace;
    places: number;
    /** Offer "only this place" when the line allows it (`contextAllowed`: every story line); a rewrite keeps its scope. Else the "changes everywhere" note. */
    allowContext: boolean;
    /** Outside the story, at a place of the game data: offer "a separate translation is needed here" (a request to the pack's maintainer). */
    here?: GameHere | null;
    pending: boolean;
    onSave: (context: boolean) => void;
    nickname: string;
    /** The proposal being saved: the preview's "now". */
    text: string;
    t: T;
}

/** Saving a proposal for an entry in several places: for all of them (with the list and a confirmation) or for this place only. */
export function SaveDialog({ open, onOpenChange, place, places, allowContext, here = null, pending, onSave, nickname, text, t }: ISaveDialogProps): React.ReactElement {
    const scopeId = useId();
    const [context, setContext] = useState(false);
    const [understood, setUnderstood] = useState(false);
    const forAll = !context;
    return (
        <AlertDialog open={open} onOpenChange={onOpenChange}>
            <AlertDialogPopup className="max-w-6xl">
                <AlertDialogHeader>
                    <AlertDialogTitle>{t("save.title", { count: places })}</AlertDialogTitle>
                    <AlertDialogDescription>{t("save.proposalOnly")}</AlertDialogDescription>
                    {allowContext || here ? (
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
                                    <span className="flex items-center gap-1 font-medium">
                                        {here ? t("save.onlyHere") : t("save.here")}
                                        {here ? <InfoHint label={t("save.onlyHereHint")}>{t("save.onlyHereHint")}</InfoHint> : null}
                                    </span>
                                    {here ? null : <span className="text-muted-foreground text-xs">{t("save.hereHint")}</span>}
                                    {here ? (
                                        <span className="text-muted-foreground text-xs">
                                            <GamePlace section={here.section} objectId={here.objectId} objectEn={here.objectEn ?? null} field={here.field} idx={here.idx} t={t} />
                                        </span>
                                    ) : null}
                                </span>
                            </label>
                        </RadioGroup>
                    ) : (
                        <p className="pt-2 font-medium text-sm">{t("save.hereShort", { count: places })}</p>
                    )}
                    {forAll ? (
                        <div className="flex flex-col gap-2 pt-2">
                            <PlacesPreview place={place} after={cleanText(text)} nickname={nickname} t={t} />
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

/** The token editor's words. */
export function tokenLabels(t: T): ITokenLabels {
    return {
        insertNickname: t("editor.insertNickname"),
        insertToken: (token) => t("editor.insertToken", { token }),
        remove: t("editor.removeToken"),
        wrap: (tag) => t("editor.wrap", { tag }),
        unwrap: (tag) => t("editor.unwrap", { tag }),
        wrapHint: t("editor.wrapHint"),
    };
}

/** What the editor of a line offers and knows: the substitutions and the tags of the line's English, each tag in words and in its look. */
export function useEditorTools(en: string, t: T): { palette: string[]; tags: ITagButton[]; lookup: TagLookup; labels: ITokenLabels; words: (name: string) => string } {
    const game = useGameTags();
    const locale = useLocale();
    const lookup: TagLookup = (open) => {
        const tag = tagOf(open);
        return { look: spanLook(tag), label: tagWords(tag, game, t, locale) };
    };
    const pairs = tagPairs(en);
    return {
        palette: tokensIn(en),
        tags: pairs.map((pair) => ({ open: pair.open, close: pair.close, ...lookup(pair.open) })),
        lookup,
        labels: tokenLabels(t),
        // A tag of a diff (`@ba.vup`, `i`, `color`) in words.
        words: (name) => {
            const pair = pairs.find((x) => x.name === name);
            return pair ? lookup(pair.open).label : name.startsWith("@") || name.startsWith("$") ? lookup(`<${name}>`).label : name === "color" || name === "size" ? lookup(`<${name}=>`).label : lookup(`<${name}>`).label;
        },
    };
}

/** "The tags are not the original's", under an editor: what is missing and what is extra, in words. Nothing when they agree. */
export function TagsWarning({ en, text, words, t }: { en: string; text: string; words: (name: string) => string; t: T }): React.ReactElement | null {
    const diff = tagDiff(en, text);
    if (diff.missing.length + diff.extra.length === 0) return null;
    const list = (names: readonly string[]) => [...new Set(names)].map(words).join(", ");
    return (
        <span className="text-warning-foreground text-xs">
            {t("editor.tagsDiffer")}
            {diff.missing.length > 0 ? ` ${t("editor.tagsMissing", { tags: list(diff.missing) })}` : ""}
            {diff.extra.length > 0 ? ` ${t("editor.tagsExtra", { tags: list(diff.extra) })}` : ""}
        </span>
    );
}

interface IEditorProps {
    line: TranslatorLine;
    place: EditPlace;
    initial: string;
    source: ProposalSource;
    /** Editing a one-place proposal: it stays one. */
    context: boolean;
    /** The place of the game data: of the one-place proposal being edited, else of the row (a new one may ask a separate translation there). */
    here?: GameHere | null;
    nickname: string;
    onClose: () => void;
    t: T;
}

/** The in-row editor of my proposal: Ctrl+Enter saves, Esc cancels; a substitution is one chip, a tag a span as in the game (TokenEditor.tsx). */
export function ProposalEditor({ line, place, initial, source, context, here = null, nickname, onClose, t }: IEditorProps): React.ReactElement {
    const id = useId();
    const hintId = useId();
    const locale = useLocale();
    const [text, setText] = useState(initial);
    const [asking, setAsking] = useState(false);
    const tools = useEditorTools(line.en, t);
    const save = useEditMutation((c: { context: boolean; confirmMany: boolean }) => createProposalFn({ data: { place, text, source, context: c.context, here, confirmMany: c.confirmMany } }), t("edit.failedSave"));
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
                {editorLabel(source, t, locale)}
            </label>
            <TokenEditor id={id} initial={initial} nickname={nickname} palette={tools.palette} tags={tools.tags} lookup={tools.lookup} onChange={setText} onSubmit={submit} onCancel={onClose} labels={tools.labels} describedBy={hintId} />
            <TagsWarning en={line.en} text={text} words={tools.words} t={t} />
            <span id={hintId} className="text-subtle-foreground text-xs">
                {t("save.proposalOnly")}
            </span>
            {line.ruGameMatch === "lines" || line.en.includes("\n") ? <span className="text-subtle-foreground text-xs">{t("editor.wholeEntry")}</span> : null}
            {sameAsGame ? <span className="text-subtle-foreground text-xs">{t("editor.sameAsGame")}</span> : null}
            {context ? <span className="text-subtle-foreground text-xs">{here ? t("editor.onlyHere") : t("editor.contextOnly")}</span> : <PlacesNote line={line} place={place} nickname={nickname} t={t} />}
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
            {asking ? <SaveDialog open={asking} onOpenChange={setAsking} place={place} places={line.places} allowContext={line.contextAllowed} here={here} pending={save.isPending} onSave={(c) => save.mutate({ context: c, confirmMany: !c }, done)} nickname={nickname} text={text} t={t} /> : null}
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
    const locale = useLocale();
    const [text, setText] = useState("");
    const [quote, setQuote] = useState(initialQuote);
    const [variant, setVariant] = useState<string>(LINE);
    const options: { value: string; label: string }[] = [{ value: LINE, label: t("note.aboutLine") }];
    if (line.ruGame) options.push({ value: "game", label: t("note.aboutGame") });
    for (const p of proposalsAt(edits, place)) options.push({ value: `proposal:${p.id}`, label: t("note.aboutProposal", { name: p.author.name }) });
    for (const source of TRANSLATOR_SOURCES) if (line[source]) options.push({ value: source, label: t("note.aboutTranslator", { name: translatorName(source, locale) }) });
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
