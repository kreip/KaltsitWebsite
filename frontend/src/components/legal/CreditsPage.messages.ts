import { defineMessages, type MessageMap } from "#/lib/i18n/messages";

/** The credits page (/credits): who made the site and the fonts it ships, with the notices their licences ask for. */
export const namespace = "legal";

export const messages = {
    "credits.heading": {
        text: "Credits and licences",
        description: "Heading of the credits page.",
    },
    "credits.lead": {
        text: "Who made this site and the fonts it uses, and under which licences.",
        description: "Line under the heading of the credits page.",
    },
    "credits.site.heading": {
        text: "The site",
        description: "Section heading on the credits page: the site's origin.",
    },
    "credits.site.body": {
        text: "Muelsyse is a fork of {original} by Eltik ({repo}). The Russian translation and the «Русификатор» section are by the ArknightsRU team.",
        description: "Credits page: the site's origin. `{original}` is the original site's name (Myrtle), `{repo}` a link to its source code; Eltik and ArknightsRU are names and stay as-is.",
    },
    "credits.fonts.heading": {
        text: "Fonts",
        description: "Section heading on the credits page: the fonts and their licences.",
    },
} satisfies MessageMap;

export const { keys } = defineMessages({ namespace, messages });
