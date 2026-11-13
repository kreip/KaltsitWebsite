/**
 * The Glossary tab: the team's whole glossary as last downloaded from its
 * Google Sheet, with search, a sheet filter, sorting, and columns worked out
 * over the story (the sheet is never changed): how many lines hold the term,
 * the Russian the game uses for it, how consistent the lines are with the
 * glossary, its Chinese, and its lines on demand. Members can download it
 * again. Above the table: the terms members suggested for the glossary
 * (SelectionMenu.tsx), for the lead to accept or decline; the sheet itself is
 * never written from the site.
 *
 * Counting a term spelled the same as another (`Amiya`, `Amiya (Guard)`,
 * `Amiya (Medic)`): the story's English cannot tell them apart, so the lines
 * are counted once, at the entry without a qualifier (else the first one);
 * the others show "—" with a hint naming it.
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Check, ExternalLink, List, RefreshCw, Replace, Trash2, Undo2, X } from "lucide-react";
import type React from "react";
import { Fragment, useEffect, useId, useMemo, useRef, useState } from "react";
import { SearchField } from "#/components/SearchField";
import { Alert, AlertDescription } from "#/components/ui/alert";
import { Badge } from "#/components/ui/badge";
import { Button } from "#/components/ui/button";
import { Collapsible, CollapsiblePanel, CollapsibleTrigger } from "#/components/ui/collapsible";
import { Dialog, DialogDescription, DialogFooter, DialogHeader, DialogPanel, DialogPopup, DialogTitle, DialogTrigger } from "#/components/ui/dialog";
import { Empty, EmptyHeader, EmptyTitle } from "#/components/ui/empty";
import { Label } from "#/components/ui/label";
import { LoadFailed } from "#/components/ui/load-failed";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "#/components/ui/select";
import { Skeleton } from "#/components/ui/skeleton";
import { Spinner } from "#/components/ui/spinner";
import { Switch } from "#/components/ui/switch";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "#/components/ui/table";
import { toastManager } from "#/components/ui/toast";
import { useFormatters, useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { decideSuggestionFn, deleteSuggestionFn, EDIT_QUERY_ROOTS, fixGlossaryVariantFn, glossaryFixQueryOptions, glossaryLinesQueryOptions, glossaryQueryOptions, refreshGlossaryFn, suggestionsQueryOptions, translatorsMeQueryOptions } from "#/lib/translators/api";
import type { GameVariant } from "#/types/generated/GameVariant";
import type { Glossary } from "#/types/generated/Glossary";
import type { GlossarySuggestion } from "#/types/generated/GlossarySuggestion";
import type { GlossaryTerm } from "#/types/generated/GlossaryTerm";
import type { TranslatorsMe } from "#/types/generated/TranslatorsMe";
import { diffLabels, isLead } from "./LineEdits";
import { StoryText } from "./StoryText";
import { useStoryNames } from "./storyNames";
import { TextDiff } from "./TextDiff";
import type { messages } from "./translators.messages";

type T = TypedT<typeof messages>;

type SortKey = "sheet" | "en" | "ru" | "count" | "consistency";
const SORTS: readonly SortKey[] = ["sheet", "en", "ru", "count", "consistency"];
const ALL = "__all__";

const PROSE = "whitespace-normal py-1.5 align-top leading-snug";

/** Search over the English (as written and the key), the Russian and the plural, any case, ё = е. */
function fold(s: string): string {
    return s.toLowerCase().replaceAll("ё", "е");
}

function matches(term: GlossaryTerm, q: string): boolean {
    if (!q) return true;
    return [term.en, term.key, term.ru, term.ruPlural ?? "", term.synonyms ?? ""].some((v) => fold(v).includes(q));
}

/** The sheet's name as shown: the sheet copied from a translator's own glossary by what it is. */
function sheetLabel(term: GlossaryTerm, t: T): string {
    return term.fromSova ? t("glossary.translatorSheet") : term.sheet;
}

