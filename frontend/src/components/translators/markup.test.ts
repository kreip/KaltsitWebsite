import { describe, expect, it } from "vitest";
import { lostHighlights, parseMarkup, plainMarkup, sameTags, serializeMarkup, tagCounts, tagDiff, tagHealth, tagPairs, tokensIn } from "./markup";

const SAMPLES = [
    "Deals <@ba.kw>Arts damage</>",
    "ATK <@ba.vup>+{atk:0%}</>, <$ba.stun><@ba.kw>Stuns</></> for {stun} seconds",
    "<i>Whispers</i> and <color=#FF0000>shouts</color>, Dr. {@nickname}.",
    "<p=1>First.</><p=2>Second.</>",
    "<@lv.item><Roadblock></> blocks the way",
    "A stray </> closer, an unclosed <i>opener",
    "<size=20><b>Big</b></size>\nnext line",
    "<@ba.kw><i>mixed</> end</i>",
    "",
];

describe("the game's markup", () => {
    it("parses and writes back every text unchanged", () => {
        for (const s of SAMPLES) expect(serializeMarkup(parseMarkup(s))).toBe(s);
    });

    it("tells styles, terms, Unity spans and substitutions apart", () => {
        const nodes = parseMarkup("ATK <@ba.vup>+{atk:0%}</>, <$ba.stun>Stuns</> <i>now</i>");
        expect(nodes.map((n) => (n.kind === "span" ? `${n.type}:${n.name}` : n.kind))).toEqual(["text", "style:@ba.vup", "text", "term:$ba.stun", "text", "inline:i"]);
        const style = nodes[1];
        expect(style.kind === "span" && style.children).toEqual([
            { kind: "text", value: "+" },
            { kind: "token", raw: "{atk:0%}" },
        ]);
        expect(plainMarkup(nodes)).toBe("ATK +{atk:0%}, Stuns now");
    });

    it("keeps a name in angle brackets and a stray closer as text", () => {
        expect(plainMarkup(parseMarkup("<@lv.item><Roadblock></>"))).toBe("<Roadblock>");
        expect(parseMarkup("a </> b </i> c")).toEqual([{ kind: "text", value: "a </> b </i> c" }]);
    });

    it("lists the tags the palette offers, each once, with its closer", () => {
        expect(tagPairs("<@ba.vup>a</> <@ba.vup>b</> <$ba.stun>c</> <i>d</i> <color=#fff>e</color> <p=1>f</>")).toEqual([
            { type: "style", name: "@ba.vup", open: "<@ba.vup>", close: "</>" },
            { type: "term", name: "$ba.stun", open: "<$ba.stun>", close: "</>" },
            { type: "inline", name: "i", open: "<i>", close: "</i>" },
            { type: "color", name: "color", open: "<color=#fff>", close: "</color>" },
        ]);
        // Never closed in the original: the palette still closes it.
        expect(tagPairs("<i>open")[0].close).toBe("</i>");
    });

    it("lists the substitutions once", () => {
        expect(tokensIn("Dr. {@Nickname}, {0} and {atk:0%}, {0} again, {@nickname}")).toEqual(["{@nickname}", "{0}", "{atk:0%}"]);
        expect(tokensIn("no tokens <@ba.kw>here</>")).toEqual([]);
    });

    it("compares a translation's tags with the original's", () => {
        const en = "ATK <@ba.vup>+10</> and <@ba.vup>+5</>, <$ba.stun>Stuns</>";
        expect([...tagCounts(en)]).toEqual([
            ["@ba.vup", 2],
            ["$ba.stun", 1],
        ]);
        expect(sameTags(en, "АТК <@ba.vup>+10</> и <@ba.vup>+5</>, <$ba.stun>оглушает</>")).toBe(true);
        expect(tagDiff(en, "АТК <@ba.vup>+10</> и +5, <i>оглушает</i>")).toEqual({ missing: ["@ba.vup", "$ba.stun"], extra: ["i"] });
        // The colour itself may differ; the tag must be there.
        expect(sameTags("<color=#fff>a</color>", "<color=#000>б</color>")).toBe(true);
    });

    // The same examples stand in backend/src/translators/texts/problems.rs.
    it("knows which highlights the RichTextFix plugin puts back", () => {
        // Kolooo's skill: the numbers come back blue; a term's words are plain in the game anyway.
        const skill = "Deals <@ba.vup>230%</> ATK as Physical damage, <$ba.stun>Stuns</> the target for <@ba.vup>1.25</> seconds";
        expect(lostHighlights(skill, "Наносит 230% АТК физического урона, оглушая цель на 1.25 секунд")).toEqual([]);
        expect(tagHealth(skill, "Наносит 230% АТК физического урона, оглушая цель на 1.25 секунд")).toEqual({ broken: false, noHighlight: false });
        expect(lostHighlights("<$ba.stun>Stuns</> <i>all</i>", "Оглушает всех")).toEqual(["i"]);
        // Without the term, nothing is lost; a number written otherwise (1,25) is.
        const plain = "Deals <@ba.vup>230%</> ATK for <@ba.vup>1.25</> seconds";
        expect(lostHighlights(plain, "Наносит 230% АТК на 1.25 секунд")).toEqual([]);
        expect(lostHighlights(plain, "Наносит 230% АТК на 1,25 секунд")).toEqual(["@ba.vup"]);
        // A substitution stands for a number; a number of the span that is also outside it cannot be placed.
        expect(lostHighlights("ATK <@ba.vup>+{atk:0%}</>", "Атака +{atk:0%}")).toEqual([]);
        expect(lostHighlights("<@ba.vup>5</> of 5", "5 из 5")).toEqual(["@ba.vup"]);
        // A style over the whole text comes back; a word does not; a style the plugin does not know is dropped.
        expect(lostHighlights("<@ba.rem>Can store 2 charges</>", "Может накапливать 2 заряда")).toEqual([]);
        expect(lostHighlights("Gains <@ba.vup>Stealth</> now", "Получает невидимость")).toEqual(["@ba.vup"]);
        expect(lostHighlights("<@act.timeLimit>3 days</> left", "Осталось 3 дня")).toEqual(["@act.timeLimit"]);
        // An ASCII word is found whole only.
        expect(lostHighlights("Gains <@ba.vup>SP</> faster", "Быстрее получает SP")).toEqual([]);
        expect(lostHighlights("Gains <@ba.vup>SP</> faster", "Быстрее получает SPD")).toEqual(["@ba.vup"]);
        // A translation with tags of its own gets only whole-text styles back.
        expect(lostHighlights("<i>All</i> of <@ba.vup>10</> and <@ba.vup>20</>", "<i>Всё</i> из 10 и <@ba.vup>20</>")).toEqual(["@ba.vup"]);
        expect(lostHighlights("<@ba.rem>All <@ba.vup>10</></>", "Всё <@ba.vup>10</>")).toEqual([]);
    });
});
