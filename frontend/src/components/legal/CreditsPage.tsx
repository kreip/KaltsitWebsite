/**
 * Credits and licences (/credits): the site's origin (the original project and
 * its source, the creator's GitHub) and the fonts it ships with the notices
 * their licences require — the Sarkaz face is CC BY-NC 4.0 (attribution by
 * name), the two Terra faces are OFL derivatives renamed because OFL reserves
 * "Samigirian", and OpenDyslexic ships unmodified under OFL 1.1, whose
 * copyright notice travels with the font. The footer links here.
 */
import { PageHeader } from "#/components/ui/page-header";
import { REPO_URL } from "#/lib/constants";
import { useRichT, useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import type { messages as footerMessages } from "../Footer.messages";
import type { messages } from "./CreditsPage.messages";

const LINK = "underline underline-offset-2 transition-colors hover:text-foreground";

function ExternalLink({ href, children }: { href: string; children: React.ReactNode }): React.ReactElement {
    return (
        <a href={href} target="_blank" rel="noreferrer" className={LINK}>
            {children}
        </a>
    );
}

export function CreditsPage(): React.ReactElement {
    const t: TypedT<typeof messages> = useT("legal");
    const rt = useRichT("legal");
    const ft: TypedT<typeof footerMessages> = useT("common");
    const rft = useRichT("common");
    return (
        <div className="page-shell flex max-w-3xl flex-col gap-8">
            <PageHeader title={t("credits.heading")} description={t("credits.lead")} />
            <section className="flex flex-col gap-2">
                <h2 className="font-semibold text-foreground text-lg">{t("credits.site.heading")}</h2>
                <p className="text-muted-foreground text-sm leading-relaxed">
                    {rt("credits.site.body", {
                        original: "Myrtle",
                        repo: <ExternalLink href={REPO_URL}>{REPO_URL.replace(/^https:\/\//, "")}</ExternalLink>,
                    })}
                </p>
            </section>
            <section className="flex flex-col gap-2">
                <h2 className="font-semibold text-foreground text-lg">{t("credits.fonts.heading")}</h2>
                <p className="text-muted-foreground text-sm leading-relaxed">
                    {rft("footer.fontCredit", {
                        endfield: <ExternalLink href="https://github.com/lhclbt/Endfield_Font">Endfield_Font</ExternalLink>,
                        ccLicence: <ExternalLink href="https://creativecommons.org/licenses/by-nc/4.0/">CC BY-NC 4.0</ExternalLink>,
                        samigirian: <ExternalLink href="https://github.com/Siphercase/Samigirian">Samigirian</ExternalLink>,
                        oflLicence: <ExternalLink href="/terra-fonts/OFL.txt">OFL 1.1</ExternalLink>,
                        openDyslexic: <ExternalLink href="https://github.com/antijingoist/opendyslexic">OpenDyslexic</ExternalLink>,
                        odLicence: <ExternalLink href="/opendyslexic/OFL.txt">OFL 1.1</ExternalLink>,
                    })}
                </p>
                <p className="text-muted-foreground text-sm">{ft("footer.disclaimer")}</p>
            </section>
        </div>
    );
}
