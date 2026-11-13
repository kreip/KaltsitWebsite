/**
 * One story's lines as a dense table: # | speaker | Russian as in the game |
 * English | Chinese (optional). Under the Russian: the glossary terms of the
 * line's English with whether the Russian holds their form, the proposals,
 * the translators' variants (each under its author's name) and the
 * machine's (each with "Propose this"), the notes and
 * remarks. Hovering the Russian shows its actions (LineEdits.tsx), over the
 * game's text and under the approved variant. With Service on, the stage
 * commands sit between the lines as gray rows, in words.
 */
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Check, PenLine, X } from "lucide-react";
import type React from "react";
import { Fragment, useContext, useId, useMemo, useState } from "react";
import { Badge } from "#/components/ui/badge";
import { InfoHint } from "#/components/ui/info-hint";
import { Label } from "#/components/ui/label";
import { Select, SelectContent, SelectItem, SelectSeparator, SelectTrigger, SelectValue } from "#/components/ui/select";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "#/components/ui/table";
import { toastManager } from "#/components/ui/toast";
import { useFormatters, useLocale, useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { setRecommendRuleFn, translatorsMeQueryOptions } from "#/lib/translators/api";
import { cn } from "#/lib/utils";
import type { EditPlace } from "#/types/generated/EditPlace";
import type { GameHere } from "#/types/generated/GameHere";
import type { KeyEdits } from "#/types/generated/KeyEdits";
import type { LineTerm } from "#/types/generated/LineTerm";
import type { OutdatedEdit } from "#/types/generated/OutdatedEdit";
import type { StoryCommand } from "#/types/generated/StoryCommand";
import type { TranslatorLine } from "#/types/generated/TranslatorLine";
import type { TranslatorsMe } from "#/types/generated/TranslatorsMe";
import { BulkApprove, type IBulkLine } from "./BulkApprove";
import { type Address, checkVariant, type ICandidate, type IVariantCheck, lineProblems, neighbourAddress, PROBLEM_KINDS, type ProblemKind, type RecommendRule, recommend } from "./checks";
import {
    ActionRow,
    AgreeButton,
    ApproveButton,
    ApprovedVariant,
    approvedVariant,
    CellActions,
    CommentEditor,
    diffLabels,
    type EditorState,
    IconAction,
    isLead,
    LineComments,
    ON_HOVER,
    OutdatedHere,
    ProposalEditor,
    Proposals,
    placeOf,
    placeVotes,
    proposalsAt,
    quotesAt,
    StatusDot,
    TONE,
    Toned,
    VariantNotes,
} from "./LineEdits";
import { LineHistoryDialog } from "./LineHistory";
import { Marks } from "./Marks";
import { type IFind, StoryText } from "./StoryText";
import { DISPLAYED_KINDS, describeCommands } from "./service";
import { TextDiff } from "./TextDiff";
import type { messages } from "./translators.messages";
import { TRANSLATOR_SOURCES, translatorName } from "./translatorVariants";
import { GameOnly } from "./ViewMenu";

type T = TypedT<typeof messages>;

/** A prose cell wraps, reads from the top and stays dense (the kit cell is one nowrap line). */
export const PROSE = "whitespace-normal py-1.5 align-top leading-snug";

export function Speaker({ line, t }: { line: TranslatorLine; t: T }): React.ReactElement | null {
    if (line.kind === "option")
        return (
            <Badge variant="tint" size="sm">
                {t("line.option", { part: line.part })}
            </Badge>
        );
    if (line.kind === "subtitle") return <span className="text-muted-foreground">{t("line.subtitle")}</span>;
    if (line.kind === "sticker") return <span className="text-muted-foreground">{t("line.sticker")}</span>;
    if (!line.speakerEn) return null;
    const ru = line.speakerRu && line.speakerRu !== line.speakerEn ? line.speakerRu : null;
    return (
        <span className="flex flex-col">
            <span className="font-medium">{ru ?? line.speakerEn}</span>
            {ru ? <span className="text-muted-foreground text-xs">{line.speakerEn}</span> : null}
        </span>
    );
}

interface ITermGroup {
    matched: string;
    ru: string[];
    /** true / false when checked; null for Sova's sheet, a line without Russian, or a line approved since the term last changed. */
    found: boolean | null;
    fromSova: boolean;
    /** Approved after the glossary last changed the term: checked by the team, no mark. */
    verified: boolean;
    /** Approved, but the glossary changed the term since: marked again, with a note. */
    changed: boolean;
    title: string;
}

/** One entry per English text found: a key with several rows (`Amiya (Guard)`, `Amiya (Medic)`) shows all their Russian. */
function groupTerms(terms: readonly LineTerm[], sovaSheet: string): ITermGroup[] {
    const out = new Map<string, ITermGroup>();
    for (const term of terms) {
        let g = out.get(term.matched);
        if (!g) {
            g = { matched: term.matched, ru: [], found: null, fromSova: true, verified: false, changed: false, title: "" };
            out.set(term.matched, g);
        }
        if (!g.ru.includes(term.ru)) g.ru.push(term.ru);
        if (!term.fromSova) g.fromSova = false;
        if (term.verified) g.verified = true;
        if (term.changedAfterApproval) g.changed = true;
        if (term.found === true) g.found = true;
        else if (term.found === false && g.found === null) g.found = false;
        const about = [term.fromSova ? sovaSheet : term.sheet, term.ruPlural, term.note].filter(Boolean).join(" · ");
        g.title = g.title ? `${g.title}\n${about}` : about;
    }
    return [...out.values()];
}

function Terms({ terms, t }: { terms: readonly LineTerm[]; t: T }): React.ReactElement | null {
    if (terms.length === 0) return null;
    return (
        <ul className="flex flex-wrap gap-x-3 gap-y-0.5 text-xs" aria-label={t("terms.label")}>
            {groupTerms(terms, t("glossary.translatorSheet")).map((g) => (
                <li key={g.matched} className="inline-flex items-center gap-1" title={g.title}>
                    <span className="text-muted-foreground">{g.matched}</span>
                    <span className="text-subtle-foreground" aria-hidden="true">
                        →
                    </span>
                    <span>{g.ru.join(" / ")}</span>
                    {g.found === true ? (
                        <span className="inline-flex text-success-foreground">
                            <Check className="size-3.5" aria-hidden="true" />
                            <span className="sr-only">{t("terms.found")}</span>
                        </span>
                    ) : g.found === false ? (
                        <span className="inline-flex items-center gap-0.5 text-warning-foreground">
                            <X className="size-3.5" aria-hidden="true" />
                            {g.changed ? t("terms.changedAfterApproval") : t("terms.notFound")}
                        </span>
                    ) : g.verified ? (
                        <span className="inline-flex text-subtle-foreground" title={t("terms.verifiedHint")}>
                            <Check className="size-3.5" aria-hidden="true" />
                            <span className="sr-only">{t("terms.verified")}</span>
                        </span>
                    ) : g.fromSova ? (
                        <span className="text-subtle-foreground">({t("terms.variant")})</span>
                    ) : null}
                </li>
            ))}
        </ul>
    );
}

/** A translator's or the machine's variant on one line: label, text, its marks, then its own actions (Approve; propose from it, agree). */
function InlineVariant({ tone, label, text, nickname, marks, actions }: { tone: string; label: string; text: string; nickname: string; marks?: React.ReactNode; actions?: React.ReactNode }): React.ReactElement {
    return (
        <div className="border-(--tone) border-s-2 ps-2" style={{ "--tone": tone } as React.CSSProperties}>
            <span className="me-1.5 text-subtle-foreground text-xs">{label}</span>
            <StoryText text={text} nickname={nickname} />
            {marks}
            {actions ? <span className="ms-1.5">{actions}</span> : null}
        </div>
    );
}

/** The same text, spaces aside: a translator's variant equal to the game's text is not shown. */
function sameText(a: string | null | undefined, b: string | null | undefined): boolean {
    const squash = (x: string) => x.replaceAll("\\n", "\n").replace(/\s+/g, " ").trim();
    return Boolean(a && b) && squash(a as string) === squash(b as string);
}

/** A translator's variant at a line, unless it is the game's text again. */
export function translatorText(line: TranslatorLine, source: (typeof TRANSLATOR_SOURCES)[number]): string | null {
    const text = line[source];
    return text && !sameText(text, line.ruGame) ? text : null;
}

/** A line's checks per variant and its recommended variant (checks.ts holds the rule). */
export interface IAnalysis {
    checks: Map<string, IVariantCheck>;
    recommended: string | null;
    /** What is wrong with the line as the game shows it (none once approved): its row is yellow. */
    problems: Set<ProblemKind>;
}

/** A line with a problem stands out in yellow. */
export const PROBLEM_ROW = "bg-warning/8";

/** "Errors: all (12)", then each problem with its count: the status filter's problem options. */
export function problemOptions(analyses: readonly IAnalysis[]): { value: "problems" | ProblemKind; count: number }[] {
    const count = (k: ProblemKind) => analyses.filter((a) => a.problems.has(k)).length;
    return [{ value: "problems" as const, count: analyses.filter((a) => a.problems.size > 0).length }, ...PROBLEM_KINDS.map((k) => ({ value: k, count: count(k) }))];
}

export function analyse(line: TranslatorLine, edits: KeyEdits | undefined, place: EditPlace, neighbours: Address | null, rule: RecommendRule): IAnalysis {
    const checks = new Map<string, IVariantCheck>();
    const candidates: ICandidate[] = [];
    const add = (variant: string, text: string | null | undefined, agreements = 0) => {
        if (!text) return;
        const check = checkVariant(text, variant, line, neighbours);
        checks.set(variant, check);
        candidates.push({ variant, agreements, check });
    };
    for (const p of proposalsAt(edits, place)) add(`proposal:${p.id}`, p.text, p.agreements.length);
    for (const source of TRANSLATOR_SOURCES) add(source, translatorText(line, source));
    add("ai", line.ai);
    add("game", line.ruGame);
    if (edits?.approval) checks.set("approved", checkVariant(edits.approval.text, "approved", line, neighbours));
    const approvedHere = Boolean(edits?.approval) || proposalsAt(edits, place).some((p) => p.context && p.approvedAt !== null);
    return { checks, recommended: line.editable ? recommend(candidates, approvedHere, rule) : null, problems: lineProblems(line.status === "approved", checks) };
}

export interface IRuCellProps {
    line: TranslatorLine;
    analysis: IAnalysis;
    /** Where the line stands: a story line, or a text outside the story (the search). */
    place: EditPlace;
    nickname: string;
    showVariants: boolean;
    /** Compare: the proposals and the approved text as changes against the text one step back. */
    showDiff: boolean;
    edits: KeyEdits | undefined;
    outdated: readonly OutdatedEdit[];
    me: TranslatorsMe;
    editor: EditorState | null;
    onEditor: (state: EditorState | null) => void;
    /** The search's text, highlighted in the game's Russian. */
    find?: IFind | null;
    /** Outside the story: the place of the game data the row stands at (a proposal may ask a separate translation there). */
    here?: GameHere | null;
    t: T;
}

export function RuCell(props: IRuCellProps): React.ReactElement {
    // Only as in the game: the game's text, nothing else.
    if (useContext(GameOnly)) {
        const { line, nickname, find = null, t } = props;
        return <div data-sel="ru">{line.ruGame ? <StoryText text={line.ruGame} nickname={nickname} find={find} /> : <span className="text-subtle-foreground">{t("ru.missing")}</span>}</div>;
    }
    return <FullRuCell {...props} />;
}

function FullRuCell({ line, analysis, place, nickname, showVariants, showDiff, edits, outdated, me, editor, onEditor, find = null, here = null, t }: IRuCellProps): React.ReactElement {
    const f = useFormatters();
    const locale = useLocale();
    const [history, setHistory] = useState(false);
    const quotes = quotesAt(edits, place);
    const notes = { edits, place, me, t };
    const { checks, recommended } = analysis;
    const approved = approvedVariant(edits);
    const marks = (variant: string, skipGlossary = false) => {
        const check = checks.get(variant);
        return check ? <Marks check={check} recommended={recommended === variant} skipGlossary={skipGlossary} t={t} /> : null;
    };
    // A translator's or the machine's variant: Approve (at once, no "propose" step) | propose from it, agree.
    const variantActions = (variant: "sova" | "ai", text: string) => {
        if (!line.editable) return undefined;
        const votes = placeVotes(edits, place, variant);
        return (
            <ActionRow
                className={votes.length > 0 ? undefined : ON_HOVER}
                main={isLead(me) && approved !== variant ? <ApproveButton place={place} variant={variant} me={me} approved={false} t={t} /> : undefined}
                secondary={
                    <>
                        {!editor ? <IconAction icon={<PenLine aria-hidden="true" />} label={t("action.proposeThis")} onClick={() => onEditor({ kind: "proposal", text, source: variant, context: false })} /> : null}
                        <AgreeButton place={place} variant={variant} people={votes} me={me} inBar t={t} />
                    </>
                }
            />
        );
    };
    const proposalCount = proposalsAt(edits, place).length;
    return (
        <div className="group/cell flex flex-col gap-1" data-sel="ru">
            {history ? <LineHistoryDialog place={place} open={history} onOpenChange={setHistory} nickname={nickname} t={t} /> : null}
            <ApprovedVariant edits={edits} place={place} me={me} nickname={nickname} marks={marks("approved")} compareWith={showDiff ? line.ruGame : null} t={t} />
            {/* The actions hang over the game's text, under the approved variant: its line (who approved, Withdraw) stays in sight. */}
            <div className="relative">
                {editor === null ? (
                    <CellActions
                        line={line}
                        onOpen={onEditor}
                        onHistory={() => setHistory(true)}
                        main={line.ruGame && line.editable && approved !== "game" && isLead(me) ? <ApproveButton place={place} variant="game" me={me} approved={false} confirmProposals={proposalCount} t={t} /> : undefined}
                        agree={line.ruGame && line.editable && (edits?.gameAgreements.length ?? 0) === 0 ? <AgreeButton place={place} variant="game" people={edits?.gameAgreements ?? []} me={me} inBar t={t} /> : undefined}
                        t={t}
                    />
                ) : null}
                {line.ruGame ? <StoryText text={line.ruGame} nickname={nickname} quotes={quotes} find={find} /> : <span className="text-subtle-foreground">{t("ru.missing")}</span>}
                {marks("game", true)}
                {line.ruGame && line.editable && (edits?.gameAgreements.length ?? 0) > 0 ? <AgreeButton place={place} variant="game" people={edits?.gameAgreements ?? []} me={me} t={t} /> : null}
            </div>
            {showDiff && line.ruGame && line.ruGameBefore && line.ruGameChangedAt !== null ? (
                <Toned tone={TONE.remark} label={t("diff.gameChanged", { date: f.date(line.ruGameChangedAt * 1000, { dateStyle: "medium" }) })}>
                    <TextDiff before={line.ruGameBefore} after={line.ruGame} labels={diffLabels(t)} />
                </Toned>
            ) : null}
            <VariantNotes {...notes} variant="game" />
            {line.pluginRole === "story" ? (
                <span className="flex items-center gap-1">
                    <Badge variant="warning" size="sm">
                        {t("plugin.badge")}
                    </Badge>
                    <InfoHint label={t("plugin.hint")}>{t("plugin.hint")}</InfoHint>
                </span>
            ) : null}
            {line.pluginRole === "ambiguous:story" ? (
                <span className="flex items-center gap-1">
                    <Badge variant="warning" size="sm">
                        {t("ambiguous.badge")}
                    </Badge>
                    <InfoHint label={t("ambiguous.hint")}>{t("ambiguous.hint")}</InfoHint>
                </span>
            ) : null}
            {line.ruGameMatch === "lines" ? <span className="text-subtle-foreground text-xs">{t("ru.lineByLine")}</span> : null}
            <Terms terms={line.terms} t={t} />
            <Proposals {...notes} nickname={nickname} compare={showDiff} ruGame={line.ruGame} marks={(p) => marks(`proposal:${p.id}`)} onEdit={(p) => onEditor({ kind: "proposal", text: p.text, source: p.source === "sova" || p.source === "ai" ? p.source : "own", context: p.context, here: p.here })} />
            {showVariants ? (
                <>
                    {/* Each translator's variant under its author's name. */}
                    {TRANSLATOR_SOURCES.map((source) => {
                        // The game's text again: nothing new to show.
                        const text = translatorText(line, source);
                        return text ? (
                            <Fragment key={source}>
                                <InlineVariant tone={TONE.translator} label={t("hints.translator", { name: translatorName(source, locale) })} text={text} nickname={nickname} marks={marks(source)} actions={variantActions(source, text)} />
                                <VariantNotes {...notes} variant={source} />
                            </Fragment>
                        ) : null;
                    })}
                    {line.ai ? (
                        <>
                            <InlineVariant tone={TONE.ai} label={t("hints.ai")} text={line.ai} nickname={nickname} marks={marks("ai")} actions={variantActions("ai", line.ai)} />
                            <VariantNotes {...notes} variant="ai" />
                        </>
                    ) : null}
                    {line.ruDict ? <InlineVariant tone={TONE.dict} label={t("hints.dict")} text={line.ruDict} nickname={nickname} /> : null}
                </>
            ) : null}
            <OutdatedHere items={outdated} nickname={nickname} t={t} />
            <LineComments {...notes} />
            {editor?.kind === "proposal" ? <ProposalEditor line={line} place={place} initial={editor.text} source={editor.source} context={editor.context} here={editor.context ? (editor.here ?? null) : here} nickname={nickname} onClose={() => onEditor(null)} t={t} /> : null}
            {editor && editor.kind !== "proposal" ? <CommentEditor line={line} place={place} edits={edits} kind={editor.kind} quote={editor.quote} onClose={() => onEditor(null)} t={t} /> : null}
        </div>
    );
}

/** The stage commands strictly between two script lines. */
function commandsBetween(commands: readonly StoryCommand[], after: number, before: number): StoryCommand[] {
    return commands.filter((c) => c.line > after && c.line < before && !DISPLAYED_KINDS.has(c.kind));
}

function ServiceRow({ commands, columns, nameOf, t }: { commands: readonly StoryCommand[]; columns: number; nameOf: (id: string) => string; t: T }): React.ReactElement | null {
    const words = describeCommands(commands, t, nameOf);
    if (!words) return null;
    return (
        <TableRow className="bg-muted/50 hover:bg-muted/50" aria-label={t("service.label")}>
            <TableCell colSpan={columns} className="whitespace-normal py-1 text-subtle-foreground text-xs leading-snug">
                {words}
            </TableCell>
        </TableRow>
    );
}

/** The lead's switch of the "recommended" rule, for the whole team (B unless switched). */
function RuleSelect({ rule, t }: { rule: RecommendRule; t: T }): React.ReactElement {
    const id = useId();
    const queryClient = useQueryClient();
    const set = useMutation({
        mutationFn: (next: RecommendRule) => setRecommendRuleFn({ data: { rule: next } }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: translatorsMeQueryOptions().queryKey }),
        onError: () => toastManager.add({ title: t("rule.failed"), type: "error" }),
    });
    return (
        <div className="flex w-full flex-col gap-1.5 sm:w-80">
            <span className="flex items-center gap-1">
                <Label htmlFor={id}>{t("rule.label")}</Label>
                <InfoHint label={t("rule.hint")}>{t("rule.hint")}</InfoHint>
            </span>
            <Select value={rule} onValueChange={(v: string | null) => (v === "A" || v === "B") && v !== rule && set.mutate(v)} disabled={set.isPending}>
                <SelectTrigger id={id} size="sm">
                    <SelectValue>{() => t(`rule.${rule}` as "rule.B")}</SelectValue>
                </SelectTrigger>
                <SelectContent>
                    <SelectItem value="B">{t("rule.B")}</SelectItem>
                    <SelectItem value="A">{t("rule.A")}</SelectItem>
                </SelectContent>
            </Select>
        </div>
    );
}