/** For a term spelled like others, the entry its lines are counted at (itself when it is that one or alone). */
function countedAt(terms: readonly GlossaryTerm[]): Map<number, GlossaryTerm> {
    const groups = new Map<string, GlossaryTerm[]>();
    for (const term of terms) {
        const k = term.caseSensitive ? term.key : term.key.toLowerCase();
        groups.set(k, [...(groups.get(k) ?? []), term]);
    }
    const out = new Map<number, GlossaryTerm>();
    for (const group of groups.values()) {
        const primary = group.find((g) => !g.qualifier) ?? group[0];
        for (const g of group) out.set(g.id, primary);
    }
    return out;
}

/** The share of a term's lines whose Russian holds its form, or null when it is not checked. */
function consistency(term: GlossaryTerm): number | null {
    return term.fromSova || term.checkedLines === 0 ? null : term.foundLines / term.checkedLines;
}

/** Why the download failed, in words: the backend keeps a code (`offline`, `forbidden`, `changed:<what>`, `other:<message>`). */
function errorWords(error: string, t: T): string {
    if (error === "offline") return t("glossary.error.offline");
    if (error === "forbidden") return t("glossary.error.forbidden");
    if (error.startsWith("changed:")) return t("glossary.error.changed", { what: error.slice("changed:".length) });
    return t("glossary.error.other", { message: error.replace(/^other:/, "") });
}

function Freshness({ data, t }: { data: Glossary; t: T }): React.ReactElement {
    const f = useFormatters();
    const when = (s: number) => f.date(s * 1000, { dateStyle: "medium", timeStyle: "short" });
    return (
        <div className="flex flex-col gap-2">
            <p className="text-muted-foreground text-sm">{data.fetchedAt !== null ? t("glossary.copy", { date: when(data.fetchedAt) }) : t("glossary.none")}</p>
            {data.lastError && data.attemptedAt !== null ? (
                <Alert variant="warning">
                    <AlertDescription>
                        {t("glossary.failed", { date: when(data.attemptedAt) })} {errorWords(data.lastError, t)}
                    </AlertDescription>
                </Alert>
            ) : null}
        </div>
    );
}

function Notes({ term }: { term: GlossaryTerm }): React.ReactElement {
    return (
        <span className="flex flex-col gap-0.5 text-muted-foreground text-xs">
            {[term.note, term.synonyms, term.comment].filter(Boolean).map((n) => (
                <span key={n}>{n}</span>
            ))}
        </span>
    );
}

/** A variant as the game writes it; for a term of several words the whole phrase, the words that are not the glossary's marked. */
function VariantWords({ v }: { v: GameVariant }): React.ReactElement {
    if (v.wrong.length === 0) return <span className={v.glossary ? "font-medium" : undefined}>{v.ru}</span>;
    // The words' positions are what tells them apart (a name may repeat a word), so they key the list.
    const words = v.ru.split(" ").map((w, i) => ({ w, i }));
    return (
        <span>
            {words.map(({ w, i }) => (
                <Fragment key={i}>
                    {i > 0 ? " " : null}
                    {v.wrong.includes(i) ? <mark className="rounded-sm bg-warning/20 text-foreground">{w}</mark> : <span>{w}</span>}
                </Fragment>
            ))}
        </span>
    );
}

