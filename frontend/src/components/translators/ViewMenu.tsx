/**
 * "View": the page's display settings in one menu next to the filters, the
 * same on the Story, Search and Game sections tabs — show the variants,
 * compare, Chinese and service rows (the story only), and "Only as in the
 * game": the game's text (and the English) without proposals, approvals,
 * variants and hints. Everything is kept in the address.
 */
import { Eye } from "lucide-react";
import type React from "react";
import { createContext } from "react";
import { Button } from "#/components/ui/button";
import { Menu, MenuCheckboxItem, MenuPopup, MenuSeparator, MenuTrigger } from "#/components/ui/menu";
import { useT } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import type { ITranslatorsSearch } from "./TranslatorsPage";
import type { messages } from "./translators.messages";

type T = TypedT<typeof messages>;

/** What the tables show, from the address. */
export interface IView {
    variants: boolean;
    diff: boolean;
    cn: boolean;
    service: boolean;
    /** Only the game's text (and the English): no proposals, approvals, variants, hints. */
    gameOnly: boolean;
}

/** "Only as in the game" for every table under it (RuCell reads it). */
export const GameOnly = createContext(false);

export function viewOf(search: ITranslatorsSearch): IView {
    const gameOnly = search.game === true;
    return { variants: !gameOnly && search.variants !== false, diff: !gameOnly && search.diff === true, cn: search.cn === true, service: search.service === true, gameOnly };
}

function Item({ checked, onChange, label, hint, disabled = false }: { checked: boolean; onChange: (checked: boolean) => void; label: string; hint: string; disabled?: boolean }): React.ReactElement {
    return (
        <MenuCheckboxItem checked={checked} onCheckedChange={onChange} disabled={disabled} closeOnClick={false} variant="switch">
            <span className="flex flex-col gap-0.5 py-0.5">
                <span>{label}</span>
                <span className="text-muted-foreground text-xs">{hint}</span>
            </span>
        </MenuCheckboxItem>
    );
}

/** The View menu; `story` adds Chinese and the service rows. */
export function ViewMenu({ search, onSearch, story = false }: { search: ITranslatorsSearch; onSearch: (next: ITranslatorsSearch) => void; story?: boolean }): React.ReactElement {
    const t: T = useT("translators");
    const view = viewOf(search);
    // Settings away from their default, counted on the button.
    const on = [search.variants === false, view.diff, view.cn, view.service].filter(Boolean).length;
    return (
        <Menu>
            <MenuTrigger render={<Button variant="outline" size="sm" />}>
                <Eye aria-hidden="true" />
                {t("view.button")}
                {view.gameOnly ? <span className="text-muted-foreground">· {t("view.gameOnlyShort")}</span> : on > 0 ? <span className="text-muted-foreground">· {on}</span> : null}
            </MenuTrigger>
            <MenuPopup align="start" className="w-80">
                <Item checked={view.gameOnly} onChange={(c) => onSearch({ ...search, game: c || undefined })} label={t("view.gameOnly")} hint={t("view.gameOnlyHint")} />
                <MenuSeparator />
                <Item checked={view.variants} disabled={view.gameOnly} onChange={(c) => onSearch({ ...search, variants: c ? undefined : false })} label={t("toggle.variants")} hint={t("toggle.variantsHint")} />
                <Item checked={view.diff} disabled={view.gameOnly} onChange={(c) => onSearch({ ...search, diff: c || undefined })} label={t("toggle.diff")} hint={t("toggle.diffHint")} />
                {story ? (
                    <>
                        <Item checked={view.cn} onChange={(c) => onSearch({ ...search, cn: c || undefined })} label={t("toggle.cn")} hint={t("toggle.cnHint")} />
                        <Item checked={view.service} onChange={(c) => onSearch({ ...search, service: c || undefined })} label={t("toggle.service")} hint={t("toggle.serviceHint")} />
                    </>
                ) : null}
            </MenuPopup>
        </Menu>
    );
}
