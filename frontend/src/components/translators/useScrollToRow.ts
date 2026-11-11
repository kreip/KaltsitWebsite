import { useRouterState } from "@tanstack/react-router";
import { useEffect } from "react";

/** After the rows load, bring the row named by the address's `#n-…` into view (a link from My edits or a place list). */
export function useScrollToRow(ready: boolean): void {
    const hash = useRouterState({ select: (s) => s.location.hash });
    useEffect(() => {
        if (!ready || !hash.startsWith("n-")) return;
        document.getElementById(hash)?.scrollIntoView({ block: "center" });
    }, [ready, hash]);
}
