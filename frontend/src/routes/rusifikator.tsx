import { createFileRoute, redirect } from "@tanstack/react-router";

/** `/rusifikator` is the translators' section («Русификатор») under its Russian name: the same page, the address kept. */
export const Route = createFileRoute("/rusifikator")({
    beforeLoad: ({ search }) => {
        throw redirect({ to: "/translators", search: search as Record<string, unknown> });
    },
});
