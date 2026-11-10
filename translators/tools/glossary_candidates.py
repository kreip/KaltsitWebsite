"""Candidates for the team glossary, for its keeper to name.

Reads (only): the section's story lines and glossary (CSV exported from the
database), EN gamedata (operators, NPCs, enemies, teams and nations, operator
dossiers: race and place of birth, items), the ArknightsRU dictionaries.
Checks each against arknights.wiki.gg (MediaWiki API, about one request a
second, cached next to the output in wiki-cache/). Writes one xlsx with a
sheet per category of the glossary's keeper.
"""

import collections
import csv
import json
import re
import sys

import ahocorasick
from openpyxl import Workbook
from openpyxl.styles import Alignment, Font, PatternFill
from openpyxl.utils import get_column_letter

csv.field_size_limit(10**9)
GD = "/gd"
DICT = "/dict"
LINES = "/gl/lines.csv"
GLOSSARY = "/gl/glossary.csv"
OUT = sys.argv[1]

# Thresholds (written into the report).
SPEAKER_MIN = 10        # a speaker with at least this many lines
WORD_MIN = 15           # a capitalised word mid-sentence at least this often…
WORD_LOWER_MAX = 0.2    # …and lower-case at most this share of that
PHRASE_MIN = 8          # a capitalised phrase of 2–3 words at least this often
ENTITY_MIN = 3          # enemies, items from gamedata: mentioned in at least this many lines
VARIANT_MIN, VARIANT_SHARE, VARIANT_PRECISION = 2, 0.05, 0.25
COMMON_SHARE = 0.02     # for a meaning's row: a word in more than 2 % of all lines is a common word
MIXED_SHARE = 0.10      # "inconsistent": the second variant has at least 10 % of the first's lines…
MIXED_MIN = 3           # …and at least 3 lines

SHEETS = ["Игровые персонажи", "Расы и не только", "Организации", "Страны, города и локации", "Различные термины"]


def load(name):
    return json.load(open(f"{GD}/{name}.json", encoding="utf-8"))


def kv(lst):
    return {x["key"]: x["value"] for x in lst}


TAG = re.compile(r"<[^<>]*>")


def clean(s):
    return TAG.sub("", s).replace("{@nickname}", "Doctor").replace("{@Nickname}", "Doctor").replace("\\n", "\n")


# ---------- Russian stems (as the section's matcher) ----------
ENDINGS = ["ами", "ями", "ого", "его", "ому", "ему", "ыми", "ими", "ой", "ей", "ом", "ем", "ам", "ям", "ах", "ях", "ов", "ев", "ий", "ый", "ая", "яя", "ое", "ее", "ую", "юю", "ы", "и", "а", "я", "у", "ю", "е", "о", "ь", "й"]


def stem(w):
    for e in ENDINGS:
        if w.endswith(e) and len(w) - len(e) >= 3:
            return w[: -len(e)]
    return w


def words(s):
    return [w for w in re.split(r"[^\w]+", s.lower().replace("ё", "е")) if w]


def has_form(text, form):
    form = re.sub(r"\(.*?\)", "", form)
    stems = [stem(w) for w in words(form.replace("«", " ").replace("»", " ").replace('"', " ")) if w != "ооо"]
    if not stems:
        return False
    tw = words(text)
    return any(all(tw[i + j].startswith(stems[j]) for j in range(len(stems))) for i in range(len(tw) - len(stems) + 1))


RU_WORD = re.compile(r"[А-Яа-яЁё]{3,}")


def ru_stems(s):
    return [(stem(w.lower().replace("ё", "е")), w) for w in RU_WORD.findall(s)]


# ---------- data ----------
print("reading lines…", file=sys.stderr)
lines = []  # (story, ord, kind, speaker_en, speaker_ru, speaker_cn, en, ru, sova)
with open(LINES, encoding="utf-8") as f:
    for r in csv.reader(f):
        lines.append(r)
EN = [clean(r[6]) for r in lines]
RU = [r[7] for r in lines]
SOVA = [r[8] for r in lines]