type LineFilter = "all" | "proposals" | "recommended" | "outdated" | "remarks" | "problems" | ProblemKind;
const FILTERS: readonly LineFilter[] = ["all", "proposals", "recommended", "outdated", "remarks"];

/** The status filter's word for an option: a problem with its count. */
export function filterWord(value: string, t: T, count?: number): string {
    if (value === "problems") return t("problem.all", { count: count ?? 0 });
    if ((PROBLEM_KINDS as readonly string[]).includes(value)) return t("problem.item", { name: t(`problem.${value}` as "problem.glossary"), count: count ?? 0 });
    return t(`filter.${value}` as "filter.all");
}

export interface ILinesTableProps {
    storyId: string;
    lines: readonly TranslatorLine[];
    /** What was proposed, approved and said, by entry. */
    edits: readonly KeyEdits[];
    /** Decisions left at this story's changed places. */
    outdated: readonly OutdatedEdit[];
    me: TranslatorsMe;
    label: string;
    showCn: boolean;
    showVariants: boolean;
    showDiff: boolean;
    /** The script's commands when Service is on, else null. */
    commands: readonly StoryCommand[] | null;
    /** A sprite's name for "On stage". */
    nameOf: (charId: string) => string;
    /** Shown for `{@nickname}`. */
    nickname: string;
}

