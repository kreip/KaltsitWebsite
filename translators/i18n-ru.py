#!/usr/bin/env python3
"""Load the translators' section's Russian into the site's i18n (locale `ru`).

    ./translators/i18n-ru.py [--enable] [backend-url]

1. Syncs the frontend's source catalog to the backend (what `bun run
   i18n:sync` does), so every `translators.*` key exists.
2. Creates the `ru` locale if it is missing: switched OFF unless `--enable`
   (enabling a locale shows it in the site's language switcher for everyone,
   which is the site owner's call).
3. Writes every text of `frontend/src/components/translators/translators.ru.json`
   as the `ru` translation of its `translators.<key>`, straight into
   `ui_messages` with the key's current source hash, the way the admin panel
   stores a translation (the backend's message route needs a signed-in admin,
   which a script does not have). The backend's catalog cache picks them up
   within a minute.

4. Writes the texts of `translators/site-ru.json` (the site's header, menus,
   sign-in dialog and shared buttons: what stands around the section) the
   same way, but only where the key has no `ru` translation yet, so nothing
   translated in the admin panel is ever overwritten.

The section shows Russian without this (it ships the same file); this puts
the texts where the admin panel edits them, and an edit there wins over the
file when the site runs in `ru`. The internal service key is read from the
running backend container, so it is never written here.
"""

import json
import pathlib
import re
import subprocess
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
RU = ROOT / "frontend/src/components/translators/translators.ru.json"
SITE_RU = ROOT / "translators/site-ru.json"
META = ROOT / "frontend/src/lib/i18n/source-catalog.meta.json"
SOURCE = ROOT / "frontend/src/lib/i18n/source-catalog.json"


def arguments(text: str) -> list[str]:
    """The arguments of an ICU message: `{name}` and `{name, plural, …}`, in the text and inside the branches,
    not the branches' own words (`dark {Dark}`)."""
    found: set[str] = set()

    def closing(i: int) -> int:
        """The index of the brace that closes the one at `i`."""
        depth = 0
        for j in range(i, len(text)):
            depth += text[j] == "{"
            depth -= text[j] == "}"
            if depth == 0:
                return j
        return len(text) - 1

    def message(start: int, end: int) -> None:
        i = start
        while i < end:
            if text[i] != "{":
                i += 1
                continue
            close = closing(i)
            m = re.match(r"\s*(\w+)\s*(,\s*\w+\s*,)?", text[i + 1:close])
            if m:
                found.add(m.group(1))
                if m.group(2):  # plural / select: every `{…}` after the type is a branch, itself a message
                    j = i + 1 + m.end()
                    while j < close:
                        if text[j] == "{":
                            k = closing(j)
                            message(j + 1, k)
                            j = k
                        j += 1
            i = close + 1

    message(0, len(text))
    return sorted(found)


def site_texts() -> dict[str, str]:
    """The site's own texts in Russian: known keys only, each keeping its arguments."""
    source = json.loads(SOURCE.read_text(encoding="utf-8"))
    texts = json.loads(SITE_RU.read_text(encoding="utf-8"))
    for key, value in texts.items():
        if key not in source:
            raise SystemExit(f"site-ru.json: no such key: {key}")
        if arguments(value) != arguments(source[key]):
            raise SystemExit(f"site-ru.json: {key}: the arguments differ from the English")
    return texts


def main() -> int:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    enable = "--enable" in sys.argv
    base = (args[0] if args else "http://localhost:3060").rstrip("/") + "/api"
    key = subprocess.run(
        ["docker", "compose", "exec", "-T", "backend", "printenv", "SERVICE_KEY"],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout.strip()

    def call(method: str, path: str, body=None):
        req = urllib.request.Request(
            base + path,
            method=method,
            # Compact: the whole catalog goes in one request, close to the backend's 2 MiB body limit.
            data=None if body is None else json.dumps(body, separators=(",", ":"), ensure_ascii=False).encode(),
            headers={"x-service-key": key, "content-type": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=60) as res:
            raw = res.read()
            return json.loads(raw) if raw else None

    entries = json.loads(META.read_text(encoding="utf-8"))["entries"]
    synced = call("POST", "/admin/i18n/sync", {"entries": entries})
    print(f"keys synced: {json.dumps(synced, ensure_ascii=False)[:200]}")

    locales = call("GET", "/admin/i18n/locales") or []
    codes = {loc.get("code"): loc for loc in (locales if isinstance(locales, list) else locales.get("locales", []))}
    if "ru" not in codes or (enable and not codes["ru"].get("enabled")):
        call("PUT", "/admin/i18n/locales", {
            "code": "ru", "english_name": "Russian", "native_name": "Русский",
            "fallback_locale": "en", "gamedata_server": "en",
            "enabled": enable or bool(codes.get("ru", {}).get("enabled")),
            "sort_order": 10,
        })
        print(f"locale ru: {'enabled' if enable else 'created, switched off'}")

    texts = json.loads(RU.read_text(encoding="utf-8"))
    q = lambda v: "'" + v.replace("'", "''") + "'"  # noqa: E731
    rows = ",\n".join(f"({q('translators.' + k)}, {q(v)})" for k, v in texts.items())
    sql = f"""
INSERT INTO ui_messages (key, locale, value, source_hash, source_text, updated_by, updated_at)
SELECT k.key, 'ru', t.value, k.source_hash, k.source_text, NULL, now()
FROM (VALUES {rows}) AS t(key, value) JOIN ui_message_keys k ON k.key = t.key
ON CONFLICT (key, locale) DO UPDATE SET value = EXCLUDED.value, source_hash = EXCLUDED.source_hash,
    source_text = EXCLUDED.source_text, updated_at = now();
"""
    out = subprocess.run(
        ["docker", "compose", "exec", "-T", "postgres", "sh", "-c",
         'psql -v ON_ERROR_STOP=1 -q -U "$POSTGRES_USER" -d "$POSTGRES_DB"'],
        cwd=ROOT, input=sql, check=True, capture_output=True, text=True,
    )
    print(out.stdout.strip() or f"ru texts written: {len(texts)}")

    site = site_texts()
    rows = ",\n".join(f"({q(k)}, {q(v)})" for k, v in site.items())
    sql = f"""
WITH added AS (
    INSERT INTO ui_messages (key, locale, value, source_hash, source_text, updated_by, updated_at)
    SELECT k.key, 'ru', t.value, k.source_hash, k.source_text, NULL, now()
    FROM (VALUES {rows}) AS t(key, value) JOIN ui_message_keys k ON k.key = t.key
    ON CONFLICT (key, locale) DO NOTHING
    RETURNING 1
) SELECT count(*) FROM added;
"""
    out = subprocess.run(
        ["docker", "compose", "exec", "-T", "postgres", "sh", "-c",
         'psql -v ON_ERROR_STOP=1 -qAt -U "$POSTGRES_USER" -d "$POSTGRES_DB"'],
        cwd=ROOT, input=sql, check=True, capture_output=True, text=True,
    )
    print(f"site texts (header, menus, shared buttons): {len(site)} in the file, {out.stdout.strip()} new")
    return 0


if __name__ == "__main__":
    sys.exit(main())