df_ru = collections.Counter()
df_sova = collections.Counter()
for t in RU:
    df_ru.update({s for s, _ in ru_stems(t)})
for t in SOVA:
    df_sova.update({s for s, _ in ru_stems(t)})

speakers = collections.Counter(r[3] for r in lines if r[3])
speaker_ru = collections.defaultdict(collections.Counter)
speaker_cn = collections.defaultdict(collections.Counter)
for r in lines:
    if r[3]:
        if r[4]:
            speaker_ru[r[3]][r[4]] += 1
        if r[5]:
            speaker_cn[r[3]][r[5]] += 1

glossary = collections.defaultdict(list)
with open(GLOSSARY, encoding="utf-8") as f:
    for sheet, en, key, qual, ru, plural, cs, from_sova, cn in csv.reader(f):
        glossary[key.lower()].append({"sheet": sheet, "ru": ru, "plural": plural, "from_sova": from_sova == "t", "cn": cn, "qual": qual})


def decode(raw):
    out, i, eq = ["", ""], 0, 0
    while i < len(raw):
        c = raw[i]
        if c == "\\" and i + 1 < len(raw):
            n = raw[i + 1]
            out[eq] += {"=": "=", "\\": "\\", "n": "\n", "r": "\r"}.get(n, "\\" + n)
            i += 2
            continue
        if c == "/" and raw[i + 1 : i + 2] == "/":
            break
        if c == "=":
            if eq:
                return None
            eq = 1
        else:
            out[eq] += c
        i += 1
    return (out[0], out[1]) if eq and out[0] and out[1] else None


dictionary = {}
for name in ["12_story.txt", "13_names.txt", "11_ui.txt"]:  # later wins, as the game loads them
    for raw in open(f"{DICT}/{name}", encoding="utf-8").read().split("\n"):
        d = decode(raw.rstrip("\r"))
        if d:
            dictionary[d[0]] = d[1]

# ---------- candidates from gamedata ----------
cands = {}  # key (EN as written + meaning) -> dict


def add(en, sheet, source, meaning=None, always=False):
    en = en.strip()
    if not en or len(en) < 2 or not re.search(r"[A-Za-z]", en):
        return
    mined = "тексте сюжета" in source or source.startswith("Говорящий")
    k = (en, meaning)
    c = cands.get(k)
    if c is None:
        c = cands[k] = {"en": en, "sheet": sheet, "sources": [], "meaning": meaning, "always": always, "mined": mined}
    elif c["mined"] and not mined:
        # The game's own tables say better than a guess from the text.
        c["sheet"], c["mined"] = sheet, False
    if source not in c["sources"]:
        c["sources"].append(source)
    c["always"] = c["always"] or always


chars = kv(load("character_table")["Characters"])
nations, groups = set(), set()
for cid, c in chars.items():
    if c.get("Profession") in ("TOKEN", "TRAP"):
        continue
    add(c["Name"], SHEETS[0], "Оператор", always=True)
    if c.get("NationId"):
        nations.add(c["NationId"])
    for g in ("GroupId", "TeamId"):
        if c.get(g):
            groups.add(c[g])

hb = load("handbook_info_table")
for nid, n in kv(hb["NpcDict"]).items():
    add(n["Name"], SHEETS[0], "NPC (справочник)", always=True)

races, births = collections.Counter(), collections.Counter()
for cid, h in kv(hb["HandbookDict"]).items():
    s = json.dumps(h, ensure_ascii=False)
    for m in re.finditer(r"\[Race\] ([^\\\n\"]+)", s):
        for r in re.split(r"[/,]| or ", m.group(1)):
            if r.strip() and r.strip().lower() not in ("unknown", "undisclosed", "n/a"):
                races[r.strip()] += 1
    for m in re.finditer(r"\[Place of Birth\] ([^\\\n\"]+)", s):
        for p in re.split(r"[/,]", m.group(1)):
            if p.strip() and p.strip().lower() not in ("unknown", "undisclosed", "n/a"):
                births[p.strip()] += 1
