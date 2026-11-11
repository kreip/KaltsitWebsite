import { describe, expect, it } from "vitest";
import { formatMessage } from "#/lib/i18n";
import type { TypedT } from "#/lib/i18n/messages";
import type { messages } from "./translators.messages";
import ru from "./translators.ru.json";
import { fieldWords, fileWord, gameWhere, plainText, tableWord } from "./where";

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
        expect(gameWhere("other", "campaign_table", "text", 0, t)).toBe("Интерфейс и прочее › Аннигиляция");
    });

    it("names a table and a dictionary in words, and leaves an unknown one as it is", () => {
        expect(tableWord("clue_data", t)).toBe("Улики");
        expect(tableWord("character_table", t)).toBe("Операторы (прочее)");
        expect(tableWord("brand_new_table", t)).toBe("brand_new_table");
        expect(fileWord("13_names.txt", t)).toBe("Словарь имён");
        expect(fileWord("plugin:skill", t)).toBe("Каталог плагина");
        expect(fileWord("99_other.txt", t)).toBe("99_other.txt");
    });

    it("reads the English without tags", () => {
        expect(plainText("<i>Ursus</i> Slug ")).toBe("Ursus Slug");
    });
});
