/**
 * The Game sections tab: the game's texts outside the story, by section. A
 * section with objects (operators, enemies, stages, items) is the Story
 * tab over again with "Section / Object" for "Chapter / Story": the chosen
 * object's texts as the same dense table, Field | Russian as in the game |
 * English, in the order the game shows them, with the same actions. A text
 * is edited as the text itself: one translation wherever its English stands
 * ("N places" and the confirmation as everywhere). «Interface and other» has
 * no objects: it is the search over that section.
 */
import { useQuery } from "@tanstack/react-query";
import type React from "react";
import { useId, useMemo, useState } from "react";
import { Combobox, ComboboxEmpty, ComboboxInput, ComboboxItem, ComboboxList, ComboboxPopup } from "#/components/ui/combobox";
import { Empty, EmptyHeader, EmptyTitle } from "#/components/ui/empty";
import { Label } from "#/components/ui/label";
import { LoadFailed } from "#/components/ui/load-failed";
import { Meter } from "#/components/ui/meter";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "#/components/ui/select";
import { Skeleton } from "#/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "#/components/ui/table";
import { useFormatters, useLocale, useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { objectLinesQueryOptions, objectsQueryOptions, translatorsMeQueryOptions } from "#/lib/translators/api";
import { cn } from "#/lib/utils";
import type { TranslatorsMe } from "#/types/generated/TranslatorsMe";
import type { RecommendRule } from "./checks";
import { type EditorState, StatusDot } from "./LineEdits";
import { analyse, PROSE, RuCell } from "./LinesTable";
import { type ISearchState, SearchTab } from "./SearchTab";
import { StoryText } from "./StoryText";
import { Toggle } from "./SwitchToggle";
import type { messages } from "./translators.messages";
import { useScrollToRow } from "./useScrollToRow";
import { CARD_SECTIONS, type CardSection, fieldLabel } from "./where";

type T = TypedT<typeof messages>;

/** The sections of the tab, in its order: those with objects, then the list of the rest. */
export const GAME_SECTIONS = [...CARD_SECTIONS, "other"] as const;
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
    // One editor open at a time: the row and what it is open for.
    const [editing, setEditing] = useState<{ id: string; state: EditorState } | null>(null);
    if (query.isError) return <LoadFailed message={t("sections.load.lines")} onRetry={() => query.refetch()} />;
    if (!query.data) return <Skeleton className="h-96 w-full rounded-2xl" />;
    const nickname = me.nickname ?? t("nickname.fallback");
    const share = item.total ? item.approved / item.total : 0;
    const code = query.data.code;
    return (
        <div className="flex flex-col gap-2">
            <p className="text-subtle-foreground text-xs">{t("sections.summary", { count: rows.length, ru: rows.filter((r) => r.line.ruGame).length })}</p>
            <span className="flex items-center gap-2">
                <Meter value={share * 100} aria-label={t("sections.progressLabel")} className="w-32" />
                <span className="text-subtle-foreground text-xs">{t("sections.progressLong", { checked: f.percent(share), waiting: f.number(item.waiting) })}</span>
            </span>
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
                    {rows.map((r, i) => {
                        const id = `${r.field}.${r.idx}.${r.line.editKey}`;
                        return (
                            <TableRow key={id} id={`n-${i + 1}`} className="scroll-mt-(--site-header-height) target:bg-primary/10" data-field={r.field} data-status={r.line.status}>
                                <TableCell className="py-1.5 align-top text-subtle-foreground text-xs tabular-nums leading-snug">
                                    <span className="flex items-center gap-1.5">
                                        {i + 1}
                                        <StatusDot status={r.line.status} t={t} />
                                    </span>
                                </TableCell>
                                <TableCell className={cn(PROSE, "text-muted-foreground")}>{fieldLabel(section, r.field, r.idx, t)}</TableCell>
                                <TableCell className={PROSE}>
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
                                        t={t}
                                    />
                                </TableCell>
                                <TableCell className={PROSE}>
                                    <StoryText text={r.line.en} nickname={nickname} />
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
    const ids = { object: useId(), variants: useId(), diff: useId() };
    const locale = useLocale();
    const me = useQuery(translatorsMeQueryOptions());
    const objects = useQuery(objectsQueryOptions(section));
    const items: IObjectItem[] = useMemo(() => {
        const out = (objects.data ?? []).map((o) => ({ id: o.id, label: locale === "ru" ? (o.ru ?? o.en) : o.en, en: o.en, kind: kindOf(section, o.id), total: o.total, approved: o.approved, waiting: o.waiting }));
        // Operators before their summons and devices; by name, a number as a number (1-2 before 1-10).
        const order = (i: IObjectItem) => (i.kind ? KIND_ORDER[i.kind] : 0);
        return out.sort((a, b) => order(a) - order(b) || a.label.localeCompare(b.label, locale, { numeric: true }));
    }, [objects.data, locale, section]);
    const item = items.find((i) => i.id === search.obj) ?? null;
    const showVariants = search.variants !== false;
    const showDiff = search.diff === true;
    return (
        <>
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
                                                {i.en !== i.label || i.kind ? <span className="truncate text-subtle-foreground text-xs">{[i.en !== i.label ? i.en : null, i.kind ? t(`sections.kind.${i.kind}` as "sections.kind.token") : null].filter(Boolean).join(" · ")}</span> : null}
                                                <ProgressShort item={i} t={t} />
                                            </span>
                                        </ComboboxItem>
                                    )}
                                </ComboboxList>
                            </ComboboxPopup>
                        </Combobox>
                    </div>
                    <Toggle id={ids.variants} checked={showVariants} onChange={(checked) => onSearch({ ...search, variants: checked ? undefined : false })} label={t("toggle.variants")} hint={t("toggle.variantsHint")} />
                    <Toggle id={ids.diff} checked={showDiff} onChange={(checked) => onSearch({ ...search, diff: checked || undefined })} label={t("toggle.diff")} hint={t("toggle.diffHint")} />
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