race_names = {r for r, n in races.items() if n >= 1}

teams = kv(load("handbook_team_table")["Handbook_teams"])
nation_names = set()
for tid, t in teams.items():
    if tid == "none":
        continue
    if tid in nations:
        nation_names.add(t["PowerName"])

# Nation and race at once (Ursus): two rows, one per meaning.
for tid, t in teams.items():
    if tid == "none":
        continue
    name = t["PowerName"]
    if tid == "rhodes":
        # The game files Rhodes Island as operators' "nation"; it is a company.
        add(name, SHEETS[2], "Организация (справочник фракций; у операторов — как «нация»)", always=True)
    elif tid in nations:
        add(name, SHEETS[3], "Страна (справочник фракций)", meaning="страна" if name in race_names else None, always=True)
    else:
        add(name, SHEETS[2], "Организация (справочник фракций)", always=True)
for r in race_names:
    add(r, SHEETS[1], "Раса (досье операторов)", meaning="раса" if r in nation_names else None, always=True)
for p, n in births.items():
    if p not in nation_names:
        if p != "Rhodes Island":
            add(p, SHEETS[3], "Место рождения (досье операторов)")

eh = load("enemy_handbook_table")
for eid, e in kv(eh["EnemyData"]).items():
    boss = e.get("EnemyLevel") == "BOSS"
    add(e["Name"], SHEETS[0] if boss else SHEETS[4], "Враг (босс)" if boss else "Враг")
for rid, r in kv(eh["RaceData"]).items():
    add(r["RaceName"], SHEETS[1], "Тип врага")

it = load("item_table")
for iid, i in kv(it["Items"]).items():
    add(i["Name"], SHEETS[4], "Предмет")

# ---------- candidates mined from the story ----------
print("mining the story…", file=sys.stderr)
STOP = set("I I'm I'll I've I'd Oh Ah Eh Uh Hm Hmm Huh Yes No Ok Okay OK Well So But And Or If Then The A An This That These Those It It's He She We They You Your My Our Their His Her What Who Why How When Where Mr Ms Mrs Miss Sir Lord Lady Captain Doctor Dr".split())
LOC = set("City Island Islands Mountain Mountains Plains District Street Square Base Tower Port Gate Forest Desert Sea River Lake Valley Wastes Ruins Plaza Hall Bridge Station Camp Town Village Region Province Territory Mine Mines Hospital Castle Palace Cathedral Prison Arena Harbor Harbour Wall Walls Ridge Canyon Highlands Wilderness Frontier Borderlands".split())
ORG = set("Company Guard Guards Army Corps Division Association Order Church Council Department Bureau Squad Team Group Gang Family Knights Industries Inc Agency Union Federation Senate Court Office Academy School University Committee Brigade Battalion Legion Syndicate Guild Institute Party Movement Front Pharmaceutical Pharmaceuticals Garrison Militia Watch Police Navy Regiment Clan Tribe Commission Foundation Chamber".split())
word_re = re.compile(r"[A-Za-z][A-Za-z'\-]*")
cap_count, low_count, phrase_count = collections.Counter(), collections.Counter(), collections.Counter()
for text in EN:
    for sent in re.split(r"(?<=[.!?…])\s+|\n|[\"“”«»]", text):
        toks = [re.sub(r"('s|s'|')$", "", w) for w in word_re.findall(sent)]
        toks = [w for w in toks if w]
        run = []
        for i, w in enumerate(toks):
            mid = i > 0
            if w[0].isupper() and not w.isupper() and mid and w not in STOP:
                cap_count[w] += 1
                run.append(w)
            else:
                if w.islower():
                    low_count[w.capitalize()] += 1
                if len(run) >= 2:
                    for n in (2, 3):
                        for j in range(len(run) - n + 1):
                            phrase_count[" ".join(run[j : j + n])] += 1
                run = []
        if len(run) >= 2:
            for n in (2, 3):
                for j in range(len(run) - n + 1):
                    phrase_count[" ".join(run[j : j + n])] += 1


