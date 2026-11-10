/**
 * The translators' section speaks the site's language (the header's switch).
 * In `ru` its `translators` messages come from the backend catalog (the admin
 * panel's edits), with the Russian shipped with the section
 * (`translators.ru.json`, the same texts `translators/i18n-ru.py` loads into
 * the backend) standing in for any key the catalog lacks.
 */
import type React from "react";
import { useMemo } from "react";
import { I18nProvider, useI18n } from "#/lib/i18n";
import ru from "./translators.ru.json";

/** The section's Russian, keyed as the catalog is (`translators.…`). */
const RU: Record<string, string> = Object.fromEntries(Object.entries(ru as Record<string, string>).map(([k, v]) => [`translators.${k}`, v]));

export function SectionLocale({ children }: { children: React.ReactNode }): React.ReactElement {
    const site = useI18n();
    const messages = useMemo(() => ({ ...RU, ...site.messages }), [site.messages]);
    if (site.locale !== "ru") return <>{children}</>;
    return (
        <I18nProvider locale="ru" available={site.available} messages={messages} gamedataServer={site.gamedataServer} localeGamedataServer={site.localeGamedataServer} gamedataServerPicked={site.gamedataServerPicked} gamedataServers={site.gamedataServers}>
            {children}
        </I18nProvider>
    );
}
