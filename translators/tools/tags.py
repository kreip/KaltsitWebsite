#!/usr/bin/env python3
"""The game's markup tags as a list: what each is called, its colour or the
game's explanation, and how many texts use it.

    python3 translators/tools/tags.py [backend-url] > translators/out/tags.md

Reads `GET /api/translators/tags` (rebuilt by the import) with the internal
service key of the running backend container, and counts the substitutions
(`{…}`) and Unity's tags (`<i>`, `<color=…>`) with the section's search.
"""
import json
import subprocess
import sys
import urllib.parse
import urllib.request

BASE = (sys.argv[1] if len(sys.argv) > 1 else "http://localhost:3060") + "/api/translators"
KEY = subprocess.run(
    ["docker", "compose", "exec", "-T", "backend", "printenv", "SERVICE_KEY"],
    capture_output=True, text=True, check=True,
).stdout.strip()

# What a style marks, by the last part of its name (as the section's `tags.ts`).
STYLES = {
    "vup": "повышение (число растёт)", "vdown": "понижение (число падает)", "rem": "примечание",
    "acrem": "примечание", "kw": "ключевое слово", "key": "ключевое слово", "imp": "ключевое слово",
    "talpu": "прибавка потенциала", "item": "объект этапа", "muitem": "объект этапа",
    "fs": "условие этапа", "ez": "условие этапа", "get": "получено", "lose": "потеряно",
}


def get(path, **params):
    url = f"{BASE}/{path}"
    if params:
        url += "?" + urllib.parse.urlencode(params)
    req = urllib.request.Request(url, headers={"x-service-key": KEY})
    with urllib.request.urlopen(req, timeout=60) as res:
        return json.load(res)


def uses(text):
    return get("texts", q=text, limit=1)["total"]


tags = get("tags")
styles = [t for t in tags if t["kind"] == "style"]
terms = [t for t in tags if t["kind"] == "term"]
print("# Теги текстов игры\n")
print(f"Собрано импортом: {len(styles)} цветовых тегов, {len(terms)} терминов. «Текстов» — сколько английских текстов игры содержит тег.\n")

print("## Термины `<$…>текст</>` — в игре подчёркнуты, по нажатию показывают пояснение\n")
print("| тег | название (EN) | название (RU, пак) | пояснение (RU, иначе EN) | текстов |")
print("|---|---|---|---|---|")
for t in terms:
    about = (t["descriptionRu"] or t["descriptionEn"] or "").replace("\n", " ").replace("|", "/")
    print(f"| `<{t['tag']}>` | {t['nameEn'] or '—'} | {t['nameRu'] or '—'} | {about[:120]} | {t['uses']} |")

print("\n## Цветовые теги `<@…>текст</>`\n")
print("| тег | что отмечает | цвет в игре | текстов |")
print("|---|---|---|---|")
for t in styles:
    what = STYLES.get(t["tag"].rsplit(".", 1)[-1], "выделение (своего названия нет)")
    print(f"| `<{t['tag']}>` | {what} | {t['color'] or '—'} | {t['uses']} |")

print("\n## Теги Unity\n")
print("| тег | что делает | текстов |")
print("|---|---|---|")
for tag, what in [("<i>", "курсив"), ("<b>", "жирный"), ("<u>", "подчёркнутый"), ("<color=", "цвет (значение — в теге)"), ("<size=", "размер шрифта"), ("<p=", "абзац в сюжете (закрывается `</>`)")]:
    shown = tag if tag.endswith(">") else tag + "…>"
    print(f"| `{shown}` | {what} | {uses(tag)} |")

print("\n## Подстановки `{…}` — игра подставляет значение\n")
print("| подстановка | что это | текстов |")
print("|---|---|---|")
for token, what in [("{@nickname}", "ник Доктора (в сюжете)"), ("{@nbs}", "неразрывный пробел (в сюжете)"), ("{0}", "первое значение по порядку"), ("{1}", "второе значение"), ("{2}", "третье значение"), (":0%}", "число в процентах (`{atk:0%}`)"), (":0.0}", "число с одним знаком после запятой"), (":0}", "целое число"), ("{-", "число с обратным знаком (`{-cost}`)")]:
    print(f"| `{token}` | {what} | {uses(token)} |")
print("\nИмена в подстановках навыков (`{atk}`, `{duration}`, `{attack@atk_scale}`…) — названия величин из данных навыка; их сотни, переводить и менять их нельзя.")
