/**
 * The translators' section (`/translators`): its backend calls, under
 * `/api/translators`. Every call goes through a server function so the
 * http-only `site_token` reaches the backend: the section's data is only
 * answered to a member (`translators.members`).
 */
import { infiniteQueryOptions, queryOptions } from "@tanstack/react-query";
import { createServerFn } from "@tanstack/react-start";
import { setCookie } from "@tanstack/react-start/server";
import { env } from "#/env";
import { parseError } from "#/lib/api/_shared";
import { optionalSiteToken } from "#/lib/api/_shared.server";
import { backendFetch } from "#/lib/fetch";
import type { ApprovalBatchResult } from "#/types/generated/ApprovalBatchResult";
import type { CharName } from "#/types/generated/CharName";
import type { EditCreated } from "#/types/generated/EditCreated";
import type { EditPlace } from "#/types/generated/EditPlace";
import type { EditRows } from "#/types/generated/EditRows";
import type { ExportResult } from "#/types/generated/ExportResult";
import type { GameObject } from "#/types/generated/GameObject";
import type { Glossary } from "#/types/generated/Glossary";
import type { GlossaryLines } from "#/types/generated/GlossaryLines";
import type { GlossarySuggestion } from "#/types/generated/GlossarySuggestion";
import type { KeyPlaces } from "#/types/generated/KeyPlaces";
import type { LineHistory } from "#/types/generated/LineHistory";
import type { LoginResponse } from "#/types/generated/LoginResponse";
import type { NameRu } from "#/types/generated/NameRu";
import type { ObjectLines } from "#/types/generated/ObjectLines";
import type { ProposalVersion } from "#/types/generated/ProposalVersion";
import type { Release } from "#/types/generated/Release";
import type { SearchLines } from "#/types/generated/SearchLines";
import type { StoryProgress } from "#/types/generated/StoryProgress";
import type { TranslatorStoryLines } from "#/types/generated/TranslatorStoryLines";
import type { TranslatorsMe } from "#/types/generated/TranslatorsMe";

/** Every query of the section starts with this, so one invalidation refreshes them all (sign-in, sign-out). */
export const TRANSLATORS_QUERY_ROOT = ["translators"] as const;

export const getTranslatorsMeFn = createServerFn({ method: "GET" }).handler(async (): Promise<TranslatorsMe> => {
    const res = await backendFetch("/translators/me", { bearerToken: optionalSiteToken() });
    if (!res.ok) throw await parseError(res);
    return (await res.json()) as TranslatorsMe;
});

/** The lead's choice of the "recommended" rule, for the whole team: `A` or `B`. */
export const setRecommendRuleFn = createServerFn({ method: "POST" })
    .inputValidator((d: { rule: "A" | "B" }) => d)
    .handler(async ({ data }): Promise<null> => {
        const res = await backendFetch("/translators/settings/recommend-rule", { method: "PUT", bearerToken: optionalSiteToken(), body: JSON.stringify({ rule: data.rule }) });
        if (!res.ok) throw await parseError(res);
        return null;
    });

export function translatorsMeQueryOptions() {
    return queryOptions({ queryKey: [...TRANSLATORS_QUERY_ROOT, "me"], queryFn: () => getTranslatorsMeFn(), staleTime: 60_000 });
}

export const getTranslatorLinesFn = createServerFn({ method: "GET" })
    .inputValidator((d: { storyId: string }) => d)
    .handler(async ({ data }): Promise<TranslatorStoryLines> => {
        const res = await backendFetch(`/translators/stories/${encodeURIComponent(data.storyId)}/lines`, { bearerToken: optionalSiteToken() });
        if (!res.ok) throw await parseError(res);
        return (await res.json()) as TranslatorStoryLines;
    });

export function translatorLinesQueryOptions(storyId: string) {
    return queryOptions({ queryKey: [...TRANSLATORS_QUERY_ROOT, "lines", storyId], queryFn: () => getTranslatorLinesFn({ data: { storyId } }), staleTime: 5 * 60_000 });
}

export const getGlossaryFn = createServerFn({ method: "GET" }).handler(async (): Promise<Glossary> => {
    const res = await backendFetch("/translators/glossary", { bearerToken: optionalSiteToken() });
    if (!res.ok) throw await parseError(res);
    return (await res.json()) as Glossary;
});

