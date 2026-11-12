/**
 * "Export…" in My edits: what (approved, for the game — the lead only; or
 * every proposal, for checking — REVIEW), the format (one 00_translators.txt
 * or patched copies of 11_ui / 12_story / 13_names), then the report and
 * each file to download (plugin.csv, context.csv and report.txt too). The
 * files come from the section's own `/translators/export` download route.
 */
import { useMutation, useQuery } from "@tanstack/react-query";
import { Download, FileOutput, PackageCheck } from "lucide-react";
import type React from "react";
import { useId, useState } from "react";
import { Alert, AlertDescription } from "#/components/ui/alert";
import { Button } from "#/components/ui/button";
import { Dialog, DialogHeader, DialogPanel, DialogPopup, DialogTitle, DialogTrigger } from "#/components/ui/dialog";
import { Input } from "#/components/ui/input";
import { Label } from "#/components/ui/label";
import { LoadFailed } from "#/components/ui/load-failed";
import { Radio, RadioGroup } from "#/components/ui/radio-group";
import { Spinner } from "#/components/ui/spinner";
import { useFormatters } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { type ExportFormat, type ExportMode, exportFn, markReleaseFn, releasesQueryOptions } from "#/lib/translators/api";
import { useEditMutation } from "./LineEdits";
import type { messages } from "./translators.messages";

type T = TypedT<typeof messages>;

function Choice({ value, label, hint, disabled }: { value: string; label: string; hint?: string; disabled?: boolean }): React.ReactElement {
    return (
        // biome-ignore lint/a11y/noLabelWithoutControl: the kit Radio inside is the control
        <label className="flex items-start gap-2 text-sm">
            <Radio value={value} disabled={disabled} />
            <span className="flex flex-col">
                <span className="font-medium">{label}</span>
                {hint ? <span className="text-muted-foreground text-xs">{hint}</span> : null}
            </span>
        </label>
    );
}

/** "Went out in pack <version>": the pack's maintainer marks it after exporting for the game. */
function MarkRelease({ t }: { t: T }): React.ReactElement {
    const f = useFormatters();
    const ids = { version: useId(), note: useId() };
    const [version, setVersion] = useState("");
    const [note, setNote] = useState("");
    const releases = useQuery(releasesQueryOptions());
    const mark = useEditMutation(() => markReleaseFn({ data: { version: version.trim(), note: note.trim() } }), t("release.failed"));
    const last = releases.data?.[0];
    return (
        <div className="flex flex-col gap-2 border-t pt-3">
            <span className="font-medium text-sm">{t("release.title")}</span>
            <span className="text-muted-foreground text-xs">{t("release.hint")}</span>
            <div className="flex flex-wrap items-end gap-2">
                <div className="flex w-32 flex-col gap-1">
                    <Label htmlFor={ids.version}>{t("release.version")}</Label>
                    <Input id={ids.version} size="sm" value={version} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setVersion(e.target.value)} placeholder="1.0.30" />
                </div>
                <div className="flex min-w-48 flex-1 flex-col gap-1">
                    <Label htmlFor={ids.note}>{t("release.note")}</Label>
                    <Input id={ids.note} size="sm" value={note} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setNote(e.target.value)} />
                </div>
                <Button size="sm" onClick={() => mark.mutate(undefined)} disabled={mark.isPending || version.trim() === ""}>
                    {mark.isPending ? <Spinner /> : <PackageCheck aria-hidden="true" />}
                    {t("release.mark")}
                </Button>
            </div>
            {mark.data ? <span className="text-success-foreground text-xs">{t("release.done", { version: mark.data.version, dictionary: mark.data.dictionary, plugin: mark.data.plugin, context: mark.data.context })}</span> : null}
            {last ? <span className="text-subtle-foreground text-xs">{t("release.last", { version: last.version, date: f.date(last.at * 1000, { dateStyle: "medium" }), by: last.by ?? "" })}</span> : null}
        </div>
    );
}