/** "Replace with the glossary's form": the lines a variant stands in, what each would get, and the button that writes the proposals. */
function FixDialog({ term, variant, onDone, t }: { term: GlossaryTerm; variant: GameVariant; onDone: () => void; t: T }): React.ReactElement {
    const queryClient = useQueryClient();
    const nameOf = useStoryNames();
    const me = useQuery(translatorsMeQueryOptions());
    const nickname = me.data?.nickname ?? t("nickname.fallback");
    const preview = useQuery(glossaryFixQueryOptions(term.id, variant.stem));
    const apply = useMutation({
        mutationFn: () => fixGlossaryVariantFn({ data: { id: term.id, stem: variant.stem, apply: true } }),
        onSuccess: (data) => {
            toastManager.add({ title: t("glossary.fix.done", { count: data.created }), type: "success" });
            for (const root of EDIT_QUERY_ROOTS) queryClient.invalidateQueries({ queryKey: root });
            queryClient.invalidateQueries({ queryKey: [...glossaryQueryOptions().queryKey] });
            onDone();
        },
        onError: () => toastManager.add({ title: t("glossary.fix.failed"), type: "error" }),
    });
    const sure = preview.data?.lines.filter((l) => l.result !== null) ?? [];
    const entries = new Set(sure.map((l) => `${l.storyId}:${l.en}`)).size;
    const prose = "whitespace-normal py-1.5 align-top leading-snug";
    return (
        <>
            <DialogHeader>
                <DialogTitle>{t("glossary.fix.title", { from: variant.ru, to: term.ru })}</DialogTitle>
                <DialogDescription>{t("glossary.fix.description")}</DialogDescription>
            </DialogHeader>
            <DialogPanel>
                {preview.isError ? (
                    <LoadFailed layout="inline" message={t("glossary.lines.load")} onRetry={() => preview.refetch()} />
                ) : !preview.data ? (
                    <Skeleton className="h-40 w-full" />
                ) : preview.data.lines.length === 0 ? (
                    <p className="text-muted-foreground text-sm">{t("glossary.lines.empty")}</p>
                ) : (
                    <div className="flex flex-col gap-2">
                        <p className="text-subtle-foreground text-xs">{t("glossary.fix.summary", { sure: sure.length, unsure: preview.data.unsure })}</p>
                        <div className="max-h-[60vh] overflow-y-auto">
                            <Table aria-label={t("glossary.fix.table")} className="wrap-anywhere text-xs">
                                <TableHeader>
                                    <TableRow>
                                        <TableHead className="w-48">{t("edits.col.where")}</TableHead>
                                        <TableHead>{t("edits.col.was")}</TableHead>
                                        <TableHead>{t("glossary.fix.col.result")}</TableHead>
                                        <TableHead>{t("table.en")}</TableHead>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {preview.data.lines.map((l) => {
                                        const n = nameOf(l.storyId);
                                        return (
                                            <TableRow key={`${l.storyId}-${l.ord}`}>
                                                <TableCell className={prose}>
                                                    {n ? (
                                                        <Link to="/translators" search={{ group: n.groupId, story: l.storyId }} hash={`n-${l.ord + 1}`} className="underline-offset-4 hover:underline">
                                                            {t("places.item", { group: n.group, story: n.story, n: l.ord + 1 })}
                                                        </Link>
                                                    ) : (
                                                        t("places.itemBare", { story: l.storyId, n: l.ord + 1 })
                                                    )}
                                                    {l.places > 1 ? <span className="text-subtle-foreground"> · {t("where.places", { count: l.places })}</span> : null}
                                                    {l.approved ? <span className="text-subtle-foreground"> · {t("glossary.fix.approved")}</span> : null}
                                                </TableCell>
                                                <TableCell className={prose}>
                                                    <StoryText text={l.text} nickname={nickname} />
                                                </TableCell>
                                                <TableCell className={prose}>{l.result !== null ? <TextDiff before={l.text} after={l.result} labels={diffLabels(t)} /> : <span className="text-warning-foreground">{t("glossary.fix.byHand")}</span>}</TableCell>
                                                <TableCell className={`${prose} text-muted-foreground`}>
                                                    <StoryText text={l.en} nickname={nickname} />
                                                </TableCell>
                                            </TableRow>
                                        );
                                    })}
                                </TableBody>
                            </Table>
                        </div>
                    </div>
                )}
            </DialogPanel>
            <DialogFooter>
                <Button onClick={() => apply.mutate()} disabled={apply.isPending || entries === 0}>
                    {apply.isPending ? <Spinner /> : <Replace aria-hidden="true" />}
                    {t("glossary.fix.apply", { count: entries })}
                </Button>
            </DialogFooter>
        </>
    );
}

