/**
 * The Search tab: every text of the game (the story, the game data by
 * section, the interface) found by a part of its English or Russian, any
 * case, ё = е. The results are the story table's rows with the same actions:
 * a text that stands in the story is shown and edited at its first story
 * line (Sova's and the AI's variants there), any other as the text itself,
 * whose edit holds everywhere. The query and the filters live in the
 * address, so a search can be shared as a link.
 */
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import type React from "react";
import { useEffect, useId, useMemo, useRef, useState } from "react";
import { SearchField } from "#/components/SearchField";
import { Button } from "#/components/ui/button";
import { Checkbox } from "#/components/ui/checkbox";
import { Collapsible, CollapsiblePanel, CollapsibleTrigger } from "#/components/ui/collapsible";
import { Empty, EmptyHeader, EmptyTitle } from "#/components/ui/empty";
import { Label } from "#/components/ui/label";
import { LoadFailed } from "#/components/ui/load-failed";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "#/components/ui/select";
import { Skeleton } from "#/components/ui/skeleton";
import { Spinner } from "#/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "#/components/ui/table";
import { useDebounce } from "#/hooks/use-debounce";
import { useFormatters, useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { searchLinesQueryOptions, translatorsMeQueryOptions } from "#/lib/translators/api";
import type { KeyEdits } from "#/types/generated/KeyEdits";
import type { SearchLine } from "#/types/generated/SearchLine";
import type { TextAt } from "#/types/generated/TextAt";
import type { TranslatorsMe } from "#/types/generated/TranslatorsMe";
import type { RecommendRule } from "./checks";
import { type EditorState, PlacesList, StatusDot } from "./LineEdits";
import { analyse, PROSE, RuCell, Speaker } from "./LinesTable";
import { GamePlace } from "./PlaceWords";
import { type IFind, StoryText } from "./StoryText";
import { Toggle } from "./SwitchToggle";
import { useStoryNames } from "./storyNames";
import type { messages } from "./translators.messages";
import { fileWord, sectionWord } from "./where";

type T = TypedT<typeof messages>;

export const SEARCH_SECTIONS = ["story", "operators", "enemies", "stages", "items", "archives", "other"] as const;
export const SEARCH_STATUSES = ["unchecked", "proposed", "approved", "untranslated"] as const;
export type SearchSection = (typeof SEARCH_SECTIONS)[number];
export type SearchStatus = (typeof SEARCH_STATUSES)[number];

/** The Search tab's part of the address. */
export interface ISearchState {
    q?: string;
    whole?: boolean;
    section?: SearchSection;
    status?: SearchStatus;
    variants?: false;
    diff?: boolean;
}

/** One place of a found text: a link to the story line, or the game object's field in words. */
function Place({ at, t }: { at: TextAt; t: T }): React.ReactElement {
    const nameOf = useStoryNames();
    if (at.storyId === null) return <GamePlace section={at.section} objectId={at.objectId} objectEn={at.objectEn} field={at.field} idx={at.idx} t={t} />;
    const n = nameOf(at.storyId);
    const label = t("where.item", { group: n?.group ?? "", story: n?.story ?? at.storyId, n: at.ord === null ? "?" : at.ord + 1 });
    if (!n) return <span>{label}</span>;
    return (
        <Link to="/translators" search={{ group: n.groupId, story: at.storyId }} hash={at.ord === null ? undefined : `n-${at.ord + 1}`} className="underline-offset-4 hover:underline">
            {label}
        </Link>
    );
}

/** Where a found text stands: its first place, and "N more" that unfolds every place of it. */
function Where({ row, nickname, t }: { row: SearchLine; nickname: string; t: T }): React.ReactElement {
    const { hit } = row;
    const [open, setOpen] = useState(false);
    const first = hit.at.at(0);
    // The line's count is at least 1 even for a text tied to no place.
    const total = hit.places > 0 ? Math.max(row.line.places, hit.places) : 0;
    const more = total - (first ? 1 : 0);
    return (
        <Collapsible open={open} onOpenChange={setOpen} className="flex flex-col items-start gap-0.5 text-xs">
            {open ? null : first ? (
                <Place at={first} t={t} />
            ) : (
                // An entry of the dictionaries tied to no known place: its section and where it is kept.
                <span title={hit.file ?? undefined}>{[sectionWord(hit.sections.at(0) ?? "other", t), hit.file ? fileWord(hit.file, t) : null].filter(Boolean).join(" › ")}</span>
            )}
            {more > 0 ? <CollapsibleTrigger render={<Button variant="link" size="xs" className="h-auto px-0 text-subtle-foreground text-xs" />}>{open ? t("search.fewer") : t("search.alsoIn", { count: more })}</CollapsibleTrigger> : null}
            <CollapsiblePanel className="w-full">
                <PlacesList place={row.place} nickname={nickname} compact t={t} />
            </CollapsiblePanel>
        </Collapsible>
    );
}

interface IResultsProps {
    rows: readonly SearchLine[];
    edits: readonly KeyEdits[];
    me: TranslatorsMe;
    find: IFind;
    showVariants: boolean;
    showDiff: boolean;
    t: T;
}

function Results({ rows, edits, me, find, showVariants, showDiff, t }: IResultsProps): React.ReactElement {
    const byKey = useMemo(() => new Map(edits.map((e) => [e.editKey, e])), [edits]);
    const rule: RecommendRule = me.recommendRule === "A" ? "A" : "B";
    const nickname = me.nickname ?? t("nickname.fallback");
    const analyses = useMemo(() => rows.map((r) => analyse(r.line, byKey.get(r.line.editKey), r.place, null, rule)), [rows, byKey, rule]);
    // One editor open at a time: the row's text id and what it is open for.
    const [editing, setEditing] = useState<{ id: number; state: EditorState } | null>(null);
    // Who says it, as in the story's table: the column is there once a story line is found.
    const speakers = rows.some((r) => r.line.kind !== "text");
    return (
        // Fixed widths: the Russian and the English share what the narrow columns leave, half each.
        <Table aria-label={t("search.table")} className="wrap-anywhere min-w-3xl table-fixed">
            <TableHeader>
                <TableRow>
                    <TableHead className="w-12">{t("table.line")}</TableHead>
                    <TableHead className="w-56">{t("search.where")}</TableHead>
                    {speakers ? <TableHead className="w-36">{t("table.speaker")}</TableHead> : null}
                    <TableHead>{t("table.ru")}</TableHead>
                    <TableHead>{t("table.en")}</TableHead>
                </TableRow>
            </TableHeader>
            <TableBody>
                {rows.map((r, i) => {
                    const id = r.hit.id;
                    return (
                        <TableRow key={id} data-status={r.line.status}>
                            <TableCell className="py-1.5 align-top text-subtle-foreground text-xs tabular-nums leading-snug">
                                <span className="flex items-center gap-1.5">
                                    {i + 1}
                                    <StatusDot status={r.line.status} t={t} />
                                </span>
                            </TableCell>
                            <TableCell className={PROSE}>
                                <Where row={r} nickname={nickname} t={t} />
                            </TableCell>
                            {speakers ? (
                                <TableCell className={PROSE}>
                                    <Speaker line={r.line} t={t} />
                                </TableCell>
                            ) : null}
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
                                    find={find}
                                    t={t}
                                />
                            </TableCell>
                            <TableCell className={PROSE}>
                                <StoryText text={r.line.en} nickname={nickname} find={find} />
                            </TableCell>
                        </TableRow>
                    );
                })}
            </TableBody>
        </Table>
    );
}

function Found({ query, me, showVariants, showDiff, t }: { query: { q: string; whole: boolean; section: string | null; status: string | null }; me: TranslatorsMe; showVariants: boolean; showDiff: boolean; t: T }): React.ReactElement {
    const f = useFormatters();
    const search = useInfiniteQuery(searchLinesQueryOptions(query));
    const pages = search.data?.pages ?? [];
    const rows = useMemo(() => pages.flatMap((p) => p.rows), [pages]);
    const edits = useMemo(() => {
        // Each page brings its rows' edits; the newest page's copy of an entry wins.
        const out = new Map<string, KeyEdits>();
        for (const p of pages) for (const e of p.edits) out.set(e.editKey, e);
        return [...out.values()];
    }, [pages]);
    if (search.isError) return <LoadFailed message={t("search.load")} onRetry={() => search.refetch()} />;
    if (!search.data) return <Skeleton className="h-96 w-full rounded-2xl" />;
    const first = pages[0];
    if (first.indexed === 0)
        return (
            <Empty>
                <EmptyHeader>
                    <EmptyTitle>{t("empty.notImported")}</EmptyTitle>
                </EmptyHeader>
            </Empty>
        );
    const total = pages[pages.length - 1].total;
    return (
        <div className="flex flex-col gap-2">
            <p className="text-subtle-foreground text-xs">{t("search.found", { count: f.number(total), millis: f.number(first.millis) })}</p>
            {rows.length === 0 ? (
                <Empty>
                    <EmptyHeader>
                        <EmptyTitle>{t("search.none")}</EmptyTitle>
                    </EmptyHeader>
                </Empty>
            ) : (
                <Results rows={rows} edits={edits} me={me} find={{ q: query.q, whole: query.whole }} showVariants={showVariants} showDiff={showDiff} t={t} />
            )}
            {search.hasNextPage ? (
                <div className="flex flex-wrap items-center gap-3">
                    <Button variant="outline" size="sm" onClick={() => search.fetchNextPage()} disabled={search.isFetchingNextPage}>
                        {search.isFetchingNextPage ? <Spinner /> : null}
                        {t("search.more")}
                    </Button>
                    <span className="text-subtle-foreground text-xs">{t("search.shown", { shown: f.number(rows.length), total: f.number(total) })}</span>
                </div>
            ) : null}
        </div>
    );
}

export function SearchTab({ search, onSearch }: { search: ISearchState; onSearch: (next: ISearchState) => void }): React.ReactElement {
    const t: T = useT("translators");
    const me = useQuery(translatorsMeQueryOptions());
    const ids = { whole: useId(), section: useId(), status: useId(), variants: useId(), diff: useId() };
    // The box answers as one types; the address follows once typing stops.
    const [text, setText] = useState(search.q ?? "");
    const settled = useDebounce(text, 400).trim();
    // Only a change of the box moves the address (a link away must not bring the query back).
    const pushed = useRef(settled);
    useEffect(() => {
        if (settled === pushed.current) return;
        pushed.current = settled;
        if (settled !== (search.q ?? "")) onSearch({ ...search, q: settled || undefined });
    }, [settled, search, onSearch]);
    const showVariants = search.variants !== false;
    const showDiff = search.diff === true;
    const q = search.q ?? "";
    const asked = q !== "" || search.section !== undefined || search.status !== undefined;
    return (
        <>
            <div className="flex flex-wrap items-end gap-4">
                <SearchField className="w-full sm:w-80" value={text} onChange={setText} placeholder={t("search.placeholder")} label={t("search.label")} clearLabel={t("search.clear")} />
                {/* biome-ignore lint/a11y/noLabelWithoutControl: the kit Checkbox inside is the control */}
                <label className="flex items-center gap-2 pb-2 text-sm">
                    <Checkbox id={ids.whole} checked={search.whole === true} onCheckedChange={(v) => onSearch({ ...search, whole: v === true || undefined })} />
                    {t("search.whole")}
                </label>
                <div className="flex w-full flex-col gap-1.5 sm:w-56">
                    <Label htmlFor={ids.section}>{t("search.section")}</Label>
                    <Select value={search.section ?? "all"} onValueChange={(v: string | null) => onSearch({ ...search, section: v && v !== "all" ? (v as SearchSection) : undefined })}>
                        <SelectTrigger id={ids.section} size="sm">
                            <SelectValue>{() => t(`search.section.${search.section ?? "all"}` as "search.section.all")}</SelectValue>
                        </SelectTrigger>
                        <SelectContent>
                            {(["all", ...SEARCH_SECTIONS] as const).map((s) => (
                                <SelectItem key={s} value={s}>
                                    {t(`search.section.${s}` as "search.section.all")}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                </div>
                <div className="flex w-full flex-col gap-1.5 sm:w-56">
                    <Label htmlFor={ids.status}>{t("search.status")}</Label>
                    <Select value={search.status ?? "all"} onValueChange={(v: string | null) => onSearch({ ...search, status: v && v !== "all" ? (v as SearchStatus) : undefined })}>
                        <SelectTrigger id={ids.status} size="sm">
                            <SelectValue>{() => t(`search.status.${search.status ?? "all"}` as "search.status.all")}</SelectValue>
                        </SelectTrigger>
                        <SelectContent>
                            {(["all", ...SEARCH_STATUSES] as const).map((s) => (
                                <SelectItem key={s} value={s}>
                                    {t(`search.status.${s}` as "search.status.all")}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                </div>
                <Toggle id={ids.variants} checked={showVariants} onChange={(checked) => onSearch({ ...search, variants: checked ? undefined : false })} label={t("toggle.variants")} hint={t("toggle.variantsHint")} />
                <Toggle id={ids.diff} checked={showDiff} onChange={(checked) => onSearch({ ...search, diff: checked || undefined })} label={t("toggle.diff")} hint={t("toggle.diffHint")} />
            </div>
            {!asked ? (
                <Empty>
                    <EmptyHeader>
                        <EmptyTitle>{t("search.prompt")}</EmptyTitle>
                    </EmptyHeader>
                </Empty>
            ) : me.data ? (
                <Found query={{ q, whole: search.whole === true, section: search.section ?? null, status: search.status ?? null }} me={me.data} showVariants={showVariants} showDiff={showDiff} t={t} />
            ) : (
                <Skeleton className="h-96 w-full rounded-2xl" />
            )}
        </>
    );
}
