import { describe, expect, it } from "vitest";
import { diffTokens, diffWords } from "./diff";

const show = (before: string, after: string) =>
    diffWords(before, after)
        .map((p) => (p.kind === "same" ? p.text : p.kind === "add" ? `[+${p.text}]` : `[-${p.text}]`))
        .join("");

describe("comparing versions by words", () => {
    it("splits a text into words, blanks, tags, substitutions and single characters", () => {
        expect(diffTokens("Привет, <i>{@nickname}</i>… не-а")).toEqual(["Привет", ",", " ", "<i>", "{@nickname}", "</i>", "…", " ", "не-а"]);
    });

    it("shows nothing changed for the same text", () => {
        expect(diffWords("Вы не из Урсуса.", "Вы не из Урсуса.")).toEqual([{ kind: "same", text: "Вы не из Урсуса." }]);
    });

    it("marks one replaced word inside the text", () => {
        expect(show("Вы, ребята... не из Урсуса...", "Вы, ребята... не из Урсина...")).toBe("Вы, ребята... не из [-Урсуса][+Урсина]...");
    });

    it("marks an added and a dropped word", () => {
        expect(show("Отличная работа, доктор.", "Отличная работа.")).toBe("Отличная работа[-, доктор].");
        expect(show("Отличная работа.", "Просто отличная работа.")).toBe("[-Отличная][+Просто отличная] работа.");
        expect(show("Отличная работа.", "Отличная работа, Доктор.")).toBe("Отличная работа[+, Доктор].");
    });

    it("reads a rewritten phrase as one dropped run and one added run", () => {
        expect(show("Похоже, они не успели предупредить остальных.", "Похоже, у них не было времени предупредить других.")).toBe("Похоже, [-они][+у них] не [-успели][+было времени] предупредить [-остальных][+других].");
        expect(show("старый текст здесь", "совсем новый вариант")).toBe("[-старый текст здесь][+совсем новый вариант]");
    });

    it("compares against an empty text", () => {
        expect(show("", "Новый текст")).toBe("[+Новый текст]");
        expect(show("Старый текст", "")).toBe("[-Старый текст]");
    });

    it("keeps tags and substitutions whole", () => {
        expect(show("<i>Тихо.</i> {@nickname}?", "Тихо. {@nickname}!")).toBe("[-<i>]Тихо.[-</i>] {@nickname}[-?][+!]");
    });
});
