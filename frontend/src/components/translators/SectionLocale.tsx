/**
 * The translators' section speaks the site's language (the header's switch).
 * In `ru` its `translators` messages are the Russian shipped with the section
 * (`translators.ru.json`); the backend catalog's value wins only where a
 * person translated the key in the admin panel — a catalog value equal to
 * the English source is the fallback, not a translation.
 */
import type React from "react";
import { useMemo } from "react";
import { I18nProvider, useI18n } from "#/lib/i18n";
import { messages as source } from "./translators.messages";
import ru from "./translators.ru.json";

/** The section's Russian, keyed as the catalog is (`translators.…`). */
const RU: Record<string, string> = Object.fromEntries(Object.entries(ru as Record<string, string>).map(([k, v]) => [`translators.${k}`, v]));
/** The English of each key, to tell the catalog's fallback from a translation. */
const EN: Record<string, string> = Object.fromEntries(Object.entries(source).map(([k, m]) => [`translators.${k}`, m.text]));

export function SectionLocale({ children }: { children: React.ReactNode }): React.ReactElement {
    const site = useI18n();
    const messages = useMemo(() => {
        const out: Record<string, string> = { ...site.messages, ...RU };
        for (const [k, v] of Object.entries(site.messages)) if (k in RU && v !== EN[k]) out[k] = v;
        return out;
    }, [site.messages]);
    if (site.locale !== "ru") return <>{children}</>;
    return (
        <I18nProvider locale="ru" available={site.available} messages={messages} gamedataServer={site.gamedataServer} localeGamedataServer={site.localeGamedataServer} gamedataServerPicked={site.gamedataServerPicked} gamedataServers={site.gamedataServers}>
            {children}
        </I18nProvider>
    );
}
