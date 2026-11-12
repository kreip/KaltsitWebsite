/**
 * The overview above the Search and Game sections tables: how many texts
 * the section or the filter holds, how many the game shows in Russian, how
 * many are checked — and "Next unchecked line", which scrolls to the first
 * row below the screen's top that is not approved yet.
 */
import { ArrowDownToLine } from "lucide-react";
import type React from "react";
import { Button } from "#/components/ui/button";
import { Meter } from "#/components/ui/meter";
import { toastManager } from "#/components/ui/toast";
import { useFormatters } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import type { messages } from "./translators.messages";

type T = TypedT<typeof messages>;

/** "Texts: 1,240 · with Russian: 1,100 · checked: 300 (24 %)" with a meter. */
export function TextCounts({ total, ru, approved, t }: { total: number; ru: number; approved: number; t: T }): React.ReactElement {
    const f = useFormatters();
    const share = total ? approved / total : 0;
    return (
        <span className="flex flex-wrap items-center gap-x-3 gap-y-1">
            <Meter value={share * 100} aria-label={t("progress.label")} className="w-32" />
            <span className="text-subtle-foreground text-xs">{t("overview.counts", { total: f.number(total), ru: f.number(ru), approved: f.number(approved), share: f.percent(share) })}</span>
        </span>
    );
}

/** Scroll to the first row under the screen's top whose line is not approved (rows carry `data-status`). */
export function scrollToNextUnchecked(): boolean {
    const header = Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--site-header-height")) || 0;
    const rows = [...document.querySelectorAll<HTMLElement>("tr[data-status]")];
    const next = rows.find((r) => r.dataset.status !== "approved" && r.getBoundingClientRect().top > header + 8);
    if (!next) return false;
    next.scrollIntoView({ block: "start", behavior: "smooth" });
    next.classList.add("bg-primary/10");
    window.setTimeout(() => next.classList.remove("bg-primary/10"), 1500);
    return true;
}

export function NextUnchecked({ t }: { t: T }): React.ReactElement {
    return (
        <Button variant="outline" size="sm" onClick={() => scrollToNextUnchecked() || toastManager.add({ title: t("overview.noneBelow"), type: "info" })}>
            <ArrowDownToLine aria-hidden="true" />
            {t("overview.nextUnchecked")}
        </Button>
    );
}
