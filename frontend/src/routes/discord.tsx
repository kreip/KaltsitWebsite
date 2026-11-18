import { createFileRoute, redirect } from "@tanstack/react-router";
import { CONTACT_URL } from "#/lib/constants";

export const Route = createFileRoute("/discord")({
    beforeLoad: () => {
        // The original project's Discord is its author's community: this fork's is the ArknightsRU channel.
        throw redirect({ href: CONTACT_URL });
    },
});
