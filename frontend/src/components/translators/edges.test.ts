import { describe, expect, it } from "vitest";
import { trimEdges, trimRange } from "./edges";

describe("a selection's edges", () => {
    it("drops blanks and punctuation at the edges only", () => {
        expect(trimEdges("Гвард, ")).toBe("Гвард");
        expect(trimEdges(" «Родос Айленд»… ")).toBe("Родос Айленд");
        expect(trimEdges("Фристон-3")).toBe("Фристон-3");
        expect(trimEdges("Ch'en.")).toBe("Ch'en");
        expect(trimEdges("...")).toBe("");
    });

    it("leaves a word alone selected, wherever the space after it stands", () => {
        const box = document.createElement("div");
        const selected = (html: string, pick: (r: Range) => void) => {
            box.innerHTML = html;
            const range = document.createRange();
            pick(range);
            return trimRange(range);
        };
        const text = (i: number) => box.childNodes[i] as Node;
        const word = selected("Гвард идёт", (r) => {
            r.setStart(text(0), 0);
            r.setEnd(text(0), 6);
        });
        expect(word.toString()).toBe("Гвард");
        // The space is the next span's: the end comes back out of it.
        const before = selected("Гвард<span> идёт</span>", (r) => {
            r.setStart(text(0), 0);
            r.setEnd(text(1).firstChild as Node, 1);
        });
        expect(before.toString()).toBe("Гвард");
        expect(before.endContainer).toBe(text(0));
        // The box's end as an element, the guard there.
        const last = selected("идёт Гвард \u200b", (r) => {
            r.setStart(text(0), 5);
            r.setEnd(box, 1);
        });
        expect(last.toString()).toBe("Гвард");
        // A chip is whole.
        const chip = selected('<span data-token="{0}">{0}</span> урона', (r) => {
            r.setStart(box, 0);
            r.setEnd(text(1), 1);
        });
        expect(chip.toString()).toBe("{0}");
        expect(selected(" , ", (r) => r.selectNodeContents(box)).collapsed).toBe(true);
    });
});
