/**
 * The Admin tab (admins only): requests for access (accept with a role or
 * decline), the members with their role and "Remove", and the statistics —
 * proposals made, approved and rejected this week and in all, by person,
 * and each section's progress; Maintenance: the import, the site texts and,
 * with the development sign-in on, the test edits to clean away.
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, Copy, Languages, Play, Trash2, X } from "lucide-react";
import type React from "react";
import { useState } from "react";
import { AlertDialog, AlertDialogClose, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogPopup, AlertDialogTitle } from "#/components/ui/alert-dialog";
import { Badge } from "#/components/ui/badge";
import { Button } from "#/components/ui/button";
import { Checkbox } from "#/components/ui/checkbox";
import { LoadFailed } from "#/components/ui/load-failed";
import { Meter } from "#/components/ui/meter";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "#/components/ui/select";
import { Skeleton } from "#/components/ui/skeleton";
import { Spinner } from "#/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "#/components/ui/table";
import { Tabs, TabsList, TabsPanel, TabsTab } from "#/components/ui/tabs";
import { toastManager } from "#/components/ui/toast";
import { useFormatters, useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import {
    ADMIN_QUERY_ROOT,
    accessRequestsQueryOptions,
    adminStatsQueryOptions,
    decideRequestFn,
    deleteTestEditsFn,
    importStatusQueryOptions,
    membersQueryOptions,
    refreshSiteTextsFn,
    removeMemberFn,
    setMemberRoleFn,
    startImportFn,
    TRANSLATORS_QUERY_ROOT,
    tagWordsQueryOptions,
    testEditsQueryOptions,
} from "#/lib/translators/api";
import type { AccessRequest } from "#/types/generated/AccessRequest";
import type { EditCounts } from "#/types/generated/EditCounts";
import type { ImportStatus } from "#/types/generated/ImportStatus";
import type { MemberRow } from "#/types/generated/MemberRow";
import type { TestEdit } from "#/types/generated/TestEdit";
import type { TranslatorsMe } from "#/types/generated/TranslatorsMe";
import { useStoryNames } from "./storyNames";
import type { messages } from "./translators.messages";

type T = TypedT<typeof messages>;

export const ROLES = ["user", "translator", "admin"] as const;
type Role = (typeof ROLES)[number];

const PROSE = "whitespace-normal py-1.5 align-top leading-snug";

/** A change in the panel: refresh it (and "me", whose role may have changed). */
function useAdminMutation<V>(fn: (v: V) => Promise<unknown>, t: T) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: fn,
        onSuccess: () => Promise.all([queryClient.invalidateQueries({ queryKey: [...ADMIN_QUERY_ROOT] }), queryClient.invalidateQueries({ queryKey: [...TRANSLATORS_QUERY_ROOT, "me"] })]),
        onError: () => toastManager.add({ title: t("admin.failed"), type: "error" }),
    });
}

function RoleSelect({ value, onChange, disabled, label, t }: { value: Role; onChange: (role: Role) => void; disabled?: boolean; label: string; t: T }): React.ReactElement {
    return (
        <Select value={value} onValueChange={(v: string | null) => v && v !== value && onChange(v as Role)} disabled={disabled}>
            <SelectTrigger size="sm" className="w-40" aria-label={label}>
                <SelectValue>{() => t(`role.${value}`)}</SelectValue>
            </SelectTrigger>
            <SelectContent>
                {ROLES.map((r) => (
                    <SelectItem key={r} value={r}>
                        <span className="flex flex-col">
                            <span>{t(`role.${r}`)}</span>
                            <span className="text-subtle-foreground text-xs">{t(`role.${r}Hint`)}</span>
                        </span>
                    </SelectItem>
                ))}
            </SelectContent>
        </Select>
    );
}

function Heading({ children }: { children: React.ReactNode }): React.ReactElement {
    return <h2 className="font-semibold text-[14px] text-foreground">{children}</h2>;
}

