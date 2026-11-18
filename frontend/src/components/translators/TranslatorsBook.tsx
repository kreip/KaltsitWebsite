/**
 * The book view of one story, opened from the translators' section
 * (`/translators/book?group=…&story=…`): the same printable document as the
 * library's `?view=book` (`story/export/BookView`), but "Back" returns to the
 * section with the same chapter and story. Members only, like the section.
 */
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import type React from "react";
import { useMemo, useRef } from "react";
import { asset } from "#/components/operators/detail/impl/assets";
import type { messages as exportMessages } from "#/components/story/export/export.messages";
import { Button } from "#/components/ui/button";
import { LoadFailed } from "#/components/ui/load-failed";
import { Skeleton } from "#/components/ui/skeleton";
import { storyIndexQueryOptions, storyQueryOptions } from "#/lib/api/story";
import { useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { type BookGroup, bookOf } from "#/lib/story/book/book";
import { toHtml } from "#/lib/story/book/html";
import { resolveNickname, useStorySettings } from "#/lib/story/settings";
import type { StoryScript } from "#/types/generated/StoryScript";
import { SectionLocale } from "./SectionLocale";
import { ReadersOnly } from "./TranslatorsPage";
import type { messages } from "./translators.messages";

const SERVER = "en";

function Book({ script, group, back }: { script: StoryScript; group: BookGroup; back: React.ReactElement }): React.ReactElement {
    const t: TypedT<typeof exportMessages> = useT("story");
    const [settings] = useStorySettings();
    const frame = useRef<HTMLIFrameElement>(null);
    const html = useMemo(() => {
        const book = bookOf({ index: { groups: [group] }, scripts: new Map([[script.id, script]]), server: SERVER }, { kind: "story", storyId: script.id }, { nickname: resolveNickname(settings.nickname), images: "cg+bg", branches: "all" });
        return toHtml(book, { typeface: "inter", fonts: { regular: "/fonts/inter-latin-400-normal.woff", bold: "/fonts/inter-latin-700-normal.woff", format: "woff" }, imageSrc: (path) => asset(path) });
    }, [script, group, settings.nickname]);
    return (
        <div className="flex h-dvh flex-col">
            <div className="flex items-center justify-between gap-2 border-border border-b px-4 py-2">
                {back}
                <Button size="sm" className="max-sm:h-11" onClick={() => frame.current?.contentWindow?.print()} data-book-print>
                    {t("export.book.print")}
                </Button>
            </div>
            <iframe ref={frame} title={script.name} srcDoc={html} className="min-h-0 w-full flex-1 bg-white" data-book-frame />
        </div>
    );
}

function Loaded({ groupId, storyId }: { groupId: string; storyId: string }): React.ReactElement {
    const t: TypedT<typeof messages> = useT("translators");
    const index = useQuery(storyIndexQueryOptions(SERVER));
    const script = useQuery(storyQueryOptions(storyId, SERVER));
    const back = (
        <Button variant="ghost" size="sm" className="max-sm:h-11" render={<Link to="/translators" search={{ group: groupId, story: storyId }} />}>
            {t("book.back")}
        </Button>
    );
    if (index.isError || script.isError) {
        return (
            <div className="page-shell flex flex-col gap-4">
                {back}
                <LoadFailed
                    message={t("load.lines")}
                    onRetry={() => {
                        void index.refetch();
                        void script.refetch();
                    }}
                />
            </div>
        );
    }
    const group = index.data?.groups.find((g) => g.id === groupId) ?? index.data?.groups.find((g) => g.stories.some((s) => s.id === storyId));
    if (!index.data || script.isPending || !group || !script.data) return <Skeleton className="h-dvh w-full" />;
    return <Book script={script.data} group={group} back={back} />;
}

function SectionBook({ group, story }: { group?: string; story?: string }): React.ReactElement {
    const t: TypedT<typeof messages> = useT("translators");
    return (
        <ReadersOnly>
            {story ? (
                <Loaded groupId={group ?? ""} storyId={story} />
            ) : (
                <div className="page-shell">
                    <Button variant="ghost" size="sm" render={<Link to="/translators" />}>
                        {t("page.title")}
                    </Button>
                </div>
            )}
        </ReadersOnly>
    );
}

/** The section's book view, in the section's language. */
export function TranslatorsBook(props: { group?: string; story?: string }): React.ReactElement {
    return (
        <SectionLocale>
            <SectionBook {...props} />
        </SectionLocale>
    );
}