def classify(phrase):
    last = phrase.split()[-1]
    if last in LOC:
        return SHEETS[3]
    if any(w in ORG for w in phrase.split()):
        return SHEETS[2]
    return SHEETS[4]


for w, n in cap_count.items():
    if n >= WORD_MIN and low_count[w] <= WORD_LOWER_MAX * n:
        add(w, classify(w), "Слово с заглавной в тексте сюжета")
for p, n in phrase_count.items():
    if n >= PHRASE_MIN:
        add(p, classify(p), "Сочетание с заглавными в тексте сюжета")
for s, n in speakers.items():
    if n >= SPEAKER_MIN and re.search(r"[A-Za-z]{2}", s) and "?" not in s:
        add(s, SHEETS[0], f"Говорящий в сюжете")
add("Rhodes Island", SHEETS[2], "Организация (справочник фракций)", always=True)

# ---------- counting ----------
print(f"{len(cands)} raw candidates; counting…", file=sys.stderr)
A = ahocorasick.Automaton()
for k, c in cands.items():
    A.add_word(c["en"], c["en"])
A.make_automaton()
mentions = collections.defaultdict(list)
for i, text in enumerate(EN):
    seen = set()
    for end, en in A.iter(text):
        start = end - len(en) + 1
        if (start > 0 and text[start - 1].isalnum()) or (end + 1 < len(text) and text[end + 1].isalnum()):
            continue
        if en not in seen:
            seen.add(en)
            mentions[en].append(i)

# Longer names win: "Rhodes Island" lines do not count for "Rhodes".
# (Kept simple: both counted; the glossary keeper decides.)

COUNTRY_CTX = re.compile(r"\b(in|to|from|of|across|into|toward|towards|leave|left|reach|enter|invade|invaded|Empire of|Imperial)\s+{}\b")
RACE_CTX = re.compile(r"\b(an?|young|old|another|fellow)\s+{}\b(?!\s+[A-Z])|\b{}\s+(man|woman|men|women|kid|kids|child|children|boy|girl|blood|guy|guys|lineage|ancestry|race)\b")


def meaning_lines(c, idx):
    if c["meaning"] is None:
        return idx
    name = re.escape(c["en"])
    pat = re.compile(COUNTRY_CTX.pattern.format(name)) if c["meaning"] == "страна" else re.compile(RACE_CTX.pattern.format(name, name))
    return [i for i in idx if pat.search(EN[i])]


def variants(idx, texts, df, precision=VARIANT_PRECISION):
    with_ru = [texts[i] for i in idx if texts[i]]
    if not with_ru:
        return []
    allst = sorted({s for t in with_ru for s, _ in ru_stems(t)}, key=len)
    canon, roots = {}, []
    for s in allst:
        r = next((r for r in roots if len(r) >= 4 and s.startswith(r)), None)
        if r is None:
            roots.append(s)
            r = s
        canon[s] = r
    cnt = collections.Counter()
    spell = collections.defaultdict(collections.Counter)
    for t in with_ru:
        seen = set()
        for s, w in ru_stems(t):
            r = canon[s]
            spell[r][w] += 1
            if r not in seen:
                seen.add(r)
                cnt[r] += 1
    rdf = collections.Counter()
    for s, r in canon.items():
        rdf[r] += df.get(s, 0)
    n = len(with_ru)
    ok = [r for r, c in cnt.items() if c >= VARIANT_MIN and c >= VARIANT_SHARE * n and c / max(rdf[r], c, 1) >= precision and (precision > 0 or rdf[r] <= COMMON_SHARE * len(texts))]
    # Words that nearly always come together are one variant ("Остров Родоса"),
    # in the order they stand in the text.
    lines_of = {r: set() for r in ok}
    pos = collections.defaultdict(list)
    for li, t in enumerate(with_ru):
        for wi, (s_, w) in enumerate(ru_stems(t)):
            r = canon[s_]
            if r in lines_of:
                lines_of[r].add(li)
                pos[r].append(wi)
    groups = []
    for r in sorted(ok, key=lambda r: -cnt[r]):
        for g in groups:
            a, b = lines_of[g[0]], lines_of[r]
            if len(a & b) >= 0.8 * max(len(a), len(b)):
                g.append(r)
                break
        else:
            groups.append([r])
    out = []
    for g in groups:
        g.sort(key=lambda r: sum(pos[r]) / max(len(pos[r]), 1))
        label = " ".join(spell[r].most_common(1)[0][0] for r in g)
        both = set.intersection(*(lines_of[r] for r in g))
        out.append((label, len(both)))
    out.sort(key=lambda x: -x[1])
    return out[:5]


