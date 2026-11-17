import { describe, expect, it } from "vitest";
import { wordSwap } from "./wordSwap";

describe("wordSwap", () => {
    it("finds one word changed for another", () => {
        expect(wordSwap("Бибек, ты где?", "Байбик, ты где?")).toEqual({ from: "Бибек", to: "Байбик" });
        expect(wordSwap("<i>Скажи Бибеку.</i>", "<i>Скажи Байбику.</i>")).toEqual({ from: "Бибеку", to: "Байбику" });
        expect(wordSwap("Это Рэндалл.", "Это Рэндолл.")).toEqual({ from: "Рэндалл", to: "Рэндолл" });
        expect(wordSwap("Это Доберман.", "Это Доберманн.")).toEqual({ from: "Доберман", to: "Доберманн" });
    });

    it("leaves a change of ending, of several words or of English alone", () => {
        expect(wordSwap("Скажи Бибек.", "Скажи Бибеку.")).toBeNull();
        expect(wordSwap("У Зоя царапина.", "У Зои царапина.")).toBeNull();
        expect(wordSwap("Бибек и Зоя.", "Байбик и Зои.")).toBeNull();
        expect(wordSwap("Бибек тут.", "Бибек тут!")).toBeNull();
        expect(wordSwap("Это SP.", "Это ОН.")).toBeNull();
        expect(wordSwap("Бибек тут.", "Бибек сейчас тут.")).toBeNull();
    });
});
