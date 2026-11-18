import { createFileRoute, redirect } from "@tanstack/react-router";
import { SHOW_DONATE } from "#/lib/constants";

export const Route = createFileRoute("/donate")({
    beforeLoad: () => {
        // The original project's ko-fi: hidden in this fork (SHOW_DONATE), the address leads home.
        if (SHOW_DONATE) throw redirect({ href: "https://ko-fi.com/eltik" });
        throw redirect({ to: "/" });
    },
});