function GameVariants({ term, t }: { term: GlossaryTerm; t: T }): React.ReactElement {
    const f = useFormatters();
    const [fixing, setFixing] = useState<GameVariant | null>(null);
    if (term.gameVariants.length === 0) return <span className="text-subtle-foreground">—</span>;
    return (
        <ul className="flex flex-col gap-0.5 text-xs">
            {term.gameVariants.map((v) => (
                <li key={v.ru} className="group/variant flex items-center gap-1 whitespace-nowrap">
                    <VariantWords v={v} />
                    <span className="text-subtle-foreground tabular-nums">{f.number(v.lines)}</span>
                    {v.glossary ? (
                        <span className="inline-flex text-success-foreground" title={t("glossary.variantGlossary")}>
                            <Check className="size-3.5" aria-hidden="true" />
                            <span className="sr-only">{t("glossary.variantGlossary")}</span>
                        </span>
                    ) : (
                        <>
                            <span className="inline-flex text-warning-foreground" title={t("glossary.variantOther")}>
                                <X className="size-3.5" aria-hidden="true" />
                                <span className="sr-only">{t("glossary.variantOther")}</span>
                            </span>
                            {v.stem ? (
                                <Button variant="ghost" size="xs" className="h-5 px-1 text-muted-foreground opacity-0 focus-visible:opacity-100 group-hover/variant:opacity-100" title={t("glossary.fix.button")} aria-label={t("glossary.fix.button")} onClick={() => setFixing(v)}>
                                    <Replace aria-hidden="true" />
                                </Button>
                            ) : null}
                        </>
                    )}
                </li>
            ))}
            <Dialog open={fixing !== null} onOpenChange={(open) => !open && setFixing(null)}>
                <DialogPopup className="max-w-5xl">{fixing ? <FixDialog term={term} variant={fixing} onDone={() => setFixing(null)} t={t} /> : null}</DialogPopup>
            </Dialog>
        </ul>
    );
}

/** A term's story lines in a dialog, optionally only those whose Russian lacks the glossary's form. */
function TermLines({ term, t }: { term: GlossaryTerm; t: T }): React.ReactElement {
    const missingId = useId();
    const [missing, setMissing] = useState(false);
    const query = useQuery(glossaryLinesQueryOptions(term.id, missing));
    const me = useQuery(translatorsMeQueryOptions());
    const nameOf = useStoryNames();
    const nickname = me.data?.nickname ?? t("nickname.fallback");
    return (
        <>
            <DialogHeader>
                <DialogTitle>{t("glossary.lines.title", { term: term.key })}</DialogTitle>
                <DialogDescription>{t("glossary.lines.description", { ru: term.ru })}</DialogDescription>
                {term.fromSova ? null : (
                    <label htmlFor={missingId} className="flex items-center gap-2 text-sm">
                        <Switch id={missingId} checked={missing} onCheckedChange={setMissing} />
                        {t("glossary.lines.missing")}
                    </label>
                )}
            </DialogHeader>
            <DialogPanel>
                {query.isError ? (
                    <LoadFailed layout="inline" message={t("glossary.lines.load")} onRetry={() => query.refetch()} />
                ) : !query.data ? (
                    <Skeleton className="h-40 w-full" />
                ) : query.data.lines.length === 0 ? (
                    <p className="text-muted-foreground text-sm">{t("glossary.lines.empty")}</p>
                ) : (
                    <div className="flex flex-col gap-2">
                        <p className="text-subtle-foreground text-xs">{t("glossary.lines.count", { shown: query.data.lines.length, total: query.data.total })}</p>
                        <ul className="flex flex-col gap-2 text-sm">
                            {query.data.lines.map((l) => {
                                const n = nameOf(l.storyId);
                                return (
                                    <li key={`${l.storyId}-${l.ord}`} className="flex flex-col gap-0.5 border-s-2 ps-2">
                                        <span className="flex items-center gap-1 text-xs">
                                            {n ? (
                                                <Link to="/translators" search={{ group: n.groupId, story: l.storyId }} hash={`n-${l.ord + 1}`} className="underline-offset-4 hover:underline">
                                                    {t("places.item", { group: n.group, story: n.story, n: l.ord + 1 })}
                                                </Link>
                                            ) : (
                                                t("places.itemBare", { story: l.storyId, n: l.ord + 1 })
                                            )}
                                            {l.found === true && l.verified ? <span className="text-subtle-foreground">{t("terms.verified")}</span> : l.found === true ? <Check className="size-3.5 text-success-foreground" aria-label={t("terms.found")} /> : null}
                                            {l.found === false ? (
                                                <span className="inline-flex items-center gap-0.5 text-warning-foreground">
                                                    <X className="size-3.5" aria-hidden="true" />
                                                    {t("terms.notFound")}
                                                </span>
                                            ) : null}
                                        </span>
                                        {l.ruGame ? <StoryText text={l.ruGame} nickname={nickname} /> : <span className="text-subtle-foreground">{t("ru.missing")}</span>}
                                        <span className="text-muted-foreground text-xs">
                                            <StoryText text={l.en} nickname={nickname} />
                                        </span>
                                    </li>
                                );
                            })}
                        </ul>
                    </div>
                )}
            </DialogPanel>
        </>
    );
}

