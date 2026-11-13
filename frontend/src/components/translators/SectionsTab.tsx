/**
 * The Game sections tab: the game's texts outside the story, by section. A
 * section with objects (operators, enemies, stages, items, and the
 * operators' archives: file, records, stories, voice lines) is the Story
 * tab over again with "Section / Object" for "Chapter / Story": the chosen
 * object's texts as the same dense table, Field | Russian as in the game |
 * English, in the order the game shows them, with the same actions. A text
 * is edited as the text itself: one translation wherever its English stands
 * ("N places" and the confirmation as everywhere). A long text is folded to
 * a few lines until clicked. «Interface and other» has no objects: it is the
 * search over that section.
 */
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { ChevronDown, ChevronLeft, ChevronRight } from "lucide-react";
import type React from "react";
import { useId, useMemo, useState } from "react";
import { Button } from "#/components/ui/button";
import { Combobox, ComboboxEmpty, ComboboxInput, ComboboxItem, ComboboxList, ComboboxPopup } from "#/components/ui/combobox";
import { Empty, EmptyHeader, EmptyTitle } from "#/components/ui/empty";
import { Label } from "#/components/ui/label";
import { LoadFailed } from "#/components/ui/load-failed";
import { Meter } from "#/components/ui/meter";
import { Select, SelectContent, SelectItem, SelectSeparator, SelectTrigger, SelectValue } from "#/components/ui/select";
import { Skeleton } from "#/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "#/components/ui/table";
import { useFormatters, useLocale, useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { objectLinesQueryOptions, objectsQueryOptions, textSummaryQueryOptions, translatorsMeQueryOptions } from "#/lib/translators/api";
import { cn } from "#/lib/utils";
import type { TranslatorsMe } from "#/types/generated/TranslatorsMe";
import type { ProblemKind, RecommendRule } from "./checks";
import { type EditorState, StatusDot } from "./LineEdits";
import { analyse, filterWord, PROBLEM_ROW, PROSE, problemOptions, RuCell } from "./LinesTable";
import { Folded, isLong, useUnfolded } from "./LongText";
import { NextUnchecked, TextCounts } from "./Overview";
import { type ISearchState, SearchTab } from "./SearchTab";
import { StoryText } from "./StoryText";
import { useStoryNames } from "./storyNames";
import type { messages } from "./translators.messages";
import { useScrollToRow } from "./useScrollToRow";
import { ViewMenu, viewOf } from "./ViewMenu";
import { type CardSection, fieldLabel, LEVEL_FIELD, noteStories, noteWords } from "./where";

type T = TypedT<typeof messages>;

/** The sections of the tab, in its order: the game's objects, the list of the rest, and the operators' archives last. */
export const GAME_SECTIONS = ["operators", "enemies", "stages", "items", "other", "archives"] as const satisfies readonly (CardSection | "other")[];
export type GameSection = (typeof GAME_SECTIONS)[number];

/** The tab's part of the address (the list of «Interface and other» also reads the search's). */
export interface ISectionsState extends ISearchState {
    /** The section; the first one when left out. */
    sec?: GameSection;
    /** The object, as the backend names it (`GameObject.id`). */
    obj?: string;
}

interface IObjectItem {
    id: string;
    /** In the section's language (the game's Russian when there is one). */
    label: string;
    /** In English, shown small and searched too. */
    en: string;
    /** An operator's record that is not an operator: a summon or a device. */
    kind: "token" | "trap" | null;
    total: number;
    approved: number;
    waiting: number;
    /** Other records of the game with the same name and texts: this card is theirs too. */
    also: readonly string[];
    /** What tells it from a different record of the same name. */
    hint: string | null;
}

/** Search folding: any case, ё = е. */
function fold(s: string): string {
    return s.toLowerCase().replaceAll("ё", "е");
}

function kindOf(section: CardSection, id: string): IObjectItem["kind"] {
    if (section !== "operators") return null;
    return id.startsWith("token_") ? "token" : id.startsWith("trap_") ? "trap" : null;
}

const KIND_ORDER = { token: 1, trap: 2 } as const;

/** How many objects the open picker lists before a name is typed (a section has up to 2,500). */
const PICKER_LIMIT = 200;

/** "12 % checked · 3 waiting", only once something happened. */
function ProgressShort({ item, t }: { item: IObjectItem; t: T }): React.ReactElement | null {
    const f = useFormatters();
    if (item.approved + item.waiting === 0) return null;
    return <span className="truncate text-subtle-foreground text-xs">{t("sections.progress", { checked: f.percent(item.total ? item.approved / item.total : 0), waiting: f.number(item.waiting) })}</span>;
}

/** Under an object's name in the picker: its English, what kind of record it is, and what tells it from its namesakes. */
function ObjectAbout({ item, t }: { item: IObjectItem; t: T }): React.ReactElement | null {
    const words = [item.en !== item.label ? item.en : null, item.kind ? t(`sections.kind.${item.kind}` as "sections.kind.token") : null, item.also.length > 0 ? t("sections.same", { count: item.also.length + 1 }) : null].filter(Boolean);
    if (words.length === 0 && !item.hint) return null;
    return (
        <>
            {words.length > 0 ? <span className="truncate text-subtle-foreground text-xs">{words.join(" · ")}</span> : null}
            {item.hint ? <span className="truncate text-subtle-foreground text-xs">{item.hint}</span> : null}
        </>
    );
}

type CardFilter = "all" | "proposals" | "problems" | ProblemKind;

interface ICardProps {
    section: CardSection;
    item: IObjectItem;
    me: TranslatorsMe;
    showVariants: boolean;
    showDiff: boolean;
    t: T;
}

/** One object's texts: the story's table with the field for the speaker. */
function Card({ section, item, me, showVariants, showDiff, t }: ICardProps): React.ReactElement {
    const f = useFormatters();
    const query = useQuery(objectLinesQueryOptions(section, item.id));
    useScrollToRow(Boolean(query.data));
    const rows = query.data?.rows ?? [];
    const edits = query.data?.edits ?? [];
    const byKey = useMemo(() => new Map(edits.map((e) => [e.editKey, e])), [edits]);
    const rule: RecommendRule = me.recommendRule === "A" ? "A" : "B";
    const analyses = useMemo(() => rows.map((r) => analyse(r.line, byKey.get(r.line.editKey), r.place, null, rule)), [rows, byKey, rule]);
    // Fields that several texts share («Talent 1 · description» at each elite phase): those rows say what tells them apart.
    const repeated = useMemo(() => {
        const count = new Map<string, number>();
        for (const r of rows) count.set(`${r.field}.${r.idx}`, (count.get(`${r.field}.${r.idx}`) ?? 0) + 1);
        return count;
    }, [rows]);
    // One editor open at a time: the row and what it is open for.
    const [editing, setEditing] = useState<{ id: string; state: EditorState } | null>(null);
    const unfolded = useUnfolded();
    // Which skills' level entries are unfolded (by the skill's number).
    const levelsOpen = useUnfolded();
    // The rows in the card's order; a skill's level entries after its description, behind a fold.
    const order = useMemo(() => {
        const out: ({ kind: "row"; i: number } | { kind: "levels"; idx: number; count: number })[] = [];
        const levels = new Map<number, number[]>();
        rows.forEach((r, i) => {
            if (r.field === LEVEL_FIELD) levels.set(r.idx, [...(levels.get(r.idx) ?? []), i]);
        });
        const placed = new Set<number>();
        rows.forEach((r, i) => {
            if (r.field === LEVEL_FIELD) return;
            out.push({ kind: "row", i });
            const next = rows[i + 1];
            const last = r.field === "skill.description" && (!next || next.field !== "skill.description" || next.idx !== r.idx);
            const list = levels.get(r.idx);
            if (last && list && !placed.has(r.idx)) {
                placed.add(r.idx);
                out.push({ kind: "levels", idx: r.idx, count: list.length });
                if (levelsOpen.has(String(r.idx))) for (const j of list) out.push({ kind: "row", i: j });
            }
        });
        // Level entries of a skill without a description row: listed where they are.
        for (const [idx, list] of levels) if (!placed.has(idx)) for (const j of list) out.push({ kind: "row", i: j });
        return out;
    }, [rows, levelsOpen]);
    const storyOf = useStoryNames();
    const filterId = useId();
    const [filter, setFilter] = useState<CardFilter>("all");
    const problems = problemOptions(analyses);
    const shows = (i: number) => (filter === "all" ? true : filter === "proposals" ? rows[i].line.status === "proposed" : filter === "problems" ? analyses[i].problems.size > 0 : analyses[i].problems.has(filter));
    if (query.isError) return <LoadFailed message={t("sections.load.lines")} onRetry={() => query.refetch()} />;
    if (!query.data) return <Skeleton className="h-96 w-full rounded-2xl" />;
    const nickname = me.nickname ?? t("nickname.fallback");
    const share = item.total ? item.approved / item.total : 0;
    const code = query.data.code;
    return (
        <div className="flex flex-col gap-2">
            <p className="text-subtle-foreground text-xs">{t("sections.summary", { count: rows.length, ru: rows.filter((r) => r.line.ruGame).length })}</p>
            <div className="flex flex-wrap items-end gap-3">
                <span className="flex items-center gap-2 pb-1.5">
                    <Meter value={share * 100} aria-label={t("sections.progressLabel")} className="w-32" />
                    <span className="text-subtle-foreground text-xs">{t("sections.progressLong", { checked: f.percent(share), waiting: f.number(item.waiting) })}</span>
                </span>
                <div className="flex w-full flex-col gap-1.5 sm:w-64">
                    <Label htmlFor={filterId}>{t("filter.label")}</Label>
                    <Select value={filter} onValueChange={(v: string | null) => setFilter((v as CardFilter) ?? "all")}>
                        <SelectTrigger id={filterId} size="sm">
                            <SelectValue>{() => filterWord(filter, t, problems.find((p) => p.value === filter)?.count)}</SelectValue>
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value="all">{t("filter.all")}</SelectItem>
                            <SelectItem value="proposals">{t("filter.proposals")}</SelectItem>
                            <SelectSeparator />
                            {problems.map((p) => (
                                <SelectItem key={p.value} value={p.value} disabled={p.count === 0}>
                                    {filterWord(p.value, t, p.count)}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                </div>
                <NextUnchecked t={t} />
            </div>
            {/* Fixed widths: the Russian and the English share what the narrow columns leave, half each. */}
            <Table aria-label={t("sections.table", { object: item.label })} className="wrap-anywhere min-w-3xl table-fixed">
                <TableHeader>
                    <TableRow>
                        <TableHead className="w-12">{t("table.line")}</TableHead>
                        <TableHead className="w-44">{t("sections.field")}</TableHead>
                        <TableHead>{t("table.ru")}</TableHead>
                        <TableHead>{t("table.en")}</TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {code ? (
                        // A stage's code: shown where the game shows it, never translated.
                        <TableRow className="text-subtle-foreground">
                            <TableCell className="py-1.5" />
                            <TableCell className={PROSE}>{t("sections.code")}</TableCell>
                            <TableCell className={PROSE}>{code}</TableCell>
                            <TableCell className={PROSE}>{code}</TableCell>
                        </TableRow>
                    ) : null}
                    {order.map((entry) => {
                        if (entry.kind === "levels") {
                            // The entries with a skill level's numbers, folded under the skill's description.
                            const open = levelsOpen.has(String(entry.idx));
                            return (
                                <TableRow key={`levels-${entry.idx}`} className="bg-muted/30 hover:bg-muted/30">
                                    <TableCell className="py-1" />
                                    <TableCell colSpan={3} className="py-1">
                                        <Button variant="link" size="xs" className="h-auto px-0 text-xs" aria-expanded={open} onClick={() => levelsOpen.toggle(String(entry.idx))}>
                                            {open ? <ChevronDown aria-hidden="true" /> : <ChevronRight aria-hidden="true" />}
                                            {t("sections.levels", { count: entry.count })}
                                        </Button>
                                    </TableCell>
                                </TableRow>
                            );
                        }
                        const i = entry.i;
                        const r = rows[i];
                        if (!shows(i)) return null;
                        const id = `${r.field}.${r.idx}.${r.line.editKey}`;
                        const long = isLong(r.line.en, r.line.ruGame);
                        // An open editor needs the whole row.
                        const folded = long && !unfolded.has(id) && editing?.id !== id;
                        return (
                            <TableRow key={id} id={`n-${i + 1}`} className={cn("scroll-mt-(--site-header-height) target:bg-primary/10", analyses[i].problems.size > 0 && PROBLEM_ROW)} data-field={r.field} data-status={r.line.status} data-place={`${r.place.storyId}|0|0`}>
                                <TableCell className="py-1.5 align-top text-subtle-foreground text-xs tabular-nums leading-snug">
                                    <span className="flex items-center gap-1.5">
                                        {i + 1}
                                        <StatusDot status={r.line.status} t={t} />
                                    </span>
                                </TableCell>
                                <TableCell className={cn(PROSE, "text-muted-foreground", r.field === LEVEL_FIELD && "ps-6")}>
                                    {fieldLabel(section, r.field, r.idx, t, r.note)}
                                    {r.field !== LEVEL_FIELD && r.note && (repeated.get(`${r.field}.${r.idx}`) ?? 0) > 1 && noteWords(r.note, t) ? <span className="block text-subtle-foreground text-xs">{noteWords(r.note, t)}</span> : null}
                                    {noteStories(r.note).map((storyId) => {
                                        const story = storyOf(storyId);
                                        return story ? (
                                            <Link key={storyId} to="/translators" search={{ group: story.groupId, story: storyId }} className="block text-xs underline-offset-4 hover:underline">
                                                {t("sections.readStory")}
                                            </Link>
                                        ) : null;
                                    })}
                                    {long ? (
                                        <Button variant="link" size="xs" className="block h-auto px-0 text-xs" onClick={() => unfolded.toggle(id)} disabled={editing?.id === id}>
                                            {folded ? t("long.show") : t("long.hide")}
                                        </Button>
                                    ) : null}
                                </TableCell>
                                <TableCell className={PROSE}>
                                    <Folded folded={folded} onOpen={() => unfolded.toggle(id)}>
                                        <RuCell
                                            line={r.line}
                                            analysis={analyses[i]}
                                            place={r.place}
                                            nickname={nickname}
                                            showVariants={showVariants}
                                            showDiff={showDiff}
                                            edits={byKey.get(r.line.editKey)}
                                            outdated={[]}
                                            me={me}
                                            editor={editing?.id === id ? editing.state : null}
                                            onEditor={(state) => setEditing(state ? { id, state } : null)}
                                            here={{ section, objectId: r.objectId, objectEn: item.en, field: r.field, idx: r.idx }}
                                            t={t}
                                        />
                                    </Folded>
                                </TableCell>
                                <TableCell className={PROSE} data-sel="en">
                                    <Folded folded={folded} onOpen={() => unfolded.toggle(id)}>
                                        <StoryText text={r.line.en} nickname={nickname} />
                                    </Folded>
                                </TableCell>
                            </TableRow>
                        );
                    })}
                </TableBody>
            </Table>
        </div>
    );
}

interface ICardsProps {
    section: CardSection;
    search: ISectionsState;
    onSearch: (next: ISectionsState) => void;
    /** The section picker, first in the row. */
    lead: React.ReactNode;
    t: T;
}

/** A section with objects: the object picker, the switches and the chosen object's card. */
function Cards({ section, search, onSearch, lead, t }: ICardsProps): React.ReactElement {
    const ids = { object: useId() };
    const locale = useLocale();
    const me = useQuery(translatorsMeQueryOptions());
    const objects = useQuery(objectsQueryOptions(section));
    const items: IObjectItem[] = useMemo(() => {
        const out = (objects.data ?? []).map((o) => ({ id: o.id, label: locale === "ru" ? (o.ru ?? o.en) : o.en, en: o.en, kind: kindOf(section, o.id), total: o.total, approved: o.approved, waiting: o.waiting, also: o.also, hint: o.hint }));
        // Operators before their summons and devices; by name, a number as a number (1-2 before 1-10).
        const order = (i: IObjectItem) => (i.kind ? KIND_ORDER[i.kind] : 0);
        return out.sort((a, b) => order(a) - order(b) || a.label.localeCompare(b.label, locale, { numeric: true }));
    }, [objects.data, locale, section]);
    // A record merged into another's card (the same name and texts) opens that card.
    const item = items.find((i) => i.id === search.obj) ?? items.find((i) => search.obj !== undefined && i.also.includes(search.obj)) ?? null;
    const view = viewOf(search);
    const showVariants = view.variants;
    const showDiff = view.diff;
    const summary = useQuery({ ...textSummaryQueryOptions({ q: "", whole: false, section, status: null }), enabled: Boolean(me.data) });
    const at = item ? items.indexOf(item) : -1;
    const step = (by: number) => {
        const next = items[at + by];
        if (next) onSearch({ ...search, obj: next.id });
    };
    return (
        <>
            {summary.data ? <TextCounts total={summary.data.total} ru={summary.data.ru} approved={summary.data.approved} t={t} /> : null}
            {objects.isError ? (
                <LoadFailed message={t("sections.load.objects")} onRetry={() => objects.refetch()} />
            ) : (
                <div className="flex flex-wrap items-end gap-4">
                    {lead}
                    <div className="flex w-full flex-col gap-1.5 sm:w-80">
                        <Label htmlFor={ids.object}>{t("sections.object")}</Label>
                        <Combobox<IObjectItem, false>
                            key={section}
                            items={items}
                            value={item}
                            limit={PICKER_LIMIT}
                            onValueChange={(v) => v && onSearch({ ...search, obj: v.id })}
                            itemToStringLabel={(i) => i.label}
                            itemToStringValue={(i) => i.id}
                            filter={(i: IObjectItem, q: string) => fold(`${i.label} ${i.en}`).includes(fold(q.trim()))}
                        >
                            <ComboboxInput id={ids.object} placeholder={t("sections.objectPlaceholder", { count: items.length })} disabled={!objects.data} />
                            <ComboboxPopup>
                                <ComboboxEmpty>{t("sections.objectEmpty")}</ComboboxEmpty>
                                <ComboboxList>
                                    {(i: IObjectItem) => (
                                        <ComboboxItem key={i.id} value={i}>
                                            <span className="flex min-w-0 flex-col">
                                                <span className="truncate">{i.label}</span>
                                                <ObjectAbout item={i} t={t} />
                                                <ProgressShort item={i} t={t} />
                                            </span>
                                        </ComboboxItem>
                                    )}
                                </ComboboxList>
                            </ComboboxPopup>
                        </Combobox>
                    </div>
                    <div className="flex items-center gap-1">
                        <Button variant="outline" size="sm" onClick={() => step(-1)} disabled={at <= 0}>
                            <ChevronLeft aria-hidden="true" />
                            {t("overview.previous")}
                        </Button>
                        <Button variant="outline" size="sm" onClick={() => step(1)} disabled={at < 0 || at >= items.length - 1}>
                            {t("overview.next")}
                            <ChevronRight aria-hidden="true" />
                        </Button>
                    </div>
                    <ViewMenu search={search} onSearch={onSearch} />
                </div>
            )}
            {item && me.data ? (
                <Card key={`${section}/${item.id}`} section={section} item={item} me={me.data} showVariants={showVariants} showDiff={showDiff} t={t} />
            ) : objects.data ? (
                <Empty>
                    <EmptyHeader>
                        <EmptyTitle>{t("sections.choose")}</EmptyTitle>
                    </EmptyHeader>
                </Empty>
            ) : objects.isError ? null : (
                <Skeleton className="h-96 w-full rounded-2xl" />
            )}
        </>
    );
}

export function SectionsTab({ search, onSearch }: { search: ISectionsState; onSearch: (next: ISectionsState) => void }): React.ReactElement {
    const t: T = useT("translators");
    const id = useId();
    const section: GameSection = search.sec ?? GAME_SECTIONS[0];
    const lead = (
        <div className="flex w-full flex-col gap-1.5 sm:w-56">
            <Label htmlFor={id}>{t("sections.section")}</Label>
            <Select value={section} onValueChange={(v: string | null) => v && v !== section && onSearch({ ...search, sec: v === GAME_SECTIONS[0] ? undefined : (v as GameSection), obj: undefined })}>
                <SelectTrigger id={id}>
                    <SelectValue>{() => t(`search.section.${section}` as "search.section.all")}</SelectValue>
                </SelectTrigger>
                <SelectContent>
                    {GAME_SECTIONS.map((s) => (
                        <SelectItem key={s} value={s}>
                            {t(`search.section.${s}` as "search.section.all")}
                        </SelectItem>
                    ))}
                </SelectContent>
            </Select>
        </div>
    );
    if (section === "other") return <SearchTab search={search} onSearch={onSearch} only="other" lead={lead} />;
    return <Cards section={section} search={search} onSearch={onSearch} lead={lead} t={t} />;
}
