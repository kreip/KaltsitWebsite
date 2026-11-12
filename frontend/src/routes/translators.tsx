import { createFileRoute, useNavigate } from "@tanstack/react-router";
import type React from "react";
import { SEARCH_SECTIONS, SEARCH_STATUSES } from "#/components/translators/SearchTab";
import { GAME_SECTIONS } from "#/components/translators/SectionsTab";
import { type ITranslatorsSearch, TranslatorsPage } from "#/components/translators/TranslatorsPage";
import { RouteError } from "#/components/ui/route-error";
import { seo } from "#/lib/seo";
import { translatorsHeadT } from "#/lib/translators/meta";

export const Route = createFileRoute("/translators")({
    component: RouteComponent,
    errorComponent: RouteError,
    validateSearch: (search: Record<string, unknown>): ITranslatorsSearch => {
        const out: ITranslatorsSearch = {};
        const on = (v: unknown) => v === true || v === 1 || v === "1" || v === "true";
        if (search.tab === "search" || search.tab === "sections" || search.tab === "glossary" || search.tab === "edits" || search.tab === "admin") out.tab = search.tab;
        const sec = GAME_SECTIONS.find((s) => s === search.sec);
        if (sec && sec !== GAME_SECTIONS[0]) out.sec = sec;
        // An object's name may read as a number or a flag in the address («1», «true»).
        if (typeof search.obj === "string" && search.obj) out.obj = search.obj;
        else if (typeof search.obj === "number" || typeof search.obj === "boolean") out.obj = String(search.obj);
        if (typeof search.group === "string" && search.group) out.group = search.group;
        if (typeof search.story === "string" && search.story) out.story = search.story;
        if (on(search.cn)) out.cn = true;
        if (on(search.service)) out.service = true;
        if (on(search.diff)) out.diff = true;
        if (on(search.game)) out.game = true;
        if (typeof search.q === "string" && search.q.trim()) out.q = search.q.trim();
        else if (typeof search.q === "number") out.q = String(search.q);
        if (on(search.whole)) out.whole = true;
        if (typeof search.gq === "string" && search.gq.trim()) out.gq = search.gq.trim();
        else if (typeof search.gq === "number") out.gq = String(search.gq);
        const section = SEARCH_SECTIONS.find((s) => s === search.section);
        if (section) out.section = section;
        const status = SEARCH_STATUSES.find((s) => s === search.status);
        if (status) out.status = status;
        if (search.variants === false || search.variants === 0 || search.variants === "0" || search.variants === "false") out.variants = false;
        return out;
    },
    head: ({ match }) => {
        const t = translatorsHeadT(match.context.i18n);
        const { meta, links } = seo({ title: t("meta.title"), description: t("meta.description"), path: "/translators", locale: match.context.i18n?.locale });
        return {
            meta: [{ charSet: "utf-8" }, { name: "viewport", content: "width=device-width, initial-scale=1" }, { name: "robots", content: "noindex" }, ...meta],
            links,
        };
    },
});

function RouteComponent(): React.ReactElement {
    const search = Route.useSearch();
    const navigate = useNavigate({ from: "/translators" });
    return <TranslatorsPage search={search} onSearch={(next) => navigate({ search: next, replace: true })} />;
}
