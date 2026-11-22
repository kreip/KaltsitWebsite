import { useRouterState } from "@tanstack/react-router";
import { useEffect } from "react";

/**
 * After the rows load, bring the row named by the address's `#n-…` into view (a link from My edits or a place list);
 * `row` is its id where the address names it otherwise (a card's `#at-…`).
 */
export function useScrollToRow(ready: boolean, row?: string): void {
    const hash = useRouterState({ select: (s) => s.location.hash });
    const id = row ?? (hash.startsWith("n-") ? hash : null);
    useEffect(() => {
        if (!ready || !id) return;
        // Once drawn: the row opens and unfolds as it is named.
        const frame = requestAnimationFrame(() => document.getElementById(id)?.scrollIntoView({ block: "center" }));
        return () => cancelAnimationFrame(frame);
    }, [ready, id]);
}
