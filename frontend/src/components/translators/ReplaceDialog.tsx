/**
 * The replace dialog's body: the lines a word stands in, what each would
 * become (in the same case), a tick box at each sure line (ticked at first)
 * and the button that writes the proposals for the ticked ones only. "Check
 * by hand" lines have no box: their case is not sure, nothing is written for
 * them. Shared by "Replace with the glossary's form" (GlossaryTab.tsx) and
 * "Replace the word in other lines" after a proposal (LineEdits.tsx).
 */
import { Link } from "@tanstack/react-router";
import { Replace } from "lucide-react";
import type React from "react";
import { useState } from "react";
import { Button } from "#/components/ui/button";
import { Checkbox } from "#/components/ui/checkbox";
import { DialogDescription, DialogFooter, DialogHeader, DialogPanel, DialogTitle } from "#/components/ui/dialog";
import { LoadFailed } from "#/components/ui/load-failed";
import { Skeleton } from "#/components/ui/skeleton";
import { Spinner } from "#/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "#/components/ui/table";
import type { TypedT } from "#/lib/i18n/messages";
import type { GlossaryFix } from "#/types/generated/GlossaryFix";
import type { GlossaryFixLine } from "#/types/generated/GlossaryFixLine";
import type { LinePick } from "#/types/generated/LinePick";
import { diffLabels } from "./LineEdits";
import { StoryText } from "./StoryText";
import { useStoryNames } from "./storyNames";
import { TextDiff } from "./TextDiff";
import type { messages } from "./translators.messages";
import { plainText, TEXT_PLACE } from "./where";

type T = TypedT<typeof messages>;

const PROSE = "whitespace-normal py-1.5 align-top leading-snug";

function lineId(l: GlossaryFixLine): string {
    return `${l.storyId}\u0000${l.ord}`;
}

function Where({ line, t }: { line: GlossaryFixLine; t: T }): React.ReactElement {
    const nameOf = useStoryNames();
    if (line.storyId === TEXT_PLACE)
        return (
            <Link to="/translators" search={{ tab: "search", q: plainText(line.en) }} className="underline-offset-4 hover:underline">
                {t("where.text")}
            </Link>
        );
    const n = nameOf(line.storyId);
    return n ? (
        <Link to="/translators" search={{ group: n.groupId, story: line.storyId }} hash={`n-${line.ord + 1}`} className="underline-offset-4 hover:underline">
            {t("places.item", { group: n.group, story: n.story, n: line.ord + 1 })}
        </Link>
    ) : (
        <span>{t("places.itemBare", { story: line.storyId, n: line.ord + 1 })}</span>
    );
}

export interface IReplaceDialogProps {
    title: string;
    description: string;
    /** Above the table (a hint about the glossary, say). */
    note?: React.ReactNode;
    data: GlossaryFix | undefined;
    isError: boolean;
    onRetry: () => void;
    pending: boolean;
    /** Write the proposals for the ticked lines. */
    onApply: (picked: LinePick[]) => void;
    nickname: string;
    t: T;
}

export function ReplaceDialogBody({ title, description, note, data, isError, onRetry, pending, onApply, nickname, t }: IReplaceDialogProps): React.ReactElement {
    // The lines unticked by hand: every sure line starts ticked.
    const [off, setOff] = useState<ReadonlySet<string>>(new Set());
    const sure = data?.lines.filter((l) => l.result !== null) ?? [];
    const picked = sure.filter((l) => !off.has(lineId(l)));
    // One proposal per entry: lines of the same English in a story are one.
    const entries = new Set(picked.map((l) => `${l.storyId}:${l.en}`)).size;
    const toggle = (l: GlossaryFixLine, on: boolean) =>
        setOff((prev) => {
            const next = new Set(prev);
            if (on) next.delete(lineId(l));
            else next.add(lineId(l));
            return next;
        });
    return (
        <>
            <DialogHeader>
                <DialogTitle>{title}</DialogTitle>
                <DialogDescription>{description}</DialogDescription>
            </DialogHeader>
            <DialogPanel>
                {isError ? (
                    <LoadFailed layout="inline" message={t("glossary.lines.load")} onRetry={onRetry} />
                ) : !data ? (
                    <Skeleton className="h-40 w-full" />
                ) : data.lines.length === 0 ? (
                    <p className="text-muted-foreground text-sm">{t("glossary.lines.empty")}</p>
                ) : (
                    <div className="flex flex-col gap-2">
                        {note}
                        <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
                            <p className="text-subtle-foreground text-xs">{t("glossary.fix.summary", { sure: sure.length, unsure: data.unsure })}</p>
                            {sure.length > 0 ? (
                                <>
                                    <Button variant="ghost" size="xs" onClick={() => setOff(new Set())} disabled={off.size === 0}>
                                        {t("replace.all")}
                                    </Button>
                                    <Button variant="ghost" size="xs" onClick={() => setOff(new Set(sure.map(lineId)))} disabled={picked.length === 0}>
                                        {t("replace.none")}
                                    </Button>
                                </>
                            ) : null}
                        </div>
                        <div className="max-h-[60vh] overflow-y-auto">
                            <Table aria-label={t("glossary.fix.table")} className="wrap-anywhere text-xs">
                                <TableHeader>
                                    <TableRow>
                                        <TableHead className="w-8">
                                            <span className="sr-only">{t("replace.pick")}</span>
                                        </TableHead>
                                        <TableHead className="w-48">{t("edits.col.where")}</TableHead>
                                        <TableHead>{t("edits.col.was")}</TableHead>
                                        <TableHead>{t("glossary.fix.col.result")}</TableHead>
                                        <TableHead>{t("table.en")}</TableHead>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {data.lines.map((l) => (
                                        <TableRow key={lineId(l)}>
                                            <TableCell className={PROSE}>{l.result !== null ? <Checkbox checked={!off.has(lineId(l))} onCheckedChange={(v) => toggle(l, v === true)} aria-label={t("replace.pick")} /> : null}</TableCell>
                                            <TableCell className={PROSE}>
                                                <Where line={l} t={t} />
                                                {l.places > 1 ? <span className="text-subtle-foreground"> · {t("where.places", { count: l.places })}</span> : null}
                                                {l.approved ? <span className="text-subtle-foreground"> · {t("glossary.fix.approved")}</span> : null}
                                            </TableCell>
                                            <TableCell className={PROSE}>
                                                <StoryText text={l.text} nickname={nickname} />
                                            </TableCell>
                                            <TableCell className={PROSE}>{l.result !== null ? <TextDiff before={l.text} after={l.result} labels={diffLabels(t)} /> : <span className="text-warning-foreground">{t("glossary.fix.byHand")}</span>}</TableCell>
                                            <TableCell className={`${PROSE} text-muted-foreground`}>
                                                <StoryText text={l.en} nickname={nickname} />
                                            </TableCell>
                                        </TableRow>
                                    ))}
                                </TableBody>
                            </Table>
                        </div>
                    </div>
                )}
            </DialogPanel>
            <DialogFooter>
                <Button onClick={() => onApply(picked.map((l) => ({ storyId: l.storyId, ord: l.ord })))} disabled={pending || entries === 0}>
                    {pending ? <Spinner /> : <Replace aria-hidden="true" />}
                    {t("glossary.fix.apply", { count: entries })}
                </Button>
            </DialogFooter>
        </>
    );
}
