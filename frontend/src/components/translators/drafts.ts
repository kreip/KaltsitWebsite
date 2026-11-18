/**
 * A proposal's draft, kept in this browser while one types (`localStorage`,
 * per place): leaving the page or reloading it loses nothing — the row opens
 * its editor again with the text, marked «Восстановлен черновик · Сбросить».
 * Saving or cancelling the proposal deletes it. Storage may be missing or
 * full (a private window): then there is simply no draft.
 */
import type { ProposalSource } from "#/lib/translators/api";
import type { EditPlace } from "#/types/generated/EditPlace";
import type { GameHere } from "#/types/generated/GameHere";

const PREFIX = "translators.draft.";

export interface IDraft {
    /** What was typed. */
    text: string;
    /** The text the editor started from (the game's, a variant's, my earlier proposal). */
    base: string;
    source: ProposalSource;
    context: boolean;
    here: GameHere | null;
    /** When it was last written (ms). */
    at: number;
}

function keyOf(place: EditPlace): string {
    return `${PREFIX}${place.storyId}|${place.line}|${place.part}|${place.editKey ?? ""}`;
}

export function readDraft(place: EditPlace): IDraft | null {
    try {
        const raw = window.localStorage.getItem(keyOf(place));
        if (!raw) return null;
        const d = JSON.parse(raw) as IDraft;
        return typeof d.text === "string" && typeof d.base === "string" ? d : null;
    } catch {
        return null;
    }
}

export function writeDraft(place: EditPlace, draft: Omit<IDraft, "at">): void {
    try {
        window.localStorage.setItem(keyOf(place), JSON.stringify({ ...draft, at: Date.now() }));
    } catch {
        // No storage (a private window, full): no draft.
    }
}

export function dropDraft(place: EditPlace): void {
    try {
        window.localStorage.removeItem(keyOf(place));
    } catch {
        // Nothing to drop.
    }
}