/** The stored glossary; polled every 2 s while a download runs. */
export function glossaryQueryOptions() {
    return queryOptions({
        queryKey: [...TRANSLATORS_QUERY_ROOT, "glossary"],
        queryFn: () => getGlossaryFn(),
        staleTime: 5 * 60_000,
        refetchInterval: (query) => (query.state.data?.refreshing ? 2_000 : false),
    });
}

/** Download the glossary again from the team's sheet (in the background); answers the stored copy with `refreshing` set. */
export const refreshGlossaryFn = createServerFn({ method: "POST" }).handler(async (): Promise<Glossary> => {
    const res = await backendFetch("/translators/glossary/refresh", { method: "POST", bearerToken: optionalSiteToken() });
    if (!res.ok) throw await parseError(res);
    return (await res.json()) as Glossary;
});

// ---- Line editing. Every change refreshes the
// story lines and My edits through `invalidateEdits`.

/** The query keys an edit changes: every story's lines and My edits. */
export const EDIT_QUERY_ROOTS = [
    [...TRANSLATORS_QUERY_ROOT, "lines"],
    [...TRANSLATORS_QUERY_ROOT, "edits"],
] as const;

/** A place in an address: a text outside the story also names its entry. */
function placeQuery(place: EditPlace): URLSearchParams {
    const q = new URLSearchParams({ storyId: place.storyId, line: String(place.line), part: String(place.part) });
    if (place.editKey !== undefined) q.set("editKey", place.editKey);
    return q;
}

async function send(path: string, method: string, body?: unknown): Promise<Response> {
    const res = await backendFetch(path, { method, bearerToken: optionalSiteToken(), body: body === undefined ? undefined : JSON.stringify(body) });
    if (!res.ok) throw await parseError(res);
    return res;
}

export type ProposalSource = "own" | "sova" | "ai";

/** Write my proposal for the line's entry (for every place, or for this place only with `context`), or rewrite the one I have; `confirmMany` once the list of places was confirmed. */
export const createProposalFn = createServerFn({ method: "POST" })
    .inputValidator((d: { place: EditPlace; text: string; source: ProposalSource; context: boolean; confirmMany: boolean }) => d)
    .handler(async ({ data }): Promise<EditCreated> => {
        const res = await send("/translators/proposals", "POST", { ...data.place, text: data.text, source: data.source, context: data.context, confirmMany: data.confirmMany });
        return (await res.json()) as EditCreated;
    });

export const rewriteProposalFn = createServerFn({ method: "POST" })
    .inputValidator((d: { id: number; text: string; confirmMany: boolean }) => d)
    .handler(async ({ data }): Promise<null> => {
        await send(`/translators/proposals/${data.id}`, "PUT", { text: data.text, confirmMany: data.confirmMany });
        return null;
    });

export const deleteProposalFn = createServerFn({ method: "POST" })
    .inputValidator((d: { id: number }) => d)
    .handler(async ({ data }): Promise<null> => {
        await send(`/translators/proposals/${data.id}`, "DELETE");
        return null;
    });

export const getProposalVersionsFn = createServerFn({ method: "GET" })
    .inputValidator((d: { id: number }) => d)
    .handler(async ({ data }): Promise<ProposalVersion[]> => {
        const res = await send(`/translators/proposals/${data.id}/versions`, "GET");
        return (await res.json()) as ProposalVersion[];
    });

export function proposalVersionsQueryOptions(id: number) {
    return queryOptions({ queryKey: [...TRANSLATORS_QUERY_ROOT, "edits", "versions", id], queryFn: () => getProposalVersionsFn({ data: { id } }) });
}

/** The history of the translation of the line at a place, newest first, each text with the one before it. */
export const getLineHistoryFn = createServerFn({ method: "GET" })
    .inputValidator((d: { place: EditPlace }) => d)
    .handler(async ({ data }): Promise<LineHistory> => {
        const res = await send(`/translators/history?${placeQuery(data.place)}`, "GET");
        return (await res.json()) as LineHistory;
    });

/** Under the lines' key, so every edit refreshes an open history. */
export function lineHistoryQueryOptions(place: EditPlace) {
    return queryOptions({ queryKey: [...TRANSLATORS_QUERY_ROOT, "lines", "history", place.storyId, place.line, place.part, place.editKey ?? null], queryFn: () => getLineHistoryFn({ data: { place } }) });
}

/** Agree with a variant of the line's entry or take it back; `proposalId` null = keep as in the game. */
export type AgreeVariant = "game" | "sova" | "ai" | "proposal";

