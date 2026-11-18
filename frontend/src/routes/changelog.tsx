import { createFileRoute, redirect } from "@tanstack/react-router";
import { ChangelogError } from "#/components/changelog/ChangelogError";
import { ChangelogPage } from "#/components/changelog/ChangelogPage";
import { changelogQueryOptions } from "#/lib/api/changelog";
import { SHOW_CHANGELOG } from "#/lib/constants";
import { metaT } from "#/lib/meta";
import { defaultOgURL } from "#/lib/og";
import { seo } from "#/lib/seo";

export const Route = createFileRoute("/changelog")({
    // The original project's development history: hidden in this fork (SHOW_CHANGELOG), the address leads home.
    beforeLoad: () => {
        if (!SHOW_CHANGELOG) throw redirect({ to: "/" });
    },
    component: ChangelogPage,
    errorComponent: ChangelogError,
    loader: ({ context }) => context.queryClient.ensureQueryData(changelogQueryOptions()),
    head: ({ match }) => {
        const t = metaT(match.context.i18n);
        const { meta, links } = seo({
            title: t("changelog.title"),
            description: t("changelog.description"),
            path: "/changelog",
            image: defaultOgURL("changelog", match.context.i18n),
            locale: match.context.i18n?.locale,
        });
        return {
            meta: [{ charSet: "utf-8" }, { name: "viewport", content: "width=device-width, initial-scale=1" }, ...meta],
            links,
        };
    },
});
