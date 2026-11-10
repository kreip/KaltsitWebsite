import { createFileRoute, useNavigate } from "@tanstack/react-router";
import type React from "react";
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
        if (search.tab === "glossary" || search.tab === "edits") out.tab = search.tab;
        if (typeof search.group === "string" && search.group) out.group = search.group;
        if (typeof search.story === "string" && search.story) out.story = search.story;
        if (on(search.cn)) out.cn = true;
        if (on(search.service)) out.service = true;
        if (on(search.diff)) out.diff = true;
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