def example(idx):
    good = [i for i in idx if 30 <= len(EN[i]) <= 200 and RU[i]]
    i = (good or idx or [None])[0]
    return ("", "") if i is None else (EN[i], RU[i])


rows = []
for k, c in cands.items():
    idx = meaning_lines(c, mentions.get(c["en"], []))
    spoken = speakers.get(c["en"], 0) if c["meaning"] is None else 0
    total = len(idx) + spoken
    if total == 0 and not c["always"]:
        continue
    if any(s in ("Враг", "Предмет") for s in c["sources"]) and len(c["sources"]) == 1 and len(idx) < ENTITY_MIN:
        continue
    # A meaning picked out of a word's lines (Ursus the race) is a small share of that word: no precision bar.
    prec = 0.0 if c["meaning"] else VARIANT_PRECISION
    vs = variants(idx, RU, df_ru, prec)
    if spoken:
        sv = speaker_ru.get(c["en"], collections.Counter())
        merged = collections.Counter(dict(vs))
        for name, nn in sv.items():
            merged[name] += nn
        vs = merged.most_common(5)
    in_dict = dictionary.get(c["en"])
    top = vs[0][0] if vs else (in_dict or "")
    sova = variants(idx, SOVA, df_sova, prec)
    cn = speaker_cn.get(c["en"], collections.Counter()).most_common(1)
    g = glossary.get(c["en"].lower(), [])
    gl = "; ".join(f"да: {x['sheet']} — {x['ru']}" + (f" ({x['qual']})" if x["qual"] else "") for x in g) or "нет"
    gcn = next((x["cn"] for x in g if x["cn"]), "")
    cons = ""
    checked = [i for i in idx if RU[i]]
    forms = [f for x in g if not x["from_sova"] for f in [*x["ru"].split(" / "), x["plural"]] if f]
    if forms and checked:
        ok = sum(1 for i in checked if any(has_form(RU[i], f) for f in forms))
        cons = f"{round(100 * ok / len(checked))} % ({ok} из {len(checked)})"
    mixed = len(vs) >= 2 and vs[1][1] >= MIXED_MIN and vs[1][1] >= MIXED_SHARE * vs[0][1]
    ex_en, ex_ru = example(idx)
    src = ", ".join(c["sources"])
    if spoken:
        src += f"; говорит в сюжете: {spoken} реплик"
    if idx:
        src += f"; упоминается в сюжете: {len(idx)} строк"
    en = c["en"] + (f" ({c['meaning']})" if c["meaning"] else "")
    rows.append({
        "sheet": c["sheet"], "en": en, "raw": c["en"], "meaning": c["meaning"], "sources": list(c["sources"]), "top": top,
        "variants": ("⚠ разнобой: " if mixed else "") + "; ".join([f"{v} — {n}" for v, n in vs] + ([f"в словаре: {in_dict}"] if in_dict and in_dict not in dict(vs) else [])),
        "sova": "; ".join(f"{v} — {n}" for v, n in sova), "cn": (cn[0][0] if cn else gcn),
        "count": total, "ex_en": ex_en, "ex_ru": ex_ru, "src": src, "gl": gl, "cons": cons, "mixed": mixed,
    })

# A word with meanings (Ursus: country, race) keeps only its meaning rows.
with_meanings = {r["raw"] for r in rows if r["meaning"]}
rows = [r for r in rows if r["meaning"] or r["raw"] not in with_meanings]