const STATUS_BADGE = { open: "info", accepted: "success", declined: "outline" } as const;

/** One suggested term: what, by whom, where it was noticed, and the lead's decision. */
function Suggestion({ s, me, t }: { s: GlossarySuggestion; me: TranslatorsMe; t: T }): React.ReactElement {
    const f = useFormatters();
    const queryClient = useQueryClient();
    const nameOf = useStoryNames();
    const done = { onSuccess: () => queryClient.invalidateQueries({ queryKey: suggestionsQueryOptions().queryKey }), onError: () => toastManager.add({ title: t("suggest.failed"), type: "error" }) };
    const decide = useMutation({ mutationFn: (status: "open" | "accepted" | "declined") => decideSuggestionFn({ data: { id: s.id, status } }), ...done });
    const remove = useMutation({ mutationFn: () => deleteSuggestionFn({ data: { id: s.id } }), ...done });
    const story = s.storyId ? nameOf(s.storyId) : undefined;
    const status = s.status === "accepted" || s.status === "declined" ? s.status : "open";
    const busy = decide.isPending || remove.isPending;
    return (
        <li className="flex flex-col gap-0.5 border-s-2 ps-2 text-sm">
            <span className="flex flex-wrap items-center gap-x-2">
                <span className="font-medium">{s.en}</span>
                <span className="text-subtle-foreground" aria-hidden="true">
                    →
                </span>
                <span>{s.ru}</span>
                <Badge variant={STATUS_BADGE[status]} size="sm">
                    {t(`suggest.status.${status}` as "suggest.status.open")}
                </Badge>
            </span>
            {s.note ? <span className="text-muted-foreground">{s.note}</span> : null}
            <span className="text-subtle-foreground text-xs">
                {t("suggest.by", { name: s.author.name, date: f.date(s.createdAt * 1000, { dateStyle: "medium" }) })}
                {s.decidedBy ? ` · ${t("suggest.decidedBy", { name: s.decidedBy.name })}` : ""}
            </span>
            {s.sourceEn ? (
                <span className="text-subtle-foreground text-xs">
                    {story && s.storyId && s.ord !== null ? (
                        <Link to="/translators" search={{ group: story.groupId, story: s.storyId }} hash={`n-${s.ord + 1}`} className="underline-offset-4 hover:underline">
                            {t("places.item", { group: story.group, story: story.story, n: s.ord + 1 })}
                        </Link>
                    ) : null}
                    {story && s.ord !== null ? ": " : ""}
                    {s.sourceEn}
                </span>
            ) : null}
            <span className="flex flex-wrap gap-1">
                {isLead(me) && status === "open" ? (
                    <>
                        <Button variant="ghost" size="xs" onClick={() => decide.mutate("accepted")} disabled={busy}>
                            <Check aria-hidden="true" />
                            {t("suggest.accept")}
                        </Button>
                        <Button variant="ghost" size="xs" onClick={() => decide.mutate("declined")} disabled={busy}>
                            <X aria-hidden="true" />
                            {t("suggest.decline")}
                        </Button>
                    </>
                ) : null}
                {isLead(me) && status !== "open" ? (
                    <Button variant="ghost" size="xs" onClick={() => decide.mutate("open")} disabled={busy}>
                        <Undo2 aria-hidden="true" />
                        {t("suggest.reopen")}
                    </Button>
                ) : null}
                {s.author.id === me.userId || isLead(me) ? (
                    <Button variant="ghost" size="xs" onClick={() => remove.mutate()} disabled={busy}>
                        <Trash2 aria-hidden="true" />
                        {t("suggest.delete")}
                    </Button>
                ) : null}
            </span>
        </li>
    );
}