function RequestRow({ r, t }: { r: AccessRequest; t: T }): React.ReactElement {
    const f = useFormatters();
    const [role, setRole] = useState<Role>("user");
    const decide = useAdminMutation((accept: boolean) => decideRequestFn({ data: { userId: r.user.id, accept, role } }), t);
    const open = r.status === "open";
    return (
        <TableRow>
            <TableCell className={PROSE}>
                <span className="flex flex-col">
                    <span className="font-medium">{r.user.name}</span>
                    <span className="text-subtle-foreground text-xs">{t("admin.uid", { uid: r.uid })}</span>
                </span>
            </TableCell>
            <TableCell className={PROSE}>{r.note ?? <span className="text-subtle-foreground">—</span>}</TableCell>
            <TableCell className="py-1.5 align-top text-muted-foreground text-xs tabular-nums">{f.date(r.createdAt * 1000, { dateStyle: "medium", timeStyle: "short" })}</TableCell>
            <TableCell className={PROSE}>
                {open ? (
                    <div className="flex flex-wrap items-center gap-2">
                        <RoleSelect value={role} onChange={setRole} label={t("admin.col.role")} t={t} />
                        <Button size="sm" onClick={() => decide.mutate(true)} disabled={decide.isPending}>
                            <Check aria-hidden="true" />
                            {t("admin.accept")}
                        </Button>
                        <Button size="sm" variant="ghost" onClick={() => decide.mutate(false)} disabled={decide.isPending}>
                            <X aria-hidden="true" />
                            {t("admin.decline")}
                        </Button>
                    </div>
                ) : (
                    <Badge variant={r.status === "accepted" ? "success" : "secondary"} size="sm">
                        {r.status === "accepted" ? t("admin.accepted", { name: r.decidedBy?.name ?? "" }) : t("admin.declined", { name: r.decidedBy?.name ?? "" })}
                    </Badge>
                )}
            </TableCell>
        </TableRow>
    );
}

function Requests({ t }: { t: T }): React.ReactElement {
    const query = useQuery(accessRequestsQueryOptions());
    if (query.isError) return <LoadFailed layout="inline" message={t("admin.load")} onRetry={() => query.refetch()} />;
    if (!query.data) return <Skeleton className="h-24 w-full" />;
    const waiting = query.data.filter((r) => r.status === "open").length;
    return (
        <section className="flex flex-col gap-2">
            <Heading>{t("admin.requests", { count: waiting })}</Heading>
            {query.data.length === 0 ? (
                <p className="text-sm text-subtle-foreground">{t("admin.requestsEmpty")}</p>
            ) : (
                <Table aria-label={t("admin.requests", { count: waiting })}>
                    <TableHeader>
                        <TableRow>
                            <TableHead className="w-56">{t("admin.col.who")}</TableHead>
                            <TableHead>{t("admin.col.note")}</TableHead>
                            <TableHead className="w-40">{t("admin.col.date")}</TableHead>
                            <TableHead className="w-96">{t("admin.col.decision")}</TableHead>
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        {query.data.map((r) => (
                            <RequestRow key={r.user.id} r={r} t={t} />
                        ))}
                    </TableBody>
                </Table>
            )}
        </section>
    );
}