# ---------- arknights.wiki.gg ----------
import os
import time
import urllib.parse
import urllib.request

WIKI = "https://arknights.wiki.gg/api.php"
UA = "KaltsitWebsite-translators/0.1 (glossary candidates; github.com/kreip/KaltsitWebsite)"
CACHE_DIR = os.path.join(os.path.dirname(OUT), "wiki-cache")
CACHE = os.path.join(CACHE_DIR, "pages.json")
os.makedirs(CACHE_DIR, exist_ok=True)
cache = json.load(open(CACHE, encoding="utf-8")) if os.path.exists(CACHE) else {}
last_call = [0.0]


def api(params):
    # Not more than about one request a second.
    wait = 1.05 - (time.time() - last_call[0])
    if wait > 0:
        time.sleep(wait)
    url = WIKI + "?" + urllib.parse.urlencode({**params, "format": "json", "formatversion": "2"})
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    for attempt in range(3):
        try:
            with urllib.request.urlopen(req, timeout=60) as r:
                last_call[0] = time.time()
                return json.load(r)
        except Exception as e:  # noqa: BLE001
            last_call[0] = time.time()
            print(f"wiki retry {attempt}: {e}", file=sys.stderr)
            time.sleep(5)
    return {}


def save_cache():
    json.dump(cache, open(CACHE, "w", encoding="utf-8"), ensure_ascii=False)


BAD = re.compile(r"[#<>\[\]|{}_]")


def fetch_pages(titles):
    """Categories, final title and URL for each title (redirects followed), into the cache."""
    todo = [t for t in dict.fromkeys(titles) if t not in cache and not BAD.search(t) and len(t) < 200]
    print(f"wiki: {len(todo)} titles to look up ({len(titles) - len(todo)} cached or skipped)", file=sys.stderr)
    for i in range(0, len(todo), 50):
        batch = todo[i : i + 50]
        params = {"action": "query", "titles": "|".join(batch), "redirects": "1", "prop": "categories|info", "inprop": "url", "cllimit": "max", "clshow": "!hidden"}
        pages, maps = {}, {}
        cont = {}
        while True:
            d = api({**params, **cont})
            q = d.get("query", {})
            for m in q.get("normalized", []) + q.get("redirects", []):
                maps[m["from"]] = m["to"]
            for pg in q.get("pages", []):
                e = pages.setdefault(pg["title"], {"title": pg["title"], "missing": "missing" in pg or "invalid" in pg, "categories": [], "url": pg.get("fullurl", "")})
                e["categories"] += [c["title"].removeprefix("Category:") for c in pg.get("categories", [])]
            if "continue" not in d:
                break
            cont = d["continue"]
        for t in batch:
            final = t
            for _ in range(3):
                final = maps.get(final, final)
            cache[t] = pages.get(final, {"title": final, "missing": True, "categories": [], "url": ""})
        save_cache()


def fetch_extracts(titles):
    todo = [t for t in dict.fromkeys(titles) if "extract" not in cache.get(t, {}) and not cache.get(t, {}).get("missing", True)]
    print(f"wiki: {len(todo)} extracts", file=sys.stderr)
    for i in range(0, len(todo), 20):
        batch = todo[i : i + 20]
        finals = {cache[t]["title"]: t for t in batch}
        d = api({"action": "query", "titles": "|".join(finals), "prop": "extracts", "exintro": "1", "explaintext": "1", "exsentences": "3", "exlimit": "20"})
        got = {pg["title"]: pg.get("extract", "") for pg in d.get("query", {}).get("pages", [])}
        for final, t in finals.items():
            cache[t]["extract"] = got.get(final, "")
        save_cache()


def wiki_title(r):
    return f"{r['raw']} (race)" if r["meaning"] == "раса" else r["raw"]


fetch_pages([wiki_title(r) for r in rows] + [r["raw"] for r in rows if r["meaning"] == "раса"])
fetch_extracts([wiki_title(r) for r in rows if set(cache.get(wiki_title(r), {}).get("categories", [])) & {"Locations", "Lore"}])