/** The terms suggested for the glossary, folded; open while any waits for the lead. */
function Suggestions({ t }: { t: T }): React.ReactElement | null {
    const query = useQuery(suggestionsQueryOptions());
    const me = useQuery(translatorsMeQueryOptions());
    const list = query.data ?? [];
    const waiting = list.filter((s) => s.status === "open").length;
    // Open at first when something waits for the lead; after that only a click folds or unfolds it.
    const [open, setOpen] = useState<boolean | null>(null);
    const first = useRef<boolean | null>(null);
    if (first.current === null && query.data) first.current = waiting > 0;
    if (!me.data || list.length === 0) return null;
    return (
        <Collapsible open={open ?? first.current ?? false} onOpenChange={setOpen}>
            <CollapsibleTrigger render={<Button variant="outline" size="sm" />}>{t("suggest.list", { count: list.length, waiting })}</CollapsibleTrigger>
            <CollapsiblePanel className="flex flex-col gap-2 pt-3">
                <p className="text-subtle-foreground text-xs">{t("suggest.listHint")}</p>
                <ul className="flex flex-col gap-3">
                    {list.map((s) => (
                        <Suggestion key={s.id} s={s} me={me.data} t={t} />
                    ))}
                </ul>
            </CollapsiblePanel>
        </Collapsible>
    );
}

export interface IGlossaryTabProps {
    /** The text looked for, kept in the address ("Find in the glossary" on a selected word sets it). */
    query: string;
    onQuery: (query: string) => void;
}

