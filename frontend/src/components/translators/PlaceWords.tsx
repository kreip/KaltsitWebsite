/**
 * A place of the game data in words («Operator › Amiya › Skill 2 ›
 * description», «Interface and other › Medals»). What the game data calls it
 * (the object's id, a table's name) shows on hover only. An object that has
 * a card on the Game sections tab is a link to it.
 */
import { Link } from "@tanstack/react-router";
import type React from "react";
import type { TypedT } from "#/lib/i18n/messages";
import type { messages } from "./translators.messages";
import { gameWhere, isCardSection, objectKey } from "./where";

type T = TypedT<typeof messages>;

export interface IGamePlaceProps {
    section: string;
    objectId: string | null;
    /** The object's English name; a table's name for `other`. */
    objectEn: string | null;
    field: string | null;
    idx: number | null;
    t: T;
}

export function GamePlace({ section, objectId, objectEn, field, idx, t }: IGamePlaceProps): React.ReactElement {
    const code = (section === "other" ? objectEn : objectId) ?? undefined;
    const words = gameWhere(section, objectEn, field, idx, t);
    const object = objectKey(section, objectId, objectEn);
    if (!object || !isCardSection(section)) return <span title={code}>{words}</span>;
    return (
        <Link to="/translators" search={{ tab: "sections", sec: section === "operators" ? undefined : section, obj: object }} title={code} className="underline-offset-4 hover:underline">
            {words}
        </Link>
    );
}