export const setAgreementFn = createServerFn({ method: "POST" })
    .inputValidator((d: { place: EditPlace; variant: AgreeVariant; proposalId: number | null; agree: boolean }) => d)
    .handler(async ({ data }): Promise<null> => {
        await send("/translators/agreements", "PUT", { ...data.place, variant: data.variant, proposalId: data.proposalId, agree: data.agree });
        return null;
    });

export interface IApprovalInput {
    place: EditPlace;
    variant: AgreeVariant;
    proposalId: number | null;
    approve: boolean;
}

function approvalBody(a: IApprovalInput) {
    return { ...a.place, variant: a.variant, proposalId: a.proposalId, approve: a.approve };
}

/** Approve a variant of the line's entry, or withdraw it (the lead). */
export const setApprovalFn = createServerFn({ method: "POST" })
    .inputValidator((d: IApprovalInput) => d)
    .handler(async ({ data }): Promise<null> => {
        await send("/translators/approvals", "PUT", approvalBody(data));
        return null;
    });

/** Approve many lines at once (the lead). */
export const approveBatchFn = createServerFn({ method: "POST" })
    .inputValidator((d: { items: IApprovalInput[] }) => d)
    .handler(async ({ data }): Promise<ApprovalBatchResult> => {
        const res = await send("/translators/approvals/batch", "POST", { items: data.items.map(approvalBody) });
        return (await res.json()) as ApprovalBatchResult;
    });

export type ExportMode = "approved" | "review";
export type ExportFormat = "file" | "patch";

/** The export's report and file list (no contents: each file downloads from `/translators/export`). */
export const exportFn = createServerFn({ method: "GET" })
    .inputValidator((d: { mode: ExportMode; format: ExportFormat }) => d)
    .handler(async ({ data }): Promise<ExportResult> => {
        const res = await send(`/translators/export?mode=${data.mode}&format=${data.format}&only=`, "GET");
        return (await res.json()) as ExportResult;
    });

/** Mark what went out in a pack version (the pack's maintainer). */
export const markReleaseFn = createServerFn({ method: "POST" })
    .inputValidator((d: { version: string; note: string }) => d)
    .handler(async ({ data }): Promise<Release> => {
        const res = await send("/translators/releases", "POST", { version: data.version, note: data.note || null });
        return (await res.json()) as Release;
    });

export const getReleasesFn = createServerFn({ method: "GET" }).handler(async (): Promise<Release[]> => {
    const res = await send("/translators/releases", "GET");
    return (await res.json()) as Release[];
});

/** Every pack version marked, newest first (refreshed with the edits). */
export function releasesQueryOptions() {
    return queryOptions({ queryKey: [...TRANSLATORS_QUERY_ROOT, "edits", "releases"], queryFn: () => getReleasesFn() });
}

/** Mark a story read by me ("I have read this scene"), or not. */
export const setStoryReadFn = createServerFn({ method: "POST" })
    .inputValidator((d: { storyId: string; read: boolean }) => d)
    .handler(async ({ data }): Promise<null> => {
        await send(`/translators/stories/${encodeURIComponent(data.storyId)}/read`, "PUT", { read: data.read });
        return null;
    });

export const getProgressFn = createServerFn({ method: "GET" }).handler(async (): Promise<StoryProgress[]> => {
    const res = await send("/translators/progress", "GET");
    return (await res.json()) as StoryProgress[];
});

/** Every story's progress (refreshed with the edits). */
export function progressQueryOptions() {
    return queryOptions({ queryKey: [...TRANSLATORS_QUERY_ROOT, "edits", "progress"], queryFn: () => getProgressFn(), staleTime: 60_000 });
}

/** A remark (`kind: "remark"`) or a translator's note (`"note"`, maybe about one variant), quoting a fragment of the line. */
export const createCommentFn = createServerFn({ method: "POST" })
    .inputValidator((d: { place: EditPlace; text: string; quote: string | null; kind: "remark" | "note"; variant: string | null; proposalId: number | null }) => d)
    .handler(async ({ data }): Promise<EditCreated> => {
        const res = await send("/translators/comments", "POST", { ...data.place, text: data.text, quote: data.quote, kind: data.kind, variant: data.variant, proposalId: data.proposalId });
        return (await res.json()) as EditCreated;
    });

export const deleteCommentFn = createServerFn({ method: "POST" })
    .inputValidator((d: { id: number }) => d)
    .handler(async ({ data }): Promise<null> => {
        await send(`/translators/comments/${data.id}`, "DELETE");
        return null;
    });