export function GlossaryTab({ query: asked, onQuery }: IGlossaryTabProps): React.ReactElement {
    const t: T = useT("translators");
    const f = useFormatters();
    const ids = { sheet: useId(), sort: useId() };
    const queryClient = useQueryClient();
    const query = useQuery(glossaryQueryOptions());
    const refresh = useMutation({
        mutationFn: () => refreshGlossaryFn(),
        onSuccess: (data) => queryClient.setQueryData(glossaryQueryOptions().queryKey, data),
    });
    // The box filters at once; the address follows it, and a query set from outside fills the box.
    const [search, setSearchText] = useState(asked);
    const setSearch = (text: string) => {
        setSearchText(text);
        onQuery(text.trim());
    };
    // biome-ignore lint/correctness/useExhaustiveDependencies: only a change of the address moves the box
    useEffect(() => {
        if (asked !== search.trim()) setSearchText(asked);
    }, [asked]);
    const [sheet, setSheet] = useState<string>(ALL);
    const [sort, setSort] = useState<SortKey>("sheet");

    const terms = query.data?.terms;
    const sheets = useMemo(() => [...new Map((terms ?? []).map((term) => [term.sheet, term])).values()], [terms]);
    const counted = useMemo(() => countedAt(terms ?? []), [terms]);
    const shown = useMemo(() => {
        const q = fold(search.trim());
        const list = (terms ?? []).filter((term) => (sheet === ALL || term.sheet === sheet) && matches(term, q));
        const count = (x: GlossaryTerm) => (counted.get(x.id)?.id === x.id ? x.storyLines : -1);
        if (sort === "en") list.sort((a, b) => f.collator.compare(a.key, b.key));
        else if (sort === "ru") list.sort((a, b) => f.collator.compare(a.ru, b.ru));
        else if (sort === "count") list.sort((a, b) => count(b) - count(a) || f.collator.compare(a.key, b.key));
        else if (sort === "consistency") list.sort((a, b) => (consistency(a) ?? 2) - (consistency(b) ?? 2) || count(b) - count(a));
        return list;
    }, [terms, search, sheet, sort, f.collator, counted]);

    if (query.isError) return <LoadFailed message={t("glossary.load")} onRetry={() => query.refetch()} />;
    if (!query.data) return <Skeleton className="h-96 w-full rounded-2xl" />;
    const data = query.data;
    const busy = data.refreshing || refresh.isPending;

    return (
        <div className="flex flex-col gap-4">
            <div className="flex flex-wrap items-start justify-between gap-3">
                <Freshness data={data} t={t} />
                <div className="flex flex-wrap items-center gap-2">
                    <span className="text-subtle-foreground text-xs">{t("glossary.readOnly")}</span>
                    <Button
                        variant="outline"
                        size="sm"
                        render={
                            <a href={data.sheetUrl} target="_blank" rel="noopener noreferrer">
                                <ExternalLink aria-hidden="true" />
                                {t("glossary.open")}
                            </a>
                        }
                    />
                    <Button size="sm" onClick={() => refresh.mutate()} disabled={busy}>
                        {busy ? <Spinner /> : <RefreshCw aria-hidden="true" />}
                        {busy ? t("glossary.refreshing") : t("glossary.refresh")}
                    </Button>
                </div>
            </div>

            <Suggestions t={t} />

            <div className="flex flex-wrap items-end gap-4">
                <SearchField className="w-full sm:w-80" value={search} onChange={setSearch} placeholder={t("glossary.search")} label={t("glossary.searchLabel")} clearLabel={t("glossary.clear")} />
                <div className="flex w-full flex-col gap-1.5 sm:w-64">
                    <Label htmlFor={ids.sheet}>{t("glossary.sheet")}</Label>
                    <Select value={sheet} onValueChange={(v: string | null) => setSheet(v ?? ALL)}>
                        <SelectTrigger id={ids.sheet}>
                            <SelectValue>{() => (sheet === ALL ? t("glossary.sheetAll") : sheetLabel(sheets.find((s) => s.sheet === sheet) ?? sheets[0], t))}</SelectValue>
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value={ALL}>{t("glossary.sheetAll")}</SelectItem>
                            {sheets.map((s) => (
                                <SelectItem key={s.sheet} value={s.sheet}>
                                    {sheetLabel(s, t)}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                </div>
                <div className="flex w-full flex-col gap-1.5 sm:w-64">
                    <Label htmlFor={ids.sort}>{t("glossary.sort")}</Label>
                    <Select value={sort} onValueChange={(v: string | null) => v && setSort(v as SortKey)}>
                        <SelectTrigger id={ids.sort}>
                            <SelectValue>{() => t(`glossary.sort.${sort}` as "glossary.sort.sheet")}</SelectValue>
                        </SelectTrigger>
                        <SelectContent>
                            {SORTS.map((s) => (
                                <SelectItem key={s} value={s}>
                                    {t(`glossary.sort.${s}` as "glossary.sort.sheet")}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                </div>
            </div>

            <p className="text-subtle-foreground text-xs">{t("glossary.summary", { shown: shown.length, total: data.terms.length })}</p>

            {shown.length === 0 ? (
                <Empty>
                    <EmptyHeader>
                        <EmptyTitle>{data.terms.length === 0 ? t("glossary.none") : t("glossary.empty")}</EmptyTitle>
                    </EmptyHeader>
                </Empty>
            ) : (
                <Table aria-label={t("glossary.tableLabel")}>
                    <TableHeader>
                        <TableRow>
                            <TableHead>{t("glossary.col.en")}</TableHead>
                            <TableHead>{t("glossary.col.ru")}</TableHead>
                            <TableHead>{t("glossary.col.game")}</TableHead>
                            <TableHead className="text-right">{t("glossary.col.consistency")}</TableHead>
                            <TableHead>{t("glossary.col.cn")}</TableHead>
                            <TableHead>{t("glossary.sheet")}</TableHead>
                            <TableHead>{t("glossary.col.notes")}</TableHead>
                            <TableHead className="text-right">{t("glossary.col.count")}</TableHead>
                            <TableHead>
                                <span className="sr-only">{t("glossary.showLines")}</span>
                            </TableHead>
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        {shown.map((term) => {
                            const at = counted.get(term.id);
                            const own = !at || at.id === term.id;
                            const share = consistency(term);
                            const shared = <span title={t("glossary.sharedHint", { key: at?.en ?? term.key })}>—</span>;
                            return (
                                <TableRow key={term.id}>
                                    <TableCell className={PROSE}>
                                        <span className="flex flex-col gap-0.5">
                                            <span className="font-medium">
                                                {term.key}
                                                {term.qualifier ? <span className="text-muted-foreground"> ({term.qualifier})</span> : null}
                                            </span>
                                            {term.en !== term.key && term.en !== `${term.key} (${term.qualifier})` ? <span className="text-subtle-foreground text-xs">{term.en}</span> : null}
                                        </span>
                                    </TableCell>
                                    <TableCell className={PROSE}>
                                        <span className="flex flex-col gap-0.5">
                                            <span>{term.ru}</span>
                                            {term.ruPlural ? <span className="text-muted-foreground text-xs">{t("glossary.plural", { form: term.ruPlural })}</span> : null}
                                        </span>
                                    </TableCell>
                                    <TableCell className={PROSE}>{own ? <GameVariants term={term} t={t} /> : shared}</TableCell>
                                    <TableCell className="py-1.5 text-right align-top tabular-nums">
                                        {!own ? shared : share === null ? <span className="text-subtle-foreground">—</span> : <span title={t("glossary.consistencyHint", { found: term.foundLines, checked: term.checkedLines })}>{f.percent(share)}</span>}
                                    </TableCell>
                                    <TableCell className={`${PROSE} whitespace-nowrap`}>{term.cn ? <span>{term.cn}</span> : term.cnGame ? <span title={t("glossary.cnFromGame")}>{term.cnGame}</span> : <span className="text-subtle-foreground">—</span>}</TableCell>
                                    <TableCell className={PROSE}>
                                        <span className="flex flex-col items-start gap-0.5 text-xs">
                                            {term.fromSova ? (
                                                <Badge variant="tint" size="sm" style={{ "--tone": "var(--dmg-physic)" } as React.CSSProperties}>
                                                    {t("glossary.translatorSheet")}
                                                </Badge>
                                            ) : (
                                                <span className="text-muted-foreground">{term.sheet}</span>
                                            )}
                                        </span>
                                    </TableCell>
                                    <TableCell className={`${PROSE} max-w-64`}>
                                        <Notes term={term} />
                                    </TableCell>
                                    <TableCell className="py-1.5 text-right align-top text-muted-foreground tabular-nums">{own ? f.number(term.storyLines) : shared}</TableCell>
                                    <TableCell className="py-1 align-top">
                                        {own && term.storyLines > 0 ? (
                                            <Dialog>
                                                <DialogTrigger render={<Button variant="ghost" size="xs" />}>
                                                    <List aria-hidden="true" />
                                                    {t("glossary.showLines")}
                                                </DialogTrigger>
                                                <DialogPopup className="max-w-3xl">
                                                    <TermLines term={term} t={t} />
                                                </DialogPopup>
                                            </Dialog>
                                        ) : null}
                                    </TableCell>
                                </TableRow>
                            );
                        })}
                    </TableBody>
                </Table>
            )}
        </div>
    );
}
