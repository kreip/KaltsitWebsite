/**
 * A place of the game data in words («Operator › Amiya › Skill 2 ›
 * description», «Interface and other › Medals»). What the game data calls it
 * (the object's id, a table's name) shows on hover only. An object that has
 * a card on the Game sections tab is a link to it; a speaker's name, to
 * its scene.
 */
import { Link } from "@tanstack/react-router";
import type React from "react";
import type { TypedT } from "#/lib/i18n/messages";
import { useStoryNames } from "./storyNames";
import type { messages } from "./translators.messages";
import { gameWhere, isCardSection, objectKey, SPEAKER_FIELD, sectionWord } from "./where";

type T = TypedT<typeof messages>;

export interface IGamePlaceProps {
    section: string;
    objectId: string | null;
    /** The object's English name; a table's name for `other`. */
    objectEn: string | null;
    field: string | null;
    idx: number | null;
    /** What tells the text from the others at the field (a skill level's entry: which level). */
    note?: string;
    t: T;
}

/** A speaker's name in a story scene: «Story › Speakers › chapter › scene», a link to the scene. */
function SpeakerPlace({ storyId, t }: { storyId: string; t: T }): React.ReactElement {
    const nameOf = useStoryNames();
    const n = nameOf(storyId);
    const words = [sectionWord("story", t), t("at.speakers"), n ? `${n.group} › ${n.story}` : storyId].join(" › ");
    if (!n) return <span>{words}</span>;
    return (
        <Link to="/translators" search={{ group: n.groupId, story: storyId }} className="underline-offset-4 hover:underline">
            {words}
        </Link>
    );
}

export function GamePlace({ section, objectId, objectEn, field, idx, note = "", t }: IGamePlaceProps): React.ReactElement {
    if (section === "story" && field === SPEAKER_FIELD && objectId) return <SpeakerPlace storyId={objectId} t={t} />;
    const code = (section === "other" ? objectEn : objectId) ?? undefined;
    const words = gameWhere(section, objectEn, field, idx, t, note);
    const object = objectKey(section, objectId, objectEn);
    if (!object || !isCardSection(section)) return <span title={code}>{words}</span>;
    return (
        <Link to="/translators" search={{ tab: "sections", sec: section === "operators" ? undefined : section, obj: object }} title={code} className="underline-offset-4 hover:underline">
            {words}
        </Link>
    );
}
