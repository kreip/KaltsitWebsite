/**
 * THE TRANSLATORS' SECTION (`/translators`), for members of the team only
 * (`translators.members`); anyone else sees "No access" and no data.
 *
 * Story tab: pick a chapter and a story, read its lines as Russian as the game
 * shows it now | English | Chinese, with the glossary's terms, the
 * translators' variants (translatorVariants.ts) and the machine's. Search tab: every text of the game (SearchTab.tsx).
 * Game sections tab: the texts outside the story, an object at a time
 * (SectionsTab.tsx).
 * Glossary tab: the team's whole glossary and the terms suggested for it.
 * A word selected in any table opens a small menu (SelectionMenu.tsx). The chapter list
 * is the story library's own index; the lines come from the section's import
 * (`/api/translators`).
 */
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { BookCheck, BookOpen, NotebookPen } from "lucide-react";
import type React from "react";
import { useId, useMemo } from "react";
import { Button } from "#/components/ui/button";
import { Collapsible, CollapsiblePanel, CollapsibleTrigger } from "#/components/ui/collapsible";
import { Combobox, ComboboxEmpty, ComboboxInput, ComboboxItem, ComboboxList, ComboboxPopup } from "#/components/ui/combobox";
import { Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyTitle } from "#/components/ui/empty";
import { Label } from "#/components/ui/label";
import { LoadFailed } from "#/components/ui/load-failed";
import { Meter } from "#/components/ui/meter";
import { PageHeader } from "#/components/ui/page-header";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "#/components/ui/select";
import { Skeleton } from "#/components/ui/skeleton";
import { Tabs, TabsList, TabsPanel, TabsTab } from "#/components/ui/tabs";
import { Toggle as KitToggle } from "#/components/ui/toggle";
import { storyIndexQueryOptions, storyQueryOptions } from "#/lib/api/story";
import { useFormatters, useLocale, useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { storyLabel } from "#/lib/story/book/book";
import { charactersQueryOptions, progressQueryOptions, setStoryReadFn, translatorLinesQueryOptions, translatorsMeQueryOptions } from "#/lib/translators/api";
import type { StoryRead } from "#/types/generated/StoryRead";
import type { TranslatorsMe } from "#/types/generated/TranslatorsMe";
import { GlossaryTab } from "./GlossaryTab";
import { TONE, Toned, useEditMutation } from "./LineEdits";
import { LinesTable } from "./LinesTable";
import { MyEditsTab } from "./MyEditsTab";
import { type SearchSection, type SearchStatus, SearchTab } from "./SearchTab";
import { SectionLocale } from "./SectionLocale";
import { type GameSection, SectionsTab } from "./SectionsTab";
import { SelectionMenu } from "./SelectionMenu";
import { Toggle } from "./SwitchToggle";
import { charBase } from "./service";
import { entryLabel, groupLabel, useRename } from "./storyNames";
import type { messages } from "./translators.messages";
import { translatorName } from "./translatorVariants";
import { useScrollToRow } from "./useScrollToRow";

type T = TypedT<typeof messages>;

export type TranslatorsTab = "story" | "search" | "sections" | "glossary" | "edits";

export interface ITranslatorsSearch {
    tab?: TranslatorsTab;
    group?: string;
    story?: string;
    cn?: boolean;
    /** Script line numbers and stage commands between the lines. */
    service?: boolean;
    /** Only `false` is kept: the variants show by default. */
    variants?: false;
    /** Compare: proposals as their changes against the text one step back. */
    diff?: boolean;
    /** Search: the text looked for, whole words only, the section and the state. */
    q?: string;
    whole?: boolean;
    section?: SearchSection;
    status?: SearchStatus;
    /** Game sections: the section and the object. */
    sec?: GameSection;
    obj?: string;
    /** Glossary: the text looked for. */
    gq?: string;
}

interface IGroupItem {
    id: string;
    /** In the section's language (the game's Russian when there is one). */
    label: string;
    /** In English, shown small and searched too. */
    en: string;
    kind: string;
    progress: IProgress;
}

/** Search folding: any case, ё = е. */
function fold(s: string): string {
    return s.toLowerCase().replaceAll("ё", "е");
}

interface ILinesProps {
    storyId: string;
    title: string;
    showCn: boolean;
    showService: boolean;
    showVariants: boolean;
    showDiff: boolean;
    t: T;
}

interface IProgress {
    total: number;
    approved: number;
    waiting: number;
    outdated: number;
}

const NO_PROGRESS: IProgress = { total: 0, approved: 0, waiting: 0, outdated: 0 };

function addProgress(a: IProgress, b: IProgress): IProgress {
    return { total: a.total + b.total, approved: a.approved + b.approved, waiting: a.waiting + b.waiting, outdated: a.outdated + b.outdated };
}

/** Every story's progress by id, from one request shared by the pickers and the table. */
function useProgress(): Map<string, IProgress> {
    const query = useQuery(progressQueryOptions());
    return useMemo(() => new Map((query.data ?? []).map((p) => [p.storyId, p])), [query.data]);
}

/** "12 % checked · 3 waiting · 1 outdated", only once something happened. */
function ProgressShort({ p, t }: { p: IProgress; t: T }): React.ReactElement | null {
    const f = useFormatters();
    if (p.approved + p.waiting + p.outdated === 0) return null;
    return <span className="truncate text-subtle-foreground text-xs">{t("progress.short", { checked: f.percent(p.total ? p.approved / p.total : 0), waiting: f.number(p.waiting), outdated: f.number(p.outdated) })}</span>;
}

/** The story's progress and who read it, with my "I have read this scene". */
function StoryStatus({ storyId, reads, me, t }: { storyId: string; reads: readonly StoryRead[]; me: TranslatorsMe; t: T }): React.ReactElement {
    const f = useFormatters();
    const p = useProgress().get(storyId) ?? NO_PROGRESS;
    const mine = reads.some((r) => r.by.id === me.userId);
    const mark = useEditMutation((read: boolean) => setStoryReadFn({ data: { storyId, read } }), t("read.failed"));
    const share = p.total ? p.approved / p.total : 0;
    return (
        <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
            <KitToggle size="sm" variant="outline" pressed={mine} onPressedChange={(read: boolean) => mark.mutate(read)} disabled={mark.isPending}>
                <BookCheck aria-hidden="true" />
                {t("read.button")}
            </KitToggle>
            {reads.length > 0 ? <span className="text-muted-foreground text-xs">{t("read.by", { names: reads.map((r) => t("read.item", { name: r.by.name, date: f.date(r.at * 1000, { dateStyle: "medium" }) })).join(", ") })}</span> : null}
            <span className="flex items-center gap-2">
                <Meter value={share * 100} aria-label={t("progress.label")} className="w-32" />
                <span className="text-subtle-foreground text-xs">{t("progress.summary", { checked: f.percent(share), waiting: f.number(p.waiting), outdated: f.number(p.outdated) })}</span>
            </span>
        </div>
    );
}

/** A translator's footnotes to the chapter (arktr brings Sova's), folded above the table, each under its author's name. */
function TranslatorNotes({ notes, t }: { notes: readonly string[]; t: T }): React.ReactElement | null {
    const locale = useLocale();
    if (notes.length === 0) return null;
    return (
        <Collapsible>
            <CollapsibleTrigger render={<Button variant="ghost" size="sm" />}>
                <NotebookPen aria-hidden="true" />
                {t("translatorNotes.title", { count: notes.length })}
            </CollapsibleTrigger>
            <CollapsiblePanel className="flex flex-col gap-2 pt-2">
                {notes.map((n) => (
                    <Toned key={n} tone={TONE.note} label={t("translatorNotes.author", { name: translatorName("sova", locale) })}>
                        {n}
                    </Toned>
                ))}
            </CollapsiblePanel>
        </Collapsible>
    );
}

function Lines({ storyId, title, showCn, showService, showVariants, showDiff, t }: ILinesProps): React.ReactElement {
    const query = useQuery(translatorLinesQueryOptions(storyId));
    const me = useQuery(translatorsMeQueryOptions());
    // The stage commands come from the site's own parse of the EN script; the sprites' names from the import.
    const script = useQuery({ ...storyQueryOptions(storyId, "en"), enabled: showService });
    const chars = useQuery({ ...charactersQueryOptions(), enabled: showService });
    const locale = useLocale();
    useScrollToRow(Boolean(query.data && me.data));
    const nameOf = useMemo(() => {
        const byId = new Map((chars.data ?? []).map((c) => [c.charId, c]));
        return (id: string) => {
            const c = byId.get(charBase(id));
            if (!c) return charBase(id);
            const name = locale === "ru" ? (c.nameRu ?? c.nameEn) : c.nameEn;
            return c.guessed ? t("service.guessed", { name }) : name;
        };
    }, [chars.data, locale, t]);
    if (query.isError) return <LoadFailed message={t("load.lines")} onRetry={() => query.refetch()} />;
    if (!query.data || !me.data) return <Skeleton className="h-96 w-full rounded-2xl" />;
    const { lines, importedAt, edits, outdated, sovaNotes, reads } = query.data;
    const nickname = me.data.nickname ?? t("nickname.fallback");
    if (importedAt === null)
        return (
            <Empty>
                <EmptyHeader>
                    <EmptyTitle>{t("empty.notImported")}</EmptyTitle>
                </EmptyHeader>
            </Empty>
        );
    if (lines.length === 0)
        return (
            <Empty>
                <EmptyHeader>
                    <EmptyTitle>{t("empty.noLines")}</EmptyTitle>
                </EmptyHeader>
            </Empty>
        );
    const withRu = lines.filter((l) => l.ruGame).length;
    return (
        <div className="flex flex-col gap-2">
            <p className="text-subtle-foreground text-xs">{t("table.summary", { count: lines.length, ru: withRu })}</p>
            <StoryStatus storyId={storyId} reads={reads} me={me.data} t={t} />
            <TranslatorNotes notes={sovaNotes} t={t} />
            <LinesTable storyId={storyId} lines={lines} edits={edits} outdated={outdated} me={me.data} label={t("table.label", { story: title })} showCn={showCn} showVariants={showVariants} showDiff={showDiff} commands={showService ? (script.data?.commands ?? null) : null} nameOf={nameOf} nickname={nickname} />
        </div>
    );
}

interface ISearchProps {
    search: ITranslatorsSearch;
    onSearch: (next: ITranslatorsSearch) => void;
}

function StoryTab({ search, onSearch }: ISearchProps): React.ReactElement {
    const t: T = useT("translators");
    const ids = { chapter: useId(), story: useId(), cn: useId(), service: useId(), variants: useId(), diff: useId() };
    const index = useQuery(storyIndexQueryOptions("en"));

    const groups = index.data?.groups ?? [];
    const rename = useRename();
    const progress = useProgress();
    const items: IGroupItem[] = useMemo(() => groups.map((g) => ({ id: g.id, label: groupLabel(g, rename), en: groupLabel(g), kind: g.category, progress: g.stories.reduce((acc, st) => addProgress(acc, progress.get(st.id) ?? NO_PROGRESS), NO_PROGRESS) })), [groups, rename, progress]);
    const group = groups.find((g) => g.id === search.group) ?? null;
    const groupItem = items.find((i) => i.id === search.group) ?? null;
    const stories = useMemo(() => (group ? [...group.stories].sort((a, b) => a.sort - b.sort) : []), [group]);
    const story = stories.find((s) => s.id === search.story) ?? null;
    const showCn = search.cn === true;
    const showService = search.service === true;
    const showVariants = search.variants !== false;
    const showDiff = search.diff === true;

    return (
        <>
            {index.isError ? (
                <LoadFailed message={t("load.index")} onRetry={() => index.refetch()} />
            ) : (
                <div className="flex flex-wrap items-end gap-4">
                    <div className="flex w-full flex-col gap-1.5 sm:w-80">
                        <Label htmlFor={ids.chapter}>{t("picker.chapter")}</Label>
                        <Combobox<IGroupItem, false>
                            items={items}
                            value={groupItem}
                            onValueChange={(v) => v && onSearch({ ...search, group: v.id, story: undefined })}
                            itemToStringLabel={(i) => i.label}
                            itemToStringValue={(i) => i.id}
                            filter={(i: IGroupItem, q: string) => fold(`${i.label} ${i.en}`).includes(fold(q.trim()))}
                        >
                            <ComboboxInput id={ids.chapter} placeholder={t("picker.chapterPlaceholder", { count: items.length })} disabled={!index.data} />
                            <ComboboxPopup>
                                <ComboboxEmpty>{t("picker.chapterEmpty")}</ComboboxEmpty>
                                <ComboboxList>
                                    {(item: IGroupItem) => (
                                        <ComboboxItem key={item.id} value={item}>
                                            <span className="flex min-w-0 flex-col">
                                                <span className="truncate">{item.label}</span>
                                                <span className="truncate text-subtle-foreground text-xs">
                                                    {item.en !== item.label ? `${item.en} · ` : ""}
                                                    {t(`category.${item.kind}` as "category.main")}
                                                </span>
                                                <ProgressShort p={item.progress} t={t} />
                                            </span>
                                        </ComboboxItem>
                                    )}
                                </ComboboxList>
                            </ComboboxPopup>
                        </Combobox>
                    </div>

                    <div className="flex w-full flex-col gap-1.5 sm:w-80">
                        <Label htmlFor={ids.story}>{t("picker.story")}</Label>
                        <Select value={story?.id ?? null} onValueChange={(v: string | null) => v && onSearch({ ...search, story: v })} disabled={!group}>
                            <SelectTrigger id={ids.story}>
                                <SelectValue placeholder={t("picker.storyPlaceholder")}>{() => (story ? entryLabel(story, rename) : t("picker.storyPlaceholder"))}</SelectValue>
                            </SelectTrigger>
                            <SelectContent>
                                {stories.map((s) => (
                                    <SelectItem key={s.id} value={s.id}>
                                        <span className="flex min-w-0 flex-col">
                                            <span className="truncate">
                                                {entryLabel(s, rename)}
                                                {s.avgTag ? <span className="text-subtle-foreground"> · {rename(s.avgTag)}</span> : null}
                                            </span>
                                            {rename(s.name) !== s.name ? <span className="truncate text-subtle-foreground text-xs">{storyLabel(s)}</span> : null}
                                            <ProgressShort p={progress.get(s.id) ?? NO_PROGRESS} t={t} />
                                        </span>
                                    </SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                    </div>

                    <Toggle id={ids.variants} checked={showVariants} onChange={(checked) => onSearch({ ...search, variants: checked ? undefined : false })} label={t("toggle.variants")} hint={t("toggle.variantsHint")} />
                    <Toggle id={ids.diff} checked={showDiff} onChange={(checked) => onSearch({ ...search, diff: checked || undefined })} label={t("toggle.diff")} hint={t("toggle.diffHint")} />
                    <Toggle id={ids.cn} checked={showCn} onChange={(checked) => onSearch({ ...search, cn: checked || undefined })} label={t("toggle.cn")} hint={t("toggle.cnHint")} />
                    <Toggle id={ids.service} checked={showService} onChange={(checked) => onSearch({ ...search, service: checked || undefined })} label={t("toggle.service")} hint={t("toggle.serviceHint")} />

                    {story && group ? (
                        <Button variant="ghost" size="sm" render={<Link to="/translators/book" search={{ group: group.id, story: story.id }} />}>
                            <BookOpen />
                            {t("book.open")}
                        </Button>
                    ) : null}
                </div>
            )}

            {story ? (
                <Lines storyId={story.id} title={entryLabel(story, rename)} showCn={showCn} showService={showService} showVariants={showVariants} showDiff={showDiff} t={t} />
            ) : (
                <Empty>
                    <EmptyHeader>
                        <EmptyTitle>{t("empty.choose")}</EmptyTitle>
                    </EmptyHeader>
                </Empty>
            )}
        </>
    );
}

/** "No access" for anyone outside the team, with the development sign-in when it is switched on. */
export function NoAccess({ devLogin }: { devLogin: boolean }): React.ReactElement {
    const t: T = useT("translators");
    return (
        <Empty>
            <EmptyHeader>
                <EmptyTitle>{t("access.title")}</EmptyTitle>
                <EmptyDescription>{t("access.description")}</EmptyDescription>
            </EmptyHeader>
            {devLogin ? (
                <EmptyContent>
                    <Button variant="outline" size="sm" render={<Link to="/translators/dev-login" />}>
                        {t("devLogin.link")}
                    </Button>
                </EmptyContent>
            ) : null}
        </Empty>
    );
}

/** Renders `children` for a member of the section only: a skeleton while checking, else "No access". */
export function MembersOnly({ children }: { children: React.ReactNode }): React.ReactElement {
    const t: T = useT("translators");
    const me = useQuery(translatorsMeQueryOptions());
    if (me.isError) return <LoadFailed message={t("load.access")} onRetry={() => me.refetch()} />;
    if (!me.data) return <Skeleton className="h-64 w-full rounded-2xl" />;
    if (!me.data.member) return <NoAccess devLogin={me.data.devLogin} />;
    return <>{children}</>;
}

export function TranslatorsPage(props: ISearchProps): React.ReactElement {
    return (
        <SectionLocale>
            <Page {...props} />
        </SectionLocale>
    );
}

function Page({ search, onSearch }: ISearchProps): React.ReactElement {
    const t: T = useT("translators");
    const tab: TranslatorsTab = search.tab ?? "story";
    return (
        <div className="page-shell flex flex-col gap-6">
            <PageHeader title={t("page.title")} />
            <MembersOnly>
                <Tabs value={tab} onValueChange={(v: string) => onSearch({ ...search, tab: v === "story" ? undefined : (v as TranslatorsTab) })}>
                    <TabsList aria-label={t("tabs.label")}>
                        <TabsTab value="story">{t("tabs.story")}</TabsTab>
                        <TabsTab value="search">{t("tabs.search")}</TabsTab>
                        <TabsTab value="sections">{t("tabs.sections")}</TabsTab>
                        <TabsTab value="glossary">{t("tabs.glossary")}</TabsTab>
                        <TabsTab value="edits">{t("tabs.edits")}</TabsTab>
                    </TabsList>
                    <TabsPanel value="story" className="flex flex-col gap-6 pt-4">
                        <StoryTab search={search} onSearch={onSearch} />
                    </TabsPanel>
                    <TabsPanel value="search" className="flex flex-col gap-6 pt-4">
                        <SearchTab search={search} onSearch={onSearch} />
                    </TabsPanel>
                    <TabsPanel value="sections" className="flex flex-col gap-6 pt-4">
                        <SectionsTab search={search} onSearch={onSearch} />
                    </TabsPanel>
                    <TabsPanel value="glossary" className="flex flex-col gap-6 pt-4">
                        <GlossaryTab query={search.gq ?? ""} onQuery={(gq) => onSearch({ ...search, gq: gq || undefined })} />
                    </TabsPanel>
                    <TabsPanel value="edits" className="flex flex-col gap-6 pt-4">
                        <MyEditsTab />
                    </TabsPanel>
                </Tabs>
                {/* A word selected in any table: find it everywhere, in the glossary, or suggest it for the glossary. The other tabs keep their place in the address. */}
                <SelectionMenu onFind={(q) => onSearch({ ...search, tab: "search", q, whole: undefined, section: undefined, status: undefined })} onGlossary={(gq) => onSearch({ ...search, tab: "glossary", gq })} />
            </MembersOnly>
        </div>
    );
}
