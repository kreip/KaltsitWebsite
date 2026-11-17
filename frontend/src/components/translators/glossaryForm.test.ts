import { describe, expect, it } from "vitest";
import { hasForm, missingTerms } from "./glossaryForm";

describe("glossary forms as the backend checks them", () => {
    it("matches by stem with an ending, a case form, an adjective", () => {
        expect(hasForm("Без Амии ничего не выйдет.", "Амия")).toBe(true);
        expect(hasForm("Спросите Маунтена.", "Маунтен")).toBe(true);
        expect(hasForm("Скажи Зое.", "Зоя")).toBe(true);
        expect(hasForm("Урсусский ребёнок.", "Урсус")).toBe(true);
        expect(hasForm("Из заражённых.", "Заражённый")).toBe(true);
        expect(hasForm("Мы вернулись на Родос Айленд.", "Родос Айленд")).toBe(true);
        expect(hasForm("Айленд Родос", "Родос Айленд")).toBe(false);
    });

    it("never takes another word for the form", () => {
        expect(hasForm("Это Доберманн.", "Доберман")).toBe(false);
        expect(hasForm("У Зоуи царапина.", "Зоя")).toBe(false);
    });

    it("names the terms a text lacks, once per English", () => {
        const term = { termId: 1, matched: "Dobermann", sheet: "", ru: "Доберман", ruPlural: null, qualifier: null, note: null, fromSova: false, found: null, verified: false, changedAfterApproval: false };
        expect(missingTerms("Доберманн…", [term])).toEqual([{ en: "Dobermann", ru: "Доберман" }]);
        expect(missingTerms("Доберман…", [term])).toEqual([]);
        expect(missingTerms("Доберманн…", [{ ...term, fromSova: true }])).toEqual([]);
    });
});
