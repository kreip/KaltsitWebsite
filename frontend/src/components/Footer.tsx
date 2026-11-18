import { Link } from "@tanstack/react-router";
import { SHOW_DONATE } from "#/lib/constants";
import { useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import type { messages } from "./Footer.messages";

/** A footer nav link. On a touch screen it grows to 44px: at `leading-none` it was about 13px tall. */
const FOOTER_LINK = "inline-flex items-center justify-center font-sans text-[12.5px] text-muted-foreground leading-none transition-colors hover:text-foreground pointer-coarse:min-h-11 pointer-coarse:min-w-11";

export default function Footer() {
    const t: TypedT<typeof messages> = useT("common");

    return (
        <footer className="border-border border-t bg-[color-mix(in_srgb,var(--background)_84%,transparent)] py-7">
            <div className="page-gutter flex flex-col gap-2.5 [--page-max:1080px]">
                <div className="flex flex-wrap items-center justify-between gap-3">
                    <div className="inline-flex items-center gap-2.5 font-sans font-semibold text-foreground text-sm leading-none">
                        <img src="/logo/bust_128.png" alt="" width={22} height={22} className="h-5.5 w-5.5 shrink-0 object-contain" />
                        <span>
                            {t("footer.siteName")} <span className="font-normal text-muted-foreground">· v3</span>
                        </span>
                    </div>
                    <nav aria-label={t("footer.siteLinks")} className="inline-flex flex-wrap items-center gap-x-4">
                        <Link to="/changelog" className={FOOTER_LINK}>
                            {t("footer.changelog")}
                        </Link>
                        <Link to="/terms" className={FOOTER_LINK}>
                            {t("footer.terms")}
                        </Link>
                        <Link to="/privacy" className={FOOTER_LINK}>
                            {t("footer.privacy")}
                        </Link>
                        <Link to="/discord" className={FOOTER_LINK}>
                            {t("footer.discord")}
                        </Link>
                        <Link to="/credits" className={FOOTER_LINK}>
                            {t("footer.credits")}
                        </Link>
                        {SHOW_DONATE ? (
                            <Link to="/donate" className={FOOTER_LINK}>
                                {t("footer.donate")}
                            </Link>
                        ) : null}
                    </nav>
                </div>
                <span className="max-w-130 font-sans text-muted-foreground text-xs leading-normal">{t("footer.disclaimer")}</span>
                {/* The font credit is REQUIRED by the fonts' licences: it lives on the credits page
                    (/credits, «Авторы и лицензии»), linked above. */}
                {/* No language picker here: it would be the third copy of one control, after the
                    header trigger and the mobile drawer, and the footer is the least likely
                    place anyone looks for it. */}
                <div className="flex flex-wrap items-center justify-between gap-3">
                    <span className="font-medium font-mono text-[11px] text-muted-foreground leading-none tracking-wide">{t("footer.builtOn")}</span>
                </div>
            </div>
        </footer>
    );
}
