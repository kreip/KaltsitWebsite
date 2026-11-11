import { describe, expect, it } from "vitest";
import { formatMessage } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import type { messages } from "./translators.messages";
import ru from "./translators.ru.json";
import { fieldWords, gameWhere, plainText } from "./where";

const t = ((key: string, args?: Parameters<typeof formatMessage>[2]) => formatMessage((ru as Record<string, string>)[key], "ru", args)) as unknown as TypedT<typeof messages>;

describe("where a text stands, in words", () => {
    it("names the part, its number and the field", () => {
        expect(fieldWords("skill.description", 2, t)).toBe("Навык 2 › описание");
        expect(fieldWords("potential", 3, t)).toBe("потенциал 3");
        expect(fieldWords("name", 0, t)).toBe("название");
        expect(fieldWords("odd.thing", 0, t)).toBe("odd › thing");
    });

    it("puts the section and the object first; a table of the rest says no field", () => {
        expect(gameWhere("operators", "Amiya", "skill.name", 1, t)).toBe("Оператор › Amiya › Навык 1 › название");
        expect(gameWhere("other", "campaign_table", "text", 0, t)).toBe("Интерфейс и прочее › campaign_table");
    });

    it("reads the English without tags", () => {
        expect(plainText("<i>Ursus</i> Slug ")).toBe("Ursus Slug");
    });
});
