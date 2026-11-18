import { describe, expect, it } from "vitest";
import { trimEdges } from "./edges";

describe("a selection's edges", () => {
    it("drops blanks and punctuation at the edges only", () => {
        expect(trimEdges("Гвард, ")).toBe("Гвард");
        expect(trimEdges(" «Родос Айленд»… ")).toBe("Родос Айленд");
        expect(trimEdges("Фристон-3")).toBe("Фристон-3");
        expect(trimEdges("Ch'en.")).toBe("Ch'en");
        expect(trimEdges("...")).toBe("");
    });
});
