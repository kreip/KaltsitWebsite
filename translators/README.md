# Для переводчиков (translators' section)

A read-only EN | RU | CN reader of the story text at `/translators`, for the
ArknightsRU translation team. The Russian column is what the game shows now
with the ArknightsRU pack (XUnity dictionaries `11_ui.txt`, `12_story.txt`,
`13_names.txt` and the RichTextFix plugin's built-in catalog). Sova's and the
AI's variants (from arktr) are shown to signed-in translators only.

Code: `backend/src/translators/`, `frontend/src/components/translators/`,
`frontend/src/lib/translators/`, `frontend/src/routes/translators*.tsx`.
Tables: Postgres schema `translators` (own migrations, `translators._migrations`).

## Local run

1. Machine-local files (not committed):

   `docker-compose.override.yml` in the repo root:

   ```yaml
   services:
       backend:
           environment:
               SERVERS: "en"
               # Development sign-in: remove the line to switch it off.
               TRANSLATORS_DEV_LOGIN: "1"
           volumes:
               - /path/to/BepInEx/Translation/ru/Text:/data/translators/ru-text:ro
               - /path/to/BepInEx/plugins/ArknightsLocalization:/data/translators/plugin:ro
               - /path/to/arktr:/data/translators/arktr:ro
   ```

   `.env` in the repo root (so plain `docker compose` picks all three files):

   ```
   COMPOSE_FILE=docker-compose.yml:translators/compose.yml:docker-compose.override.yml
   ```

2. Build and start: `docker compose up -d --build`
3. Import (re-runnable, about 3 minutes; replaces the section's rows, reads the sources only):
   `./translators/import.sh`
4. Open http://localhost:3000/translators

## Development sign-in

Off unless the backend has `TRANSLATORS_DEV_LOGIN=1`. When on, `/translators/dev-login`
signs in as a local account (`dev-translator`) that is a member of the section. The
regular game-account sign-in is not touched.

## Russian

The section speaks the site's language. `python3 translators/i18n-ru.py --enable`
loads the section's Russian (`frontend/src/components/translators/translators.ru.json`)
into the site's translations and switches the `ru` locale on: the header's language
switch then offers Русский, and the pages live under `/ru/…`. It also loads
`translators/site-ru.json` (the header, menus, sign-in dialog and shared buttons: what
stands around the section), only where a key has no Russian yet, so a translation made
in the admin panel is never overwritten. The site's other texts stay English until they
are translated there.

## Members and roles

Only members (`translators.members`) see the section. To add someone or set their
roles (they must have signed in to the site once; the command replaces the roles):

    ./translators/role.sh <uid> [editor] [maintainer] [admin]

- every member is a translator: proposes, agrees, comments;
- `editor` is the lead: the one person who approves lines and exports the approved
  lines for the game;
- `maintainer` is the pack maintainer: exports the approved lines for the game and
  marks them as released in a pack version; cannot approve;
- `admin` can do everything.

One person can hold several roles (`./translators/role.sh <uid> editor maintainer`).

## Every text in one list

The import also builds `translators.texts`: one row per entry (the English an edit is
keyed by) for the story, the game data by section (operators, enemies, stages, items,
archives) and every other entry of the pack's dictionaries and the plugin's catalog,
with the game data places of each (`translators.text_places`: operator › skill 2 ›
description). `GET /api/translators/texts` searches it by a part of the English or the
Russian (any case, ё = е), with filters by section and state. From the command line:

    python3 translators/search.py ursus
    python3 translators/search.py урсус --section operators --limit 5
    python3 translators/search.py sp --whole --status unchecked

It prints how many texts match, the database time and the first results.

The page's **Search** tab (`/translators?tab=search&q=…`) is the same search with the
story table's actions in the results (`GET /api/translators/search`). A text that stands
in the story is edited at its first story line; any other as the text itself — the
place `storyId = "@text"` with the entry's English in `editKey` — one translation
everywhere. The query and the filters are in the address, so a search is a link.

## Export

"My edits" → "Export…" prepares one archive (`.zip`) for the pack's maintainer:

- corrected copies of `11_ui.txt` / `12_story.txt` / `13_names.txt`, only the changed
  ones: in a touched line only the value changes (a `//` comment after it is dropped),
  everything else stays byte for byte;
- `14_new.txt`: entries none of the pack's files has (the pack's own `14_new.txt`, once
  it has one, with the new entries at its end);
- `plugin_strings.tsv`: lines the RichTextFix plugin shows from its built-in catalog
  (any role but `ambiguous:…`: story, skill, operator…), with the role;
- `story_lines.tsv`: every approved story line with its place (for the bundles);
- `report.txt`: each changed line (file, line number, before → after, who approved).

Two modes: approved (for the game; the lead and the maintainer) and every proposal
for review (`REVIEW_` in the names; any member). A second format writes one
`00_translators.txt` to check at home: put it into `BepInEx\Translation\ru\Text`
and press Alt+R in the game.

## Editing

Proposals, "keep as in the game" and comments are keyed by the English of the
dictionary entry the import found for a line (`story_lines.edit_key`): one decision
holds wherever the line stands; a proposal can also be made for one story line only.
Storage test against a throwaway database:
`cargo test --test translators_edits_test -- --ignored`.