export function LinesTable({ storyId, lines, edits, outdated, me, label, showCn, showVariants, showDiff, commands, nameOf, nickname }: ILinesTableProps): React.ReactElement {
    const t: T = useT("translators");
    const columns = showCn ? 5 : 4;
    const byKey = useMemo(() => new Map(edits.map((e) => [e.editKey, e])), [edits]);
    const staleAt = useMemo(() => {
        const out = new Map<string, OutdatedEdit[]>();
        for (const o of outdated) {
            const id = `${o.line}.${o.part}`;
            out.set(id, [...(out.get(id) ?? []), o]);
        }
        return out;
    }, [outdated]);
    // One editor open at a time: the row's id and what it is open for.
    const [editing, setEditing] = useState<{ id: string; state: EditorState } | null>(null);
    const [filter, setFilter] = useState<LineFilter>("all");
    const filterId = useId();
    // Every line's checks and recommendation, for its cell, the filter and the batch approvals.
    const rule: RecommendRule = me.recommendRule === "A" ? "A" : "B";
    const analyses = useMemo(() => lines.map((line, i) => analyse(line, byKey.get(line.editKey), placeOf(storyId, line), neighbourAddress(lines, i), rule)), [lines, byKey, storyId, rule]);
    const shows = (line: TranslatorLine, i: number): boolean => {
        if (filter === "all") return true;
        const place = placeOf(storyId, line);
        const e = byKey.get(line.editKey);
        if (filter === "proposals") return proposalsAt(e, place).length > 0;
        if (filter === "recommended") return analyses[i].recommended !== null;
        if (filter === "outdated") return line.status === "outdated";
        if (filter === "problems") return analyses[i].problems.size > 0;
        if (filter !== "remarks") return analyses[i].problems.has(filter);
        return (e?.comments ?? []).some((c) => c.kind !== "note" && !c.resolvedBy);
    };
    const shown = lines.filter(shows).length;
    const problems = problemOptions(analyses);
    const bulk: IBulkLine[] = lines.map((line, i) => {
        const place = placeOf(storyId, line);
        const e = byKey.get(line.editKey);
        const rec = analyses[i].recommended;
        const recText = rec === "sova" ? line.sova : rec === "ai" ? line.ai : (proposalsAt(e, place).find((p) => `proposal:${p.id}` === rec)?.text ?? null);
        return { n: i + 1, place, recommended: line.editable ? rec : null, recommendedText: recText, approved: line.status === "approved", hasGame: Boolean(line.ruGame) && line.editable, hasProposals: proposalsAt(e, place).length > 0 };
    });
    return (
        <div className="flex flex-col gap-2">
            <div className="flex flex-wrap items-end gap-3">
                <div className="flex w-full flex-col gap-1.5 sm:w-64">
                    <Label htmlFor={filterId}>{t("filter.label")}</Label>
                    <Select value={filter} onValueChange={(v: string | null) => setFilter((v as LineFilter) ?? "all")}>
                        <SelectTrigger id={filterId} size="sm">
                            <SelectValue>{() => filterWord(filter, t, problems.find((p) => p.value === filter)?.count)}</SelectValue>
                        </SelectTrigger>
                        <SelectContent>
                            {FILTERS.map((f) => (
                                <SelectItem key={f} value={f}>
                                    {t(`filter.${f}` as "filter.all")}
                                </SelectItem>
                            ))}
                            <SelectSeparator />
                            {problems.map((p) => (
                                <SelectItem key={p.value} value={p.value} disabled={p.count === 0}>
                                    {filterWord(p.value, t, p.count)}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                </div>
                {filter !== "all" ? <span className="pb-1.5 text-subtle-foreground text-xs">{t("filter.count", { shown, total: lines.length })}</span> : null}
                {isLead(me) ? <RuleSelect rule={rule} t={t} /> : null}
                {isLead(me) ? <BulkApprove lines={bulk} nickname={nickname} t={t} /> : null}
            </div>
            <Table aria-label={label}>
                <TableHeader>
                    <TableRow>
                        <TableHead className="w-12">{t("table.line")}</TableHead>
                        <TableHead className="w-36">{t("table.speaker")}</TableHead>
                        <TableHead>{t("table.ru")}</TableHead>
                        <TableHead>{t("table.en")}</TableHead>
                        {showCn ? <TableHead>{t("table.cn")}</TableHead> : null}
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {lines.map((line, i) => {
                        const id = `${line.line}.${line.part}`;
                        const previous = i > 0 ? lines[i - 1].line : 0;
                        const lineEdits = byKey.get(line.editKey);
                        if (!shows(line, i)) return null;
                        return (
                            <Fragment key={id}>
                                {commands && filter === "all" ? <ServiceRow commands={commandsBetween(commands, previous, line.line)} columns={columns} nameOf={nameOf} t={t} /> : null}
                                <TableRow
                                    id={`n-${i + 1}`}
                                    className={cn("scroll-mt-(--site-header-height) target:bg-primary/10", line.kind === "option" && "bg-primary/5", analyses[i].problems.size > 0 && PROBLEM_ROW)}
                                    data-line={line.line}
                                    data-kind={line.kind}
                                    data-status={line.status}
                                    data-place={`${storyId}|${line.line}|${line.part}`}
                                >
                                    <TableCell className="py-1.5 align-top text-subtle-foreground text-xs tabular-nums leading-snug">
                                        <span className="flex items-center gap-1.5">
                                            {i + 1}
                                            <StatusDot status={line.status} t={t} />
                                        </span>
                                        {commands ? <span className="block font-mono">{t("table.scriptLine", { line: line.line })}</span> : null}
                                    </TableCell>
                                    <TableCell className={PROSE}>
                                        <Speaker line={line} t={t} />
                                    </TableCell>
                                    <TableCell className={PROSE}>
                                        <RuCell
                                            line={line}
                                            analysis={analyses[i]}
                                            place={placeOf(storyId, line)}
                                            nickname={nickname}
                                            showVariants={showVariants}
                                            showDiff={showDiff}
                                            edits={lineEdits}
                                            outdated={staleAt.get(id) ?? []}
                                            me={me}
                                            editor={editing?.id === id ? editing.state : null}
                                            onEditor={(state) => setEditing(state ? { id, state } : null)}
                                            t={t}
                                        />
                                    </TableCell>
                                    <TableCell className={PROSE} data-sel="en">
                                        <StoryText text={line.en} nickname={nickname} quotes={quotesAt(lineEdits, placeOf(storyId, line))} />
                                    </TableCell>
                                    {showCn ? (
                                        <TableCell className={PROSE} data-sel="cn">
                                            {line.speakerCn ? <span className="block text-muted-foreground text-xs">{line.speakerCn}</span> : null}
                                            {line.cn ? <span className="whitespace-pre-line">{line.cn.replaceAll("\\n", "\n")}</span> : null}
                                        </TableCell>
                                    ) : null}
                                </TableRow>
                            </Fragment>
                        );
                    })}
                    {commands && filter === "all" && lines.length > 0 ? <ServiceRow commands={commandsBetween(commands, lines[lines.length - 1].line, Number.POSITIVE_INFINITY)} columns={columns} nameOf={nameOf} t={t} /> : null}
                </TableBody>
            </Table>
        </div>
    );
}