function MemberLine({ m, me, t }: { m: MemberRow; me: TranslatorsMe; t: T }): React.ReactElement {
    const f = useFormatters();
    const [asking, setAsking] = useState(false);
    const setRole = useAdminMutation((role: Role) => setMemberRoleFn({ data: { userId: m.user.id, role } }), t);
    const remove = useAdminMutation(() => removeMemberFn({ data: { userId: m.user.id } }), t);
    const self = m.user.id === me.userId;
    return (
        <TableRow>
            <TableCell className={PROSE}>
                <span className="flex flex-col">
                    <span className="font-medium">
                        {m.user.name}
                        {self ? <span className="font-normal text-subtle-foreground"> · {t("admin.you")}</span> : null}
                    </span>
                    <span className="text-subtle-foreground text-xs">{t("admin.uid", { uid: m.uid })}</span>
                </span>
            </TableCell>
            <TableCell className={PROSE}>{self ? <span>{t(`role.${m.role as Role}`)}</span> : <RoleSelect value={m.role as Role} onChange={(r) => setRole.mutate(r)} disabled={setRole.isPending} label={t("admin.col.role")} t={t} />}</TableCell>
            <TableCell className="py-1.5 align-top text-muted-foreground text-xs tabular-nums">{f.date(m.addedAt * 1000, { dateStyle: "medium" })}</TableCell>
            <TableCell className={PROSE}>
                {self ? null : (
                    <Button size="sm" variant="ghost" onClick={() => setAsking(true)}>
                        <Trash2 aria-hidden="true" />
                        {t("admin.remove")}
                    </Button>
                )}
                <AlertDialog open={asking} onOpenChange={setAsking}>
                    <AlertDialogPopup>
                        <AlertDialogHeader>
                            <AlertDialogTitle>{t("admin.removeTitle", { name: m.user.name })}</AlertDialogTitle>
                            <AlertDialogDescription>{t("admin.removeHint")}</AlertDialogDescription>
                        </AlertDialogHeader>
                        <AlertDialogFooter>
                            <AlertDialogClose render={<Button variant="ghost" />}>{t("editor.cancel")}</AlertDialogClose>
                            <Button variant="destructive" onClick={() => remove.mutate(undefined, { onSuccess: () => setAsking(false) })} disabled={remove.isPending}>
                                {t("admin.remove")}
                            </Button>
                        </AlertDialogFooter>
                    </AlertDialogPopup>
                </AlertDialog>
            </TableCell>
        </TableRow>
    );
}

function Members({ me, t }: { me: TranslatorsMe; t: T }): React.ReactElement {
    const query = useQuery(membersQueryOptions());
    if (query.isError) return <LoadFailed layout="inline" message={t("admin.load")} onRetry={() => query.refetch()} />;
    if (!query.data) return <Skeleton className="h-40 w-full" />;
    return (
        <section className="flex flex-col gap-2">
            <Heading>{t("admin.members", { count: query.data.length })}</Heading>
            <Table aria-label={t("admin.members", { count: query.data.length })}>
                <TableHeader>
                    <TableRow>
                        <TableHead>{t("admin.col.who")}</TableHead>
                        <TableHead className="w-48">{t("admin.col.role")}</TableHead>
                        <TableHead className="w-32">{t("admin.col.added")}</TableHead>
                        <TableHead className="w-32" />
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {query.data.map((m) => (
                        <MemberLine key={m.user.id} m={m} me={me} t={t} />
                    ))}
                </TableBody>
            </Table>
        </section>
    );
}

/** "3 / 120": this week and in all. */
function Pair({ week, total }: { week: number; total: number }): React.ReactElement {
    const f = useFormatters();
    return (
        <span className="tabular-nums">
            {f.number(week)} <span className="text-subtle-foreground">/ {f.number(total)}</span>
        </span>
    );
}

function CountsLine({ week, total, t }: { week: EditCounts; total: EditCounts; t: T }): React.ReactElement {
    const items = [
        ["admin.stat.proposed", week.proposed, total.proposed],
        ["admin.stat.approved", week.approved, total.approved],
        ["admin.stat.rejected", week.rejected, total.rejected],
        ["admin.stat.lines", week.linesApproved, total.linesApproved],
    ] as const;
    return (
        <dl className="flex flex-wrap gap-x-8 gap-y-2 text-sm">
            {items.map(([key, w, all]) => (
                <div key={key} className="flex flex-col">
                    <dt className="text-subtle-foreground text-xs">{t(key)}</dt>
                    <dd className="font-medium">
                        <Pair week={w} total={all} />
                    </dd>
                </div>
            ))}
        </dl>
    );
}