export function ExportDialog({ forGame, releases, t }: { forGame: boolean; releases: boolean; t: T }): React.ReactElement {
    const f = useFormatters();
    const ids = { mode: useId(), format: useId() };
    const lead = forGame;
    const [mode, setMode] = useState<ExportMode>(lead ? "approved" : "review");
    const [format, setFormat] = useState<ExportFormat>("patch");
    const run = useMutation({ mutationFn: () => exportFn({ data: { mode, format } }) });
    const r = run.data?.report;
    return (
        <Dialog>
            <DialogTrigger render={<Button variant="outline" size="sm" />}>
                <FileOutput aria-hidden="true" />
                {t("export.open")}
            </DialogTrigger>
            <DialogPopup className="max-w-xl">
                <DialogHeader>
                    <DialogTitle>{t("export.title")}</DialogTitle>
                </DialogHeader>
                <DialogPanel className="flex flex-col gap-4">
                    <div className="flex flex-col gap-2">
                        <span id={ids.mode} className="font-medium text-sm">
                            {t("export.mode")}
                        </span>
                        <RadioGroup
                            aria-labelledby={ids.mode}
                            value={mode}
                            onValueChange={(v) => {
                                setMode(v as ExportMode);
                                run.reset();
                            }}
                            className="gap-2"
                        >
                            <Choice value="approved" label={t("export.mode.approved")} hint={lead ? undefined : t("export.mode.approvedHint")} disabled={!lead} />
                            <Choice value="review" label={t("export.mode.review")} />
                        </RadioGroup>
                    </div>
                    <div className="flex flex-col gap-2">
                        <span id={ids.format} className="font-medium text-sm">
                            {t("export.format")}
                        </span>
                        <RadioGroup
                            aria-labelledby={ids.format}
                            value={format}
                            onValueChange={(v) => {
                                setFormat(v as ExportFormat);
                                run.reset();
                            }}
                            className="gap-2"
                        >
                            <Choice value="patch" label={t("export.format.patch")} hint={t("export.format.patchHint")} />
                            <Choice value="file" label={t("export.format.file")} hint={t("export.format.fileHint")} />
                        </RadioGroup>
                    </div>
                    <div>
                        <Button size="sm" onClick={() => run.mutate()} disabled={run.isPending}>
                            {run.isPending ? <Spinner /> : null}
                            {t("export.run")}
                        </Button>
                    </div>
                    {run.isError ? <LoadFailed layout="inline" message={t("export.load")} onRetry={() => run.mutate()} /> : null}
                    {r && run.data && mode === "approved" && releases ? <MarkRelease t={t} /> : null}
                    {r && run.data ? (
                        <div className="flex flex-col gap-2 text-sm">
                            <ul className="flex flex-col gap-0.5">
                                <li>{t("export.written", { count: r.written })}</li>
                                <li>{t("export.story", { count: r.storyRows, context: r.contextRows })}</li>
                                <li>{t("export.plugin", { count: r.pluginRows })}</li>
                                {r.onlyHereRows > 0 ? <li>{t("export.onlyHere", { count: r.onlyHereRows })}</li> : null}
                                <li className={r.skipped.length > 0 ? "text-warning-foreground" : undefined}>{t("export.skipped", { count: r.skipped.length })}</li>
                                {format === "file" ? (
                                    <li className={r.overriddenBy.length > 0 ? "text-warning-foreground" : "text-muted-foreground"}>{r.loadedAfter.length > 0 ? t("export.after", { files: r.loadedAfter.map((x) => `${x} (${r.overriddenBy.find((o) => o.file === x)?.keys ?? 0})`).join(", ") }) : t("export.afterNone")}</li>
                                ) : null}
                            </ul>
                            <Alert variant="info">
                                <AlertDescription>{format === "patch" ? t("export.howToPatch") : t("export.howTo")}</AlertDescription>
                            </Alert>
                            <div>
                                <Button
                                    size="sm"
                                    render={
                                        <a href={`/translators/export?mode=${r.mode}&format=${r.format}&token=${r.archiveToken ?? ""}`} download={r.archive}>
                                            <Download aria-hidden="true" />
                                            {t("export.archive", { name: r.archive })}
                                        </a>
                                    }
                                />
                            </div>
                            <span className="font-medium">{t("export.files")}</span>
                            <ul className="flex flex-col gap-1">
                                {run.data.files.map((file) => (
                                    <li key={file.name} className="flex items-center justify-between gap-2">
                                        <span className="truncate font-mono text-xs">
                                            {file.name} <span className="text-subtle-foreground">· {f.number(Math.ceil(file.size / 1024))} KB</span>
                                        </span>
                                        <Button
                                            variant="ghost"
                                            size="xs"
                                            render={
                                                <a href={`/translators/export?mode=${mode}&format=${format}&file=${encodeURIComponent(file.name)}`} download={file.name}>
                                                    <Download aria-hidden="true" />
                                                    {t("export.download")}
                                                </a>
                                            }
                                        />
                                    </li>
                                ))}
                            </ul>
                        </div>
                    ) : null}
                </DialogPanel>
            </DialogPopup>
        </Dialog>
    );
}