export const resolveCommentFn = createServerFn({ method: "POST" })
    .inputValidator((d: { id: number; resolved: boolean }) => d)
    .handler(async ({ data }): Promise<null> => {
        await send(`/translators/comments/${data.id}/resolved`, "PUT", { resolved: data.resolved });
        return null;
    });

export const getPlacesFn = createServerFn({ method: "GET" })
    .inputValidator((d: EditPlace) => d)
    .handler(async ({ data }): Promise<KeyPlaces> => {
        const res = await send(`/translators/places?${placeQuery(data)}`, "GET");
        return (await res.json()) as KeyPlaces;
    });

/** Every place of the entry at a place. */
export function placesQueryOptions(place: EditPlace) {
    return queryOptions({ queryKey: [...TRANSLATORS_QUERY_ROOT, "places", place.storyId, place.line, place.part, place.editKey ?? null], queryFn: () => getPlacesFn({ data: place }), staleTime: 5 * 60_000 });
}

export const getEditsFn = createServerFn({ method: "GET" })
    .inputValidator((d: { all: boolean }) => d)
    .handler(async ({ data }): Promise<EditRows> => {
        const res = await send(`/translators/edits${data.all ? "?all=true" : ""}`, "GET");
        return (await res.json()) as EditRows;
    });

/** My edits, or everyone's for the lead. */
export function editsQueryOptions(all: boolean) {
    return queryOptions({ queryKey: [...TRANSLATORS_QUERY_ROOT, "edits", all ? "all" : "mine"], queryFn: () => getEditsFn({ data: { all } }) });
}

export const getCharactersFn = createServerFn({ method: "GET" }).handler(async (): Promise<CharName[]> => {
    const res = await send("/translators/characters", "GET");
    return (await res.json()) as CharName[];
});

/** Who each story sprite is, for the Service rows. */
export function charactersQueryOptions() {
    return queryOptions({ queryKey: [...TRANSLATORS_QUERY_ROOT, "characters"], queryFn: () => getCharactersFn(), staleTime: 60 * 60_000 });
}

export const getGlossaryLinesFn = createServerFn({ method: "GET" })
    .inputValidator((d: { id: number; missing: boolean }) => d)
    .handler(async ({ data }): Promise<GlossaryLines> => {
        const res = await send(`/translators/glossary/${data.id}/lines${data.missing ? "?missing=true" : ""}`, "GET");
        return (await res.json()) as GlossaryLines;
    });

/** A glossary term's story lines (only those lacking its form with `missing`). */
export function glossaryLinesQueryOptions(id: number, missing: boolean) {
    return queryOptions({ queryKey: [...TRANSLATORS_QUERY_ROOT, "glossary", "lines", id, missing], queryFn: () => getGlossaryLinesFn({ data: { id, missing } }), staleTime: 5 * 60_000 });
}

export const getNamesFn = createServerFn({ method: "GET" }).handler(async (): Promise<NameRu[]> => {
    const res = await send("/translators/names", "GET");
    return (await res.json()) as NameRu[];
});

/** The library's chapter and story names with the game's Russian. */
export function namesQueryOptions() {
    return queryOptions({ queryKey: [...TRANSLATORS_QUERY_ROOT, "names"], queryFn: () => getNamesFn(), staleTime: 60 * 60_000 });
}

// The same cookies the site's own sign-in sets (`lib/auth/server.ts`), so the
// rest of the site sees the development translator as signed in.
const COOKIE_BASE = { secure: env.NODE_ENV === "production", sameSite: "lax" as const, path: "/", maxAge: 60 * 60 * 24 * 7 };

/**
 * Development only: sign in as the local translator account. The backend
 * answers 404 unless `TRANSLATORS_DEV_LOGIN=1` is set for it.
 */
export const translatorsDevLoginFn = createServerFn({ method: "POST" }).handler(async (): Promise<{ uid: string }> => {
    const res = await backendFetch("/translators/dev-login", { method: "POST" });
    if (!res.ok) throw await parseError(res);
    const { token, uid } = (await res.json()) as LoginResponse;
    setCookie("site_token", token, { ...COOKIE_BASE, httpOnly: true });
    setCookie("auth_indicator", "1", { ...COOKIE_BASE, httpOnly: false });
    return { uid };
});

// ---- The search over every text (the Search tab).

export interface ISearchQuery {
    q: string;
    whole: boolean;
    section: string | null;
    status: string | null;
}

/** How many found texts a page brings. */
export const SEARCH_PAGE = 50;