function Stats({ t }: { t: T }): React.ReactElement {
    const f = useFormatters();
    const query = useQuery(adminStatsQueryOptions());
    if (query.isError) return <LoadFailed layout="inline" message={t("admin.load")} onRetry={() => query.refetch()} />;
    if (!query.data) return <Skeleton className="h-64 w-full" />;
    const s = query.data;
    return (
        <section className="flex flex-col gap-3">
            <Heading>{t("admin.stats")}</Heading>
            <p className="text-subtle-foreground text-xs">{t("admin.statsHint")}</p>
            <CountsLine week={s.week} total={s.total} t={t} />
            <Table aria-label={t("admin.byPeople")}>
                <TableHeader>
                    <TableRow>
                        <TableHead>{t("admin.col.who")}</TableHead>
                        <TableHead className="w-32">{t("admin.col.role")}</TableHead>
                        <TableHead className="w-28">{t("admin.stat.proposed")}</TableHead>
                        <TableHead className="w-28">{t("admin.stat.approved")}</TableHead>
                        <TableHead className="w-28">{t("admin.stat.rejected")}</TableHead>
                        <TableHead className="w-28">{t("admin.stat.decisions")}</TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {s.people.map((p) => (
                        <TableRow key={p.person.id}>
                            <TableCell className={PROSE}>{p.person.name}</TableCell>
                            <TableCell className={PROSE}>{p.role ? t(`role.${p.role as Role}`) : <span className="text-subtle-foreground">{t("admin.notMember")}</span>}</TableCell>
                            <TableCell className={PROSE}>
                                <Pair week={p.week.proposed} total={p.total.proposed} />
                            </TableCell>
                            <TableCell className={PROSE}>
                                <Pair week={p.week.approved} total={p.total.approved} />
                            </TableCell>
                            <TableCell className={PROSE}>
                                <Pair week={p.week.rejected} total={p.total.rejected} />
                            </TableCell>
                            <TableCell className={`${PROSE} tabular-nums`}>{f.number(p.decisions)}</TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
            <Table aria-label={t("admin.bySections")}>
                <TableHeader>
                    <TableRow>
                        <TableHead className="w-48">{t("admin.col.section")}</TableHead>
                        <TableHead className="w-28">{t("admin.col.total")}</TableHead>
                        <TableHead className="w-28">{t("admin.col.ru")}</TableHead>
                        <TableHead className="w-28">{t("admin.col.waiting")}</TableHead>
                        <TableHead>{t("admin.col.checked")}</TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {s.sections.map((x) => {
                        const share = x.total ? x.approved / x.total : 0;
                        return (
                            <TableRow key={x.section}>
                                <TableCell className={PROSE}>{t(`admin.section.${x.section}` as "admin.section.story")}</TableCell>
                                <TableCell className={`${PROSE} tabular-nums`}>{f.number(x.total)}</TableCell>
                                <TableCell className={`${PROSE} tabular-nums`}>{f.number(x.ru)}</TableCell>
                                <TableCell className={`${PROSE} tabular-nums`}>{f.number(x.waiting)}</TableCell>
                                <TableCell className={PROSE}>
                                    <span className="flex items-center gap-2">
                                        <Meter value={share * 100} aria-label={t("progress.label")} className="w-32" />
                                        <span className="text-subtle-foreground text-xs tabular-nums">
                                            {f.number(x.approved)} · {f.percent(share)}
                                        </span>
                                    </span>
                                </TableCell>
                            </TableRow>
                        );
                    })}
                </TableBody>
            </Table>
        </section>
    );
}

/** The last import's summary in a line: lines, texts, places, when. */
function ImportSummary({ status, t }: { status: ImportStatus; t: T }): React.ReactElement | null {
    const f = useFormatters();
    const last = status.last as { lines?: number; texts?: number; textPlaces?: number; millis?: number } | null;
    if (!last || status.lastFinishedAt === null) return null;
    return <span className="text-muted-foreground text-xs">{t("admin.import.last", { date: f.date(status.lastFinishedAt * 1000, { dateStyle: "medium", timeStyle: "short" }), lines: f.number(last.lines ?? 0), texts: f.number(last.texts ?? 0), seconds: Math.round((last.millis ?? 0) / 1000) })}</span>;
}

/** The import and the site texts: what the deploy used to need scripts for. */
function Service({ t }: { t: T }): React.ReactElement {
    const queryClient = useQueryClient();
    const status = useQuery(importStatusQueryOptions());
    const start = useMutation({
        mutationFn: () => startImportFn(),
        onSuccess: (data) => queryClient.setQueryData(importStatusQueryOptions().queryKey, data),
        onError: () => toastManager.add({ title: t("admin.failed"), type: "error" }),
    });
    const texts = useMutation({
        mutationFn: () => refreshSiteTextsFn(),
        onSuccess: (r) => toastManager.add({ title: t("admin.siteTexts.done", { written: r.written, total: r.total, unknown: r.unknown }), type: r.unknown > 0 ? "warning" : "success" }),
        onError: () => toastManager.add({ title: t("admin.failed"), type: "error" }),
    });
    const s = status.data;
    const running = s?.running ?? false;
    const elapsed = s?.startedAt ? Math.max(0, Math.round(Date.now() / 1000 - s.startedAt)) : 0;
    return (
        <section className="flex flex-col gap-3">
            <Heading>{t("admin.service")}</Heading>
            <div className="flex flex-col gap-2">
                <div className="flex flex-wrap items-center gap-2">
                    <Button size="sm" onClick={() => start.mutate()} disabled={running || start.isPending || !s}>
                        {running ? <Spinner /> : <Play aria-hidden="true" />}
                        {running ? t("admin.import.running") : t("admin.import.run")}
                    </Button>
                    {running ? <span className="text-muted-foreground text-xs">{t("admin.import.phase", { phase: t(`admin.import.phase.${s?.phase ?? "starting"}` as "admin.import.phase.starting"), seconds: elapsed })}</span> : null}
                    {!running && s?.error ? <span className="text-destructive text-xs">{t("admin.import.error", { error: s.error })}</span> : null}
                    {!running && s ? <ImportSummary status={s} t={t} /> : null}
                </div>
                <span className="text-subtle-foreground text-xs">{t("admin.import.hint")}</span>
            </div>
            <div className="flex flex-col gap-2">
                <div className="flex flex-wrap items-center gap-2">
                    <Button size="sm" variant="outline" onClick={() => texts.mutate()} disabled={texts.isPending}>
                        {texts.isPending ? <Spinner /> : <Languages aria-hidden="true" />}
                        {t("admin.siteTexts.refresh")}
                    </Button>
                </div>
                <span className="text-subtle-foreground text-xs">{t("admin.siteTexts.hint")}</span>
            </div>
            <TestEditsBlock t={t} />
        </section>
    );
}

function testEditId(e: TestEdit): string {
    return `${e.kind}:${e.id}`;
}

/**
 * Test edits: every proposal and approval of the development account
 * (`dev-translator`), with tick boxes and "Delete selected" — an approved one
 * is withdrawn first. Only with the development sign-in on (never on the real
 * site), so the local base can be cleaned after a check.
 */
function TestEditsBlock({ t }: { t: T }): React.ReactElement | null {
    const f = useFormatters();
    const nameOf = useStoryNames();
    const query = useQuery(testEditsQueryOptions());
    const [ticked, setTicked] = useState<ReadonlySet<string>>(new Set());
    const [asking, setAsking] = useState(false);
    const remove = useAdminMutation((items: TestEdit[]) => deleteTestEditsFn({ data: { proposals: items.filter((e) => e.kind === "proposal").map((e) => Number(e.id)), approvals: items.filter((e) => e.kind === "approval").map((e) => e.id) } }), t);
    if (query.isError) return <LoadFailed layout="inline" message={t("admin.load")} onRetry={() => query.refetch()} />;
    if (!query.data?.enabled) return null;
    const items = query.data.items;
    const chosen = items.filter((e) => ticked.has(testEditId(e)));
    const toggle = (e: TestEdit, on: boolean) =>
        setTicked((prev) => {
            const next = new Set(prev);
            if (on) next.add(testEditId(e));
            else next.delete(testEditId(e));
            return next;
        });
    return (
        <div className="flex flex-col gap-2">
            <span className="font-medium text-sm">{t("admin.test.title", { count: items.length })}</span>
            <span className="text-subtle-foreground text-xs">{t("admin.test.hint")}</span>
            {items.length === 0 ? null : (
                <>
                    <div className="flex flex-wrap items-center gap-2">
                        <Button size="xs" variant="ghost" onClick={() => setTicked(new Set(items.map(testEditId)))}>
                            {t("replace.all")}
                        </Button>
                        <Button size="xs" variant="ghost" onClick={() => setTicked(new Set())} disabled={ticked.size === 0}>
                            {t("replace.none")}
                        </Button>
                        <Button size="sm" variant="destructive-outline" onClick={() => setAsking(true)} disabled={chosen.length === 0 || remove.isPending}>
                            {remove.isPending ? <Spinner /> : <Trash2 aria-hidden="true" />}
                            {t("admin.test.delete", { count: chosen.length })}
                        </Button>
                    </div>
                    <div className="max-h-96 overflow-y-auto">
                        <Table aria-label={t("admin.test.table")} className="wrap-anywhere text-xs">
                            <TableBody>
                                {items.map((e) => {
                                    const n = e.storyId === "@text" ? null : nameOf(e.storyId);
                                    return (
                                        <TableRow key={testEditId(e)}>
                                            <TableCell className="w-8 py-1.5 align-top">
                                                <Checkbox checked={ticked.has(testEditId(e))} onCheckedChange={(v) => toggle(e, v === true)} aria-label={t("admin.test.pick")} />
                                            </TableCell>
                                            <TableCell className={PROSE}>
                                                <span className="block">{e.text}</span>
                                                <span className="block text-subtle-foreground">{e.en.length > 160 ? `${e.en.slice(0, 160)}…` : e.en}</span>
                                            </TableCell>
                                            <TableCell className={`${PROSE} w-56 text-muted-foreground`}>{n ? `${n.group} › ${n.story}` : t("where.text")}</TableCell>
                                            <TableCell className={`${PROSE} w-40`}>
                                                <Badge variant={e.kind === "approval" || e.approved ? "success" : "info"} size="sm">
                                                    {e.kind === "approval" ? t("admin.test.approval") : e.approved ? t("state.approved") : t("state.proposed")}
                                                </Badge>
                                            </TableCell>
                                            <TableCell className="w-32 py-1.5 align-top text-muted-foreground tabular-nums">{f.date(e.at * 1000, { dateStyle: "medium", timeStyle: "short" })}</TableCell>
                                        </TableRow>
                                    );
                                })}
                            </TableBody>
                        </Table>
                    </div>
                </>
            )}
            <AlertDialog open={asking} onOpenChange={setAsking}>
                <AlertDialogPopup>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t("admin.test.confirmTitle", { count: chosen.length })}</AlertDialogTitle>
                        <AlertDialogDescription>{t("admin.test.confirm")}</AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogClose render={<Button variant="ghost" />}>{t("editor.cancel")}</AlertDialogClose>
                        <Button
                            variant="destructive"
                            onClick={() =>
                                remove.mutate(chosen, {
                                    onSuccess: () => {
                                        setAsking(false);
                                        setTicked(new Set());
                                    },
                                })
                            }
                            disabled={remove.isPending}
                        >
                            {remove.isPending ? <Spinner /> : null}
                            {t("admin.test.delete", { count: chosen.length })}
                        </Button>
                    </AlertDialogFooter>
                </AlertDialogPopup>
            </AlertDialog>
        </div>
    );
}

/**
 * «Теги»: every English word or phrase of the game's colour tags («moderately», «Arts damage»), in how many texts,
 * the Russian stem the pack writes for it where its text keeps the tag, and the plugin's rule (not known until the
 * plugin's rules are read). "Copy the list for Kolooo" puts it on the clipboard as tab-separated text.
 */
function TagWords({ t }: { t: T }): React.ReactElement {
    const f = useFormatters();
    const query = useQuery(tagWordsQueryOptions());
    const copy = async () => {
        const rows = query.data ?? [];
        const text = [[t("admin.tags.col.en"), t("admin.tags.col.lines"), t("admin.tags.col.ru"), t("admin.tags.col.rule")].join("\t"), ...rows.map((w) => [w.en, String(w.lines), w.ru ?? "", w.pluginRule ?? t("admin.tags.unknown")].join("\t"))].join("\n");
        try {
            await navigator.clipboard.writeText(text);
            toastManager.add({ title: t("admin.tags.copied", { count: rows.length }), type: "success" });
        } catch {
            toastManager.add({ title: t("admin.tags.copyFailed"), type: "error" });
        }
    };
    return (
        <section className="flex flex-col gap-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
                <Heading>{t("admin.tags.title")}</Heading>
                <Button variant="outline" size="sm" onClick={copy} disabled={!query.data || query.data.length === 0}>
                    <Copy aria-hidden="true" />
                    {t("admin.tags.copy")}
                </Button>
            </div>
            <p className="text-muted-foreground text-sm">{t("admin.tags.hint")}</p>
            {query.isError ? (
                <LoadFailed layout="inline" message={t("admin.tags.load")} onRetry={() => query.refetch()} />
            ) : !query.data ? (
                <Skeleton className="h-64 w-full" />
            ) : (
                <div className="max-h-[70vh] overflow-y-auto">
                    <Table aria-label={t("admin.tags.title")} className="text-sm">
                        <TableHeader>
                            <TableRow>
                                <TableHead>{t("admin.tags.col.en")}</TableHead>
                                <TableHead className="w-24 text-end">{t("admin.tags.col.lines")}</TableHead>
                                <TableHead>{t("admin.tags.col.ru")}</TableHead>
                                <TableHead className="w-48">{t("admin.tags.col.rule")}</TableHead>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {query.data.map((w) => (
                                <TableRow key={w.en}>
                                    <TableCell className={PROSE}>
                                        {w.en}
                                        <span className="ms-2 text-subtle-foreground text-xs">{w.styles.join(", ")}</span>
                                    </TableCell>
                                    <TableCell className={`${PROSE} text-end tabular-nums`}>{f.number(w.lines)}</TableCell>
                                    <TableCell className={PROSE}>{w.ru ? <span>{w.ru}</span> : <span className="text-subtle-foreground">—</span>}</TableCell>
                                    <TableCell className={`${PROSE} text-muted-foreground`}>{w.pluginRule ?? t("admin.tags.unknown")}</TableCell>
                                </TableRow>
                            ))}
                        </TableBody>
                    </Table>
                </div>
            )}
        </section>
    );
}

export function AdminTab({ me }: { me: TranslatorsMe }): React.ReactElement {
    const t: T = useT("translators");
    const [page, setPage] = useState("main");
    return (
        <Tabs value={page} onValueChange={(v: string) => setPage(v)}>
            <TabsList aria-label={t("admin.pages")}>
                <TabsTab value="main">{t("admin.page.main")}</TabsTab>
                <TabsTab value="tags">{t("admin.page.tags")}</TabsTab>
            </TabsList>
            <TabsPanel value="main" className="flex flex-col gap-8 pt-4">
                <Requests t={t} />
                <Members me={me} t={t} />
                <Service t={t} />
                <Stats t={t} />
            </TabsPanel>
            <TabsPanel value="tags" className="pt-4">
                <TagWords t={t} />
            </TabsPanel>
        </Tabs>
    );
}