# ---------- Romanov's categories ----------
CATS = ["Основные термины", "Имена", "Расы", "Страны", "Города", "Локации", "Организации и образования", "Явления", "Животные"]
CLASSES = {"Vanguard", "Guard", "Defender", "Sniper", "Caster", "Medic", "Supporter", "Specialist"}
CITY = re.compile(r"\b(city|cities|town|village|metropolis|capital|settlement)\b", re.I)
# In the defining phrase only ("X is a progressive, terminal disease"), not anywhere in the intro.
PHENOMENON = re.compile(r"\b(?:is|are) (?:an? |the )?(?:\S+ ){0,5}?(?:disease|diseases|disaster|disasters|syndrome|phenomenon|phenomena|calamity|calamities|catastrophe|catastrophes|storm|storms|plague|eclipse)\b", re.I)
CREATURE = re.compile(r"\b(?:is|are) (?:an? |the )?(?:\S+ ){0,5}?(?:creature|creatures|animal|animals|beast|beasts|species)\b", re.I)


def defining(extract):
    """The article's first real sentence, without the "This article is about…" note and punctuation."""
    text = "\n".join(l for l in extract.split("\n") if l and not l.startswith("This article is about"))
    return re.sub(r"[\"“”,;()]", " ", text)[:400].replace("  ", " ")


ANIMAL = {"Infected Creature", "Animals", "Fauna", "Beasts"}


def guess(r):
    """Our category and name type from the sources, before the wiki."""
    src = " ".join(r["sources"])
    if r["meaning"] == "раса":
        return "Расы", ""
    if r["meaning"] == "страна":
        return "Страны", ""
    if "Оператор" in src:
        return "Имена", "оператор"
    if "NPC" in src:
        return "Имена", "NPC"
    if "Враг" in src:
        return "Имена", "враг"
    if "Говорящий" in src:
        return "Имена", "говорящий"
    if "Раса" in src or "Тип врага" in src:
        return "Расы", ""
    if "Страна" in src:
        return "Страны", ""
    if "Организация" in src:
        return "Организации и образования", ""
    if "Место рождения" in src:
        return "Города", ""
    last = r["raw"].split()[-1]
    if last in ("City", "Town", "Village"):
        return "Города", ""
    if r["sheet"] == "Страны, города и локации":
        return "Локации", ""
    if r["sheet"] == "Организации":
        return "Организации и образования", ""
    return "Основные термины", ""


def by_wiki(page, ours):
    cats = set(page["categories"])
    ext = page.get("extract", "")
    if cats & ANIMAL:
        return "Животные", "враг" if any("enemies" in c for c in cats) else ""
    if any(c.endswith("enemies") for c in cats):
        return "Имена", "враг"
    if cats & CLASSES or any(c.endswith("-star") for c in cats):
        return "Имена", "оператор"
    if "NPCs" in cats or "Characters" in cats:
        return "Имена", "NPC"
    if "Races" in cats:
        return "Расы", ""
    if "Nation-states" in cats:
        return "Страны", ""
    if cats & {"Major factions", "Enemy factions", "Organizations", "Factions"}:
        return "Организации и образования", ""
    if "Locations" in cats:
        return ("Города" if CITY.search(ext) else "Локации"), ""
    if "Lore" in cats:
        d = defining(ext)
        if CREATURE.search(d) or page["title"].endswith("beast"):
            return "Животные", ""
        return ("Явления" if PHENOMENON.search(d) else "Основные термины"), ""
    return None


