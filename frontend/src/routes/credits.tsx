import { createFileRoute } from "@tanstack/react-router";
import { CreditsPage } from "#/components/legal/CreditsPage";
import { metaT } from "#/lib/meta";
import { defaultOgURL } from "#/lib/og";
import { seo } from "#/lib/seo";

export const Route = createFileRoute("/credits")({
    component: CreditsPage,
    head: ({ match }) => {
        const t = metaT(match.context.i18n);
        const { meta, links } = seo({
            title: t("credits.title"),
            description: t("credits.description"),
            path: "/credits",
            image: defaultOgURL("terms", match.context.i18n),
            locale: match.context.i18n?.locale,
        });
        return {
            meta: [{ charSet: "utf-8" }, { name: "viewport", content: "width=device-width, initial-scale=1" }, ...meta],
            links,
        };
    },
});
