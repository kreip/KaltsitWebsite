/**
 * The history of one line's translation, in a dialog: the pack's changes
 * of the game's Russian, every text of every proposal and the lead's
 * decisions, newest first, each compared by words with the text one step
 * back (TextDiff.tsx).
 */
import { useQuery } from "@tanstack/react-query";
import type React from "react";
import { Badge } from "#/components/ui/badge";
import { Dialog, DialogDescription, DialogHeader, DialogPanel, DialogPopup, DialogTitle } from "#/components/ui/dialog";
import { LoadFailed } from "#/components/ui/load-failed";
import { Skeleton } from "#/components/ui/skeleton";
import { useFormatters } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { lineHistoryQueryOptions } from "#/lib/translators/api";
import type { EditPlace } from "#/types/generated/EditPlace";
import type { HistoryEvent } from "#/types/generated/HistoryEvent";
import { TONE, Toned } from "./LineEdits";
import { StoryText } from "./StoryText";
import { TextDiff } from "./TextDiff";
import type { messages } from "./translators.messages";

type T = TypedT<typeof messages>;

const EVENT_TONE: Record<string, string> = {
    game: TONE.remark,
    proposal: TONE.proposal,
    approve: TONE.approved,
    withdraw: TONE.outdated,
};

/** What happened, in words. `first`: the proposal's first text, not a rewrite. */
function eventLabel(e: HistoryEvent, first: boolean, t: T): string {
    const name = e.by?.name ?? "";
    if (e.kind === "game") return t("lineHistory.game");
    if (e.kind === "proposal") return first ? t("lineHistory.proposed", { name }) : t("lineHistory.rewrote", { name });
    if (e.kind === "withdraw") return t("lineHistory.withdrew", { name });
    return e.source === "game" ? t("lineHistory.approvedGame", { name }) : t("lineHistory.approved", { name });
}

function HistoryList({ place, nickname, t }: { place: EditPlace; nickname: string; t: T }): React.ReactElement {
    const f = useFormatters();
    const query = useQuery(lineHistoryQueryOptions(place));
    if (query.isError) return <LoadFailed layout="inline" message={t("history.load")} onRetry={() => query.refetch()} />;
    if (!query.data) return <Skeleton className="h-24 w-full" />;
    const { events, ruGame } = query.data;
    const labels = { added: t("diff.added"), removed: t("diff.removed") };
    // Newest first: a proposal's first text is its last entry here.
    const firstOf = new Map<number, number>();
    events.forEach((e, i) => {
        if (e.kind === "proposal" && e.proposalId !== null) firstOf.set(e.proposalId, i);
    });
    return (
        <div className="flex flex-col gap-3 text-sm">
            <Toned tone={TONE.remark} label={t("lineHistory.now")}>
                {ruGame ? <StoryText text={ruGame} nickname={nickname} /> : <span className="text-subtle-foreground">{t("ru.missing")}</span>}
            </Toned>
            {events.length === 0 ? (
                <p className="text-subtle-foreground">{t("lineHistory.empty")}</p>
            ) : (
                <ol className="flex flex-col gap-3" aria-label={t("lineHistory.label")}>
                    {events.map((e, i) => (
                        <li key={`${e.kind}-${e.at}-${e.proposalId ?? ""}-${e.text ?? ""}`}>
                            <Toned
                                tone={EVENT_TONE[e.kind] ?? TONE.remark}
                                label={
                                    <>
                                        <span className="text-foreground">{eventLabel(e, e.proposalId !== null && firstOf.get(e.proposalId) === i, t)}</span>
                                        <span>· {f.date(e.at * 1000, { dateStyle: "medium", timeStyle: "short" })}</span>
                                        {e.kind === "proposal" && e.source && e.source !== "own" ? <span>· {t(`proposal.from.${e.source}` as "proposal.from.sova")}</span> : null}
                                        {e.context ? (
                                            <Badge variant="info" size="sm">
                                                {t("proposal.contextBadge")}
                                            </Badge>
                                        ) : null}
                                        {e.deleted ? (
                                            <Badge variant="secondary" size="sm">
                                                {t("lineHistory.deleted")}
                                            </Badge>
                                        ) : null}
                                    </>
                                }
                            >
                                {e.text === null ? null : e.before !== null ? <TextDiff before={e.before} after={e.text} labels={labels} /> : <StoryText text={e.text} nickname={nickname} />}
                            </Toned>
                        </li>
                    ))}
                </ol>
            )}
        </div>
    );
}

export function LineHistoryDialog({ place, open, onOpenChange, nickname, t }: { place: EditPlace; open: boolean; onOpenChange: (open: boolean) => void; nickname: string; t: T }): React.ReactElement {
    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogPopup className="max-w-2xl">
                <DialogHeader>
                    <DialogTitle>{t("lineHistory.title")}</DialogTitle>
                    <DialogDescription>{t("lineHistory.legend")}</DialogDescription>
                </DialogHeader>
                <DialogPanel>{open ? <HistoryList place={place} nickname={nickname} t={t} /> : null}</DialogPanel>
            </DialogPopup>
        </Dialog>
    );
}
