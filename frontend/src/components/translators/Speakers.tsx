/**
 * Who says a line, for «check the gender»: an operator's name with the
 * gender the game's files state (♀ / ♂), and — for a line operators of both
 * genders say with a gendered Russian — all of them, each a link to their
 * line (the archives' voice line, else the story's) in a new tab.
 */
import { Link } from "@tanstack/react-router";
import { Mars, Venus } from "lucide-react";
import type React from "react";
import type { TypedT } from "#/lib/i18n/messages";
import type { LineSpeaker } from "#/types/generated/LineSpeaker";
import type { TranslatorLine } from "#/types/generated/TranslatorLine";
import { Hint } from "./Hint";
import { GamePlace } from "./PlaceWords";
import { useStoryNames } from "./storyNames";
import type { messages } from "./translators.messages";

type T = TypedT<typeof messages>;

/** ♀ or ♂, the word on hover; nothing for a gender not known. */
export function GenderSign({ gender, t }: { gender: string | null; t: T }): React.ReactElement | null {
    if (gender !== "f" && gender !== "m") return null;
    const words = t("gender.sign", { gender });
    const Sign = gender === "f" ? Venus : Mars;
    return (
        <Hint text={words}>
            <span className="ms-1 inline-flex align-middle text-muted-foreground">
                <Sign className="size-3.5" aria-hidden="true" />
                <span className="sr-only">{words}</span>
            </span>
        </Hint>
    );
}

/** The speaker cell's name: the Russian, the English under it, the gender's sign after. */
export function SpeakerName({ en, ru, gender, t }: { en: string; ru: string | null; gender: string | null; t: T }): React.ReactElement {
    const own = ru && ru !== en ? ru : null;
    return (
        <span className="flex flex-col">
            <span className="font-medium">
                {own ?? en}
                <GenderSign gender={gender} t={t} />
            </span>
            {own ? <span className="text-muted-foreground text-xs">{en}</span> : null}
        </span>
    );
}

function SpeakerAt({ who, editKey, t }: { who: LineSpeaker; editKey: string; t: T }): React.ReactElement | null {
    const nameOf = useStoryNames();
    if (who.charId && who.voice !== null) return <GamePlace section="archives" objectId={who.charId} objectEn={who.nameEn} field="voice.text" idx={who.voice} editKey={editKey} t={t} />;
    if (!who.storyId || who.ord === null) return null;
    const n = nameOf(who.storyId);
    const words = t("where.item", { group: n?.group ?? "", story: n?.story ?? who.storyId, n: who.ord + 1 });
    if (!n) return <span>{words}</span>;
    return (
        <Link to="/translators" search={{ group: n.groupId, story: who.storyId }} hash={`n-${who.ord + 1}`} className="underline-offset-4 hover:underline" target="_blank" rel="noreferrer">
            {words}
        </Link>
    );
}

/** Every operator who says the line, with the gender and the way to each. */
export function MixedSpeakers({ line, t }: { line: TranslatorLine; t: T }): React.ReactElement | null {
    if (line.mixedSpeakers.length === 0) return null;
    return (
        <div className="flex flex-col gap-0.5 text-xs">
            <span className="text-subtle-foreground">{t("mixed.title", { count: line.mixedSpeakers.length })}</span>
            <ul className="flex max-h-72 flex-col gap-0.5 overflow-y-auto" aria-label={t("mixed.list")}>
                {line.mixedSpeakers.map((who) => (
                    <li key={who.charId ?? who.nameEn} className="flex flex-wrap items-baseline gap-x-1.5">
                        <span className="font-medium">
                            {who.nameRu ?? who.nameEn}
                            <GenderSign gender={who.gender} t={t} />
                        </span>
                        <span className="text-muted-foreground">
                            <SpeakerAt who={who} editKey={line.editKey} t={t} />
                        </span>
                    </li>
                ))}
            </ul>
        </div>
    );
}