CN_IN_EXTRACT = re.compile(r"Chinese:\s*([^;,)\s]+)")
for r in rows:
    ours, typ = guess(r)
    page = cache.get(wiki_title(r), {"missing": True, "categories": []})
    if page.get("missing") and r["meaning"] == "раса":
        page = cache.get(r["raw"], page)
    found = not page.get("missing", True)
    picked = by_wiki(page, ours) if found else None
    if picked:
        r["cat"], r["type"] = picked[0], picked[1] or typ
    else:
        r["cat"], r["type"] = ours, typ
    if r["cat"] != "Имена":
        r["type"] = ""
    r["guessed"] = picked is None
    r["wiki_cats"] = ", ".join(page.get("categories", [])) if found else "не найдено"
    r["wiki_title"] = page.get("title", "") if found else ""
    r["wiki_url"] = page.get("url", "") if found else ""
    if not r["cn"] and found:
        m = CN_IN_EXTRACT.search(page.get("extract", ""))
        if m:
            r["cn"] = m.group(1)

# ---------- workbook ----------
HEAD = ["EN", "Категория", "Тип (для имён)", "Как сейчас в игре (самый частый)", "Все варианты в игре (вариант — число)", "Сова", "CN", "Сколько раз", "Пример EN", "Пример RU", "Откуда", "Категория на вики", "Как на вики", "Ссылка", "Уже в глоссарии?", "Согласованность с глоссарием", "Как называть", "Комментарий"]
WIDTH = [28, 22, 12, 26, 44, 30, 12, 10, 60, 60, 44, 30, 24, 40, 30, 18, 24, 30]
MIXED_FILL = PatternFill("solid", fgColor="FFF2CC")
EMPTY_FILL = PatternFill("solid", fgColor="E2EFDA")
wb = Workbook()
wb.remove(wb.active)
summary = {}
for cat in CATS:
    ws = wb.create_sheet(cat)
    ws.append(HEAD)
    for i, w in enumerate(WIDTH, 1):
        ws.column_dimensions[get_column_letter(i)].width = w
    for cell in ws[1]:
        cell.font = Font(bold=True)
        cell.alignment = Alignment(wrap_text=True, vertical="top")
    ws.freeze_panes = "B2"
    rs = sorted((r for r in rows if r["cat"] == cat), key=lambda r: (-r["count"], r["en"]))
    for r in rs:
        catcell = r["cat"] + (" (предположительно)" if r["guessed"] else "")
        ws.append([r["en"], catcell, r["type"], r["top"], r["variants"], r["sova"], r["cn"], r["count"], r["ex_en"], r["ex_ru"], r["src"], r["wiki_cats"], r["wiki_title"], r["wiki_url"], r["gl"], r["cons"], "", ""])
        row = ws.max_row
        for col in range(1, len(HEAD) + 1):
            ws.cell(row, col).alignment = Alignment(wrap_text=True, vertical="top")
        if r["wiki_url"]:
            ws.cell(row, 14).hyperlink = r["wiki_url"]
            ws.cell(row, 14).font = Font(color="0563C1", underline="single")
        if r["mixed"]:
            for col in range(1, 17):
                if col != 14:
                    ws.cell(row, col).fill = MIXED_FILL
        ws.cell(row, 17).fill = EMPTY_FILL
        ws.cell(row, 18).fill = EMPTY_FILL
    ws.auto_filter.ref = f"A1:{get_column_letter(len(HEAD))}{max(ws.max_row, 1)}"
    summary[cat] = {"rows": len(rs), "mixed": sum(r["mixed"] for r in rs), "on_wiki": sum(not r["guessed"] for r in rs)}
wb.save(OUT)

top_mixed = sorted((r for r in rows if r["mixed"]), key=lambda r: -r["count"])[:10]
print(json.dumps({"cats": summary, "total": len(rows), "mixed": sum(r["mixed"] for r in rows), "on_wiki": sum(not r["guessed"] for r in rows),
                  "top_mixed": [(r["en"], r["cat"], r["count"], r["variants"]) for r in top_mixed],
                  "check": [(r["en"], r["cat"], r["type"], r["wiki_title"], r["wiki_cats"]) for r in rows if r["raw"] in ("Rhodes Island", "Ursus", "Chernobog", "Lungmen", "Oripathy", "Catastrophe", "Originium", "Originium Slug", "Amiya", "Wei Yenwu", "Infected")]},
                 ensure_ascii=False, indent=1))
