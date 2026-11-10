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
3. Import (re-runnable; replaces the section's rows, reads the sources only):
   `./translators/import.sh`
4. Open http://localhost:3000/translators

## Development sign-in

Off unless the backend has `TRANSLATORS_DEV_LOGIN=1`. When on, `/translators/dev-login`
signs in as a local account (`dev-translator`) that is a member of the section. The
regular game-account sign-in is not touched.

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

## Editing

Proposals, "keep as in the game" and comments are keyed by the English of the
dictionary entry the import found for a line (`story_lines.edit_key`): one decision
holds wherever the line stands; a proposal can also be made for one story line only.
Storage test against a throwaway database:
`cargo test --test translators_edits_test -- --ignored`.