export const searchLinesFn = createServerFn({ method: "GET" })
    .inputValidator((d: ISearchQuery & { offset: number }) => d)
    .handler(async ({ data }): Promise<SearchLines> => {
        const q = new URLSearchParams({ q: data.q, offset: String(data.offset), limit: String(SEARCH_PAGE) });
        if (data.whole) q.set("whole", "true");
        if (data.section) q.set("section", data.section);
        if (data.status) q.set("status", data.status);
        const res = await send(`/translators/search?${q}`, "GET");
        return (await res.json()) as SearchLines;
    });

/** Under the lines' key, so every edit refreshes the results; 50 at a time. */
export function searchLinesQueryOptions(query: ISearchQuery) {
    return infiniteQueryOptions({
        queryKey: [...TRANSLATORS_QUERY_ROOT, "lines", "search", query],
        queryFn: ({ pageParam }) => searchLinesFn({ data: { ...query, offset: pageParam } }),
        initialPageParam: 0,
        getNextPageParam: (last, pages) => {
            const loaded = pages.reduce((n, p) => n + p.rows.length, 0);
            return last.rows.length > 0 && loaded < last.total ? loaded : undefined;
        },
        staleTime: 60_000,
    });
}

// ---- The game sections: a section's objects and one object's texts.

export const getObjectsFn = createServerFn({ method: "GET" })
    .inputValidator((d: { section: string }) => d)
    .handler(async ({ data }): Promise<GameObject[]> => {
        const res = await send(`/translators/objects?${new URLSearchParams({ section: data.section })}`, "GET");
        return (await res.json()) as GameObject[];
    });

/** A section's objects with their progress (under the edits' key: refreshed with every edit). */
export function objectsQueryOptions(section: string) {
    return queryOptions({ queryKey: [...TRANSLATORS_QUERY_ROOT, "edits", "objects", section], queryFn: () => getObjectsFn({ data: { section } }), staleTime: 60_000 });
}

export const getObjectLinesFn = createServerFn({ method: "GET" })
    .inputValidator((d: { section: string; object: string }) => d)
    .handler(async ({ data }): Promise<ObjectLines> => {
        const res = await send(`/translators/objects/lines?${new URLSearchParams({ section: data.section, object: data.object })}`, "GET");
        return (await res.json()) as ObjectLines;
    });

/** One object's texts (under the lines' key: refreshed with every edit). */
export function objectLinesQueryOptions(section: string, object: string) {
    return queryOptions({ queryKey: [...TRANSLATORS_QUERY_ROOT, "lines", "object", section, object], queryFn: () => getObjectLinesFn({ data: { section, object } }), staleTime: 5 * 60_000 });
}

// ---- "Suggest for the glossary": terms noticed while reading, for the lead.

export const getSuggestionsFn = createServerFn({ method: "GET" }).handler(async (): Promise<GlossarySuggestion[]> => {
    const res = await send("/translators/glossary/suggestions", "GET");
    return (await res.json()) as GlossarySuggestion[];
});

/** The suggested terms, open first (under the glossary's key: a new one refreshes the list). */
export function suggestionsQueryOptions() {
    return queryOptions({ queryKey: [...TRANSLATORS_QUERY_ROOT, "glossary", "suggestions"], queryFn: () => getSuggestionsFn(), staleTime: 60_000 });
}

export interface ISuggestionInput {
    en: string;
    ru: string;
    note: string | null;
    /** The story line it was noticed in, if it was one, and that line's English. */
    storyId: string | null;
    line: number | null;
    part: number | null;
    sourceEn: string | null;
}

export const createSuggestionFn = createServerFn({ method: "POST" })
    .inputValidator((d: ISuggestionInput) => d)
    .handler(async ({ data }): Promise<EditCreated> => {
        const res = await send("/translators/glossary/suggestions", "POST", data);
        return (await res.json()) as EditCreated;
    });

/** The lead's decision: `accepted`, `declined`, or `open` to take it back. */
export const decideSuggestionFn = createServerFn({ method: "POST" })
    .inputValidator((d: { id: number; status: "open" | "accepted" | "declined" }) => d)
    .handler(async ({ data }): Promise<null> => {
        await send(`/translators/glossary/suggestions/${data.id}`, "PUT", { status: data.status });
        return null;
    });

export const deleteSuggestionFn = createServerFn({ method: "POST" })
    .inputValidator((d: { id: number }) => d)
    .handler(async ({ data }): Promise<null> => {
        await send(`/translators/glossary/suggestions/${data.id}`, "DELETE");
        return null;
    });
