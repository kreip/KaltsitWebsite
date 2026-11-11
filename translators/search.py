#!/usr/bin/env python3
"""Search every text of the game from the command line (the section's search, `GET /api/translators/texts`).

    python3 translators/search.py <text> [--section S] [--status S] [--whole] [--limit N] [--url http://localhost:3060]

    --section  story | operators | enemies | stages | items | archives | other
    --status   unchecked | proposed | approved | untranslated | plugin
    --whole    whole words only

Prints how many texts match, how long the database took, and the first results:
English, Russian and where the text stands. The internal service key is read
from the running backend container, so it is never written here.
"""

import argparse
import json
import pathlib
import subprocess
import sys
import time
import urllib.parse
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent


def where(at: dict) -> str:
    if at.get("storyId"):
        return f"сюжет › {at['storyId']} › реплика {at['ord'] + 1}"
    idx = f" {at['idx']}" if at.get("idx") else ""
    return f"{at['section']} › {at.get('objectEn')} › {at.get('field')}{idx}"


def short(text: str | None, width: int = 90) -> str:
    if text is None:
        return "—"
    text = " ".join(text.split())
    return text if len(text) <= width else text[: width - 1] + "…"


def main() -> int:
    ap = argparse.ArgumentParser(description="Search every text of the game.")
    ap.add_argument("text", nargs="?", default="")
    ap.add_argument("--section")
    ap.add_argument("--status")
    ap.add_argument("--whole", action="store_true")
    ap.add_argument("--limit", type=int, default=10)
    ap.add_argument("--url", default="http://localhost:3060")
    args = ap.parse_args()

    key = subprocess.run(
        ["docker", "compose", "exec", "-T", "backend", "printenv", "SERVICE_KEY"],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout.strip()
    params = {"q": args.text, "limit": args.limit}
    if args.section:
        params["section"] = args.section
    if args.status:
        params["status"] = args.status
    if args.whole:
        params["whole"] = "true"
    req = urllib.request.Request(
        f"{args.url.rstrip('/')}/api/translators/texts?{urllib.parse.urlencode(params)}",
        headers={"x-service-key": key},
    )
    started = time.monotonic()
    with urllib.request.urlopen(req, timeout=60) as res:
        body = json.loads(res.read())
    took = (time.monotonic() - started) * 1000
    print(f"найдено: {body['total']} из {body['indexed']} · база: {body['millis']} мс · весь запрос: {took:.0f} мс")
    for hit in body["rows"]:
        marks = [", ".join(hit["sections"]), hit["status"], hit.get("file") or "нет в словарях"]
        if hit.get("pluginRole"):
            marks.append(f"плагин: {hit['pluginRole']}")
        print(f"\n  EN  {short(hit['en'])}")
        print(f"  RU  {short(hit['ru'])}")
        print(f"      {' · '.join(marks)} · мест: {hit['places']}")
        for at in hit["at"]:
            print(f"      ↳ {where(at)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
