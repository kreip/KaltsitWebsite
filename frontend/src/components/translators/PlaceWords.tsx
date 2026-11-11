/**
 * A place of the game data in words («Operator › Amiya › Skill 2 ›
 * description», «Interface and other › Medals»). What the game data calls it
 * (the object's id, a table's name) shows on hover only.
 */
import type React from "react";
import type { TypedT } from "#/lib/i18n/messages";
import type { messages } from "./translators.messages";
import { gameWhere } from "./where";

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
    return <span title={code}>{gameWhere(section, objectEn, field, idx, t)}</span>;
}
