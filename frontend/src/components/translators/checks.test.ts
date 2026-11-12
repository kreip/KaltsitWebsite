import { describe, expect, it } from "vitest";
import type { TranslatorLine } from "#/types/generated/TranslatorLine";
import { addressOf, checkVariant, type ICandidate, lineProblems, neighbourAddress, passes, recommend } from "./checks";
import { tagHealth } from "./markup";

function line(over: Partial<TranslatorLine>): TranslatorLine {
    return {
        line: 1,
        part: 0,
        kind: "dialog",
        speakerEn: "Amiya",
        speakerRu: "Амия",
        en: "Doctor, look.",
        ruGame: "Доктор, смотрите.",
        ruGameBefore: null,
        ruGameChangedAt: null,
        ruGameSource: null,
        ruGameMatch: null,
        pluginRole: null,
        ruDict: null,
        speakerCn: null,
        cn: null,
        sova: null,
        ai: null,
        terms: [],
        editKey: "Doctor, look.",
        editable: true,
        contextAllowed: true,
        places: 1,
        status: "unchecked",
        variantGlossary: [],
        ...over,
    };
}

describe("variant checks", () => {
    it("pass a faithful variant", () => {
        expect(passes(checkVariant("Доктор, взгляните.", "sova", line({}), null))).toBe(true);
    });

    it("catch a lost tag, a lost substitution and a changed line break", () => {
        const l = line({ en: "<i>{@nickname}</i>,\\nlook.", ruGame: "<i>{@nickname}</i>,\nсмотри." });
        expect(passes(checkVariant(l.ruGame ?? "", "game", l, null))).toBe(true);
        const c = checkVariant("Доктор, смотри.", "ai", l, null);
        expect(c.brokenTag).toBe(true);
        expect(c.placeholders).toEqual(["{@nickname}"]);
        expect(c.newlines).toEqual({ en: 1, ru: 0 });
    });

    it("mark a variant noticeably longer than the game's, never a short one", () => {
        const l = line({ ruGame: "Доктор, смотрите." });
        expect(checkVariant("Доктор, смотрите внимательно на этот огромный корабль.", "proposal:1", l, null).longer).toBeGreaterThan(0.3);
        expect(checkVariant("Доктор, гляньте-ка.", "proposal:1", l, null).longer).toBeNull();
    });

    it("read ты and вы, and compare with the speaker's neighbouring lines", () => {
        expect(addressOf("Я помогу тебе.")).toBe("ты");
        expect(addressOf("Ваш план хорош, вы правы.")).toBe("вы");
        expect(addressOf("Смотрите.")).toBeNull();
        const lines = [line({ ruGame: "Вы устали?" }), line({ ruGame: "Вам помочь?" }), line({ ruGame: "?" }), line({ speakerEn: "Kal'tsit", ruGame: "Ты." })];
        expect(neighbourAddress(lines, 2)).toBe("вы");
        expect(checkVariant("Тебе помочь?", "sova", lines[2], "вы").address).toBe("вы");
        expect(checkVariant("Вам помочь?", "sova", lines[2], "вы").address).toBeNull();
    });

    it("use the backend's glossary verdict for the variant", () => {
        const l = line({ variantGlossary: [{ variant: "sova", missing: [{ en: "Ursus", ru: "Урсин" }] }] });
        expect(checkVariant("Урсус.", "sova", l, null).glossary).toEqual([{ en: "Ursus", ru: "Урсин" }]);
        expect(checkVariant("Урсус.", "ai", l, null).glossary).toEqual([]);
    });
});

describe("tags", () => {
    const en = "Deals <@ba.vup>120%</> damage to {0} targets";
    it("tell a broken tag from a missing highlight, as the backend does", () => {
        expect(tagHealth(en, "Наносит <@ba.vup>120%</> урона {0} целям")).toEqual({ broken: false, noHighlight: false });
        expect(tagHealth(en, "Наносит 120% урона {0} целям")).toEqual({ broken: false, noHighlight: true });
        expect(tagHealth(en, "Наносит <@ba.vup>120% урона {0} целям")).toEqual({ broken: true, noHighlight: false });
        expect(tagHealth(en, "Наносит <@ba.vup120%</> урона {0} целям")).toEqual({ broken: true, noHighlight: false });
        expect(tagHealth(en, "Наносит <@ba.vup>120%</> урона целям")).toEqual({ broken: true, noHighlight: false });
        expect(tagHealth("Plain", "Просто</>")).toEqual({ broken: true, noHighlight: false });
        expect(tagHealth("<Roadblock> ahead", "<Заграждение> впереди")).toEqual({ broken: false, noHighlight: false });
    });

    it("name a line's problems, none once approved", () => {
        const l = line({ en, ruGame: "Наносит 120% урона {0} целям" });
        const checks = new Map([["game", checkVariant(l.ruGame ?? "", "game", l, null)]]);
        expect([...lineProblems(false, checks)]).toEqual(["noHighlight"]);
        expect([...lineProblems(true, checks)]).toEqual([]);
    });
});

describe("recommended", () => {
    const ok = checkVariant("Доктор, взгляните.", "sova", line({}), null);
    const bad = { ...ok, brokenTag: true };
    const c = (variant: string, agreements: number, pass = true): ICandidate => ({ variant, agreements, check: pass ? ok : bad });

    it("rule B: nothing when the game's Russian passes; else the best passing variant", () => {
        expect(recommend([c("proposal:1", 3), c("sova", 0), c("game", 0)], false)).toBeNull();
        expect(recommend([c("proposal:1", 1), c("proposal:2", 3), c("sova", 0), c("game", 0, false)], false)).toBe("proposal:2");
        expect(recommend([c("proposal:1", 5, false), c("sova", 0), c("ai", 0), c("game", 0, false)], false)).toBe("sova");
        expect(recommend([c("sova", 0, false), c("ai", 0), c("game", 0, false)], false)).toBe("ai");
        expect(recommend([c("sova", 0, false), c("game", 0, false)], false)).toBeNull();
        expect(recommend([c("sova", 0), c("game", 0, false)], true)).toBeNull();
    });

    it("rule A (switched off): the best passing variant, the game's Russian last", () => {
        expect(recommend([c("proposal:1", 1), c("proposal:2", 3), c("sova", 0), c("game", 0)], false, "A")).toBe("proposal:2");
        expect(recommend([c("sova", 0, false), c("ai", 0, false), c("game", 0)], false, "A")).toBe("game");
        expect(recommend([c("sova", 0, false), c("game", 0, false)], false, "A")).toBeNull();
    });
});
