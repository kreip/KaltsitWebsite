/**
 * Development only (`/translators/dev-login`): sign in as the local translator
 * account. The backend refuses unless `TRANSLATORS_DEV_LOGIN=1` is set for it,
 * so on a deployed site this page only says it is off.
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useRouter } from "@tanstack/react-router";
import type React from "react";
import { Button } from "#/components/ui/button";
import { PageHeader } from "#/components/ui/page-header";
import { Skeleton } from "#/components/ui/skeleton";
import { useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import { TRANSLATORS_QUERY_ROOT, translatorsDevLoginFn, translatorsMeQueryOptions } from "#/lib/translators/api";
import { SectionLocale } from "./SectionLocale";
import type { messages } from "./translators.messages";

function DevLogin(): React.ReactElement {
    const t: TypedT<typeof messages> = useT("translators");
    const me = useQuery(translatorsMeQueryOptions());
    const queryClient = useQueryClient();
    const router = useRouter();
    const navigate = useNavigate();
    const login = useMutation({
        mutationFn: () => translatorsDevLoginFn(),
        onSuccess: async () => {
            await queryClient.invalidateQueries({ queryKey: TRANSLATORS_QUERY_ROOT });
            // The root route reads the session once; reload it so the header shows the account.
            await router.invalidate();
            await navigate({ to: "/translators" });
        },
    });

    return (
        <div className="page-shell flex flex-col gap-6">
            <PageHeader title={t("dev.title")} />
            {!me.data ? (
                <Skeleton className="h-9 w-48" />
            ) : me.data.devLogin ? (
                <div className="flex flex-col items-start gap-2">
                    <Button onClick={() => login.mutate()} loading={login.isPending}>
                        {t("dev.button")}
                    </Button>
                    {login.isError ? <p className="text-destructive-foreground text-sm">{t("dev.failed")}</p> : null}
                </div>
            ) : (
                <p className="text-muted-foreground text-sm">{t("dev.off")}</p>
            )}
        </div>
    );
}

/** The development sign-in, in the section's language. */
export function DevLoginPage(): React.ReactElement {
    return (
        <SectionLocale>
            <DevLogin />
        </SectionLocale>
    );
}
