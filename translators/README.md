# Для переводчиков (translators' section)

The ArknightsRU translation team's workplace at `/translators`: the story and the
game's other texts as EN | RU | CN, proposals, the translators' approvals and the export for the pack. The Russian column is what the game shows now
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

Only members (`translators.members`) see the section. One role per member:

- `user` — sees the section, proposes, agrees, remarks;
- `translator` — also approves and rejects proposals, exports the approved lines for the game;
- `admin` — all of it, marks the approved lines as released in a pack version, and the
  **Admin** tab: requests for access, the members with their roles, the statistics.

A signed-in user of the site who is not a member sees "Request access"; an admin accepts the
request with a role (or declines it) on the Admin tab, changes a member's role or removes a
member there (never their own: the section keeps an admin). From the command line (the user
must have signed in to the site once; the command replaces the role):

    ./translators/role.sh <uid> [user|translator|admin]

Migration t018 turned the old roles into these: `maintainer` (or `admin`) → `admin`, `editor` →
`translator`, every other member → `user`.

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

In the results a place of the game data is named in words (`Interface and other › Medals`,
never the table's name: that shows on hover); "N more" unfolds every place of the text.

## Game sections

The **Game sections** tab (`/translators?tab=sections&sec=…&obj=…`) is the Story tab for the
texts outside the story: a section (operators, enemies, stages, items, archives) and one of its
objects for a chapter and a story; the object's texts as the same table, `Field | Russian as
in the game | English`, in the order the game shows them, with the same actions. A text is
edited as the text itself (`storyId = "@text"`): one translation wherever its English
stands. `GET /api/translators/objects?section=` lists a section's objects with their
progress, `GET /api/translators/objects/lines?section=&object=` gives one object's texts.
An operator is one record of the game data (its id); elsewhere several records under one
name (a stage and its challenge mode) are one object, named by the English name.
«Interface and other» has no objects: it is the search over that section.

The tab's last section, **Archives**, is an operator at a time: the file's parts, the
operator records (name and introduction, with a link to the record's story on the Story
tab), the modules' and outfits' stories, the voice lines by the game's order. A text longer
than 600 characters or 8 lines is folded to a few lines until clicked (in the search too).

Where several texts of a card share a field («Talent 1 · description» at each elite phase),
each says what tells it apart: the import notes it per place (`text_places.note`: `E2+P6`
elite 2 with potential 6, `S1,…,S7` skill levels, `M3` mastery, `T2` a module's stage, `C` a
stage's challenge mode, `#…` a code of the game) and the card words it. Records of one name
with the very same texts are one object in the picker; different ones show their first text
the others lack.

## Search order and selected text

The search answers best first: the query as the whole English or Russian, then as whole
words, then at the start, then anywhere; among equals the game's names and terms before the
story's lines, then the shorter text. A text pages long (over ~1,900 bytes) is only ever
"holds the query": Postgres keeps it compressed apart from its row and unpacking it for
every test of the sort key tripled the time. Above the texts: the operators, enemies, items
and stages whose name answers the query, each with a link to its card.

A word or phrase selected in any table (a double click, a drag) opens a small menu: find it
everywhere (the Search tab), find it in the glossary, or suggest it for the glossary. A
suggestion (`translators.glossary_suggestions`; English, Russian, note, the line it was
noticed in) shows in a list of its own on the Glossary tab, where a translator accepts or
declines it. The team's sheet is never written from the site.

## The glossary and an approved line

An approved line (its entry, or a one-place proposal at it) counts as checked for every
glossary term the sheet has not changed since the approval: no cross under the line, a
gray "approved" check instead, counted as holding the form in the Glossary tab's
consistency, and lending no "as in the game" variant. The import keeps when each term's
Russian last changed (`glossary_terms.changed_at`, carried over between downloads by the
row's English); a term changed after the approval is checked again and says so. The
Glossary tab recounts in the background after every approval.

A term of several words shows the game's whole phrase in "As in the game", the words that
are not the glossary's marked. Beside a variant with a cross, "Replace with the glossary's
form" (`POST /api/translators/glossary/{id}/fix`) lists the lines, each with the text it
would get — the glossary's stem plus the game word's ending, only when the two are the same
word spelled differently; otherwise "check by hand" — and writes one proposal per entry.
A failed download is kept as a code (`offline`, `forbidden`, `changed:…`) and worded on the
page.

## The game's tags

One parse of the markup (`frontend/src/components/translators/markup.ts`) serves the tables,
the editor and the checks. Spans: a game style `<@ba.vup>…</>`, a game term `<$ba.stun>…</>`,
a story paragraph `<p=1>…</>` (all closed by `</>`), Unity's `<i>`, `<b>`, `<u>`,
`<color=…>`, `<size=…>`; a substitution `{@nickname}`, `{0}`, `{atk:0%}` is a token; anything
else in angle brackets is text (`<Roadblock>`). The import lists every style and term with
the game's colour or the term's name and explanation, in the pack's Russian too
(`translators.tags`, `GET /api/translators/tags`);
`python3 translators/tools/tags.py > translators/out/tags.md` prints the list.

A table never shows a tag spelled out: a style in its ink, a term as plain text (the
game shows it so) with the game's explanation on hover. In the editor a substitution is a chip (removed whole, inserted
with its button) and a tag a span typed into; the buttons under the box are the
substitutions and tags of the line's English: a tag's button wraps the selection, and with
the cursor inside a span "Remove the tag" drops both halves at once. A warning names the
original's tags a translation lacks or adds; the same test marks every variant.

"No highlight" is honest about the RichTextFix plugin (`markup.ts::lostHighlights`,
`texts/problems.rs::lost_highlights`; the plugin's `RichTextStyleRestorer`, read by
decompiling): into a plain translation the plugin puts back a style that covers the whole
text, a style whose text stands in the translation as it is (a number, `SP`), and a
style's colour on its numbers (a substitution counts as one) found as many times; it never
puts back words, a style it does not know (`translators/tools/plugin_styles.py` keeps its
list from the DLL; the names are matched case-sensitively) or anything into a translation
that has tags of its own (whole-text styles aside). A term `<$…>` is never a lost
highlight: the game shows its words plain.

## Skill levels in the dictionary

The pack's dictionaries hold a skill description once per level with the numbers put in
("dealing 230% ATK … for 1.25 seconds"). The import ties such an entry to the skill
descriptions of the same shape (`texts/levels.rs`: tags dropped, every substitution and
number a `#`) and tells the level by the blackboard's values (`{atk_scale:0%}` → `230%`):
`text_places.field = 'skill.level'`, the note the level(s), `of_key` the template's entry.
The operator's card folds them under the skill's description ("N entries with the levels'
numbers"); the search names them «Skill 1 › skill level 4». An entry tied to nothing is
«Interface and other › Not tied» (the file name on hover). On my proposal for a template,
"Apply to the levels" (`POST /api/translators/proposals/{id}/levels`) writes my proposals
for the level entries with each level's numbers put in, the tags dropped where the entry
has none.

## Export

"Edits" → "Export…" prepares one archive (`.zip`) for the pack's maintainer:

- corrected copies of `11_ui.txt` / `12_story.txt` / `13_names.txt`, only the changed
  ones: in a touched line only the value changes (a `//` comment after it is dropped),
  everything else stays byte for byte;
- `14_new.txt`: entries none of the pack's files has (the pack's own `14_new.txt`, once
  it has one, with the new entries at its end);
- `plugin_strings.tsv`: lines the RichTextFix plugin shows from its built-in catalog
  (any role but `ambiguous:…`: story, skill, operator…), with the role;
- `story_lines.tsv`: every approved story line with its place (for the bundles);
- `only_here.tsv`: outside the story the game has one translation per English text, so a
  proposal made for one place of the game data ("a separate translation is needed here") is
  a request to the pack's maintainer: every live one, with its place
  (`Оператор › Amiya [char_002_amiya] › skill.description 2`), the English, the game's
  Russian, the proposal, its author and the translator who approved it, if one did. Never written
  to the dictionaries;
- `report.txt`: each changed line (file, line number, before → after, who approved).

Two modes: approved (for the game; translators and admins) and every proposal
for review (`REVIEW_` in the names; any member). A second format writes one
`00_translators.txt` to check at home: put it into `BepInEx\Translation\ru\Text`
and press Alt+R in the game.

## XUnity dictionary and plugin format

The rules the import and the export follow (checked against the pack's files and the
game's loader).

**Load order.** XUnity.AutoTranslator reads every `*.txt` under
`BepInEx\Translation\ru\Text`, recursively, except `*resizer.txt` and the
`_Substitutions` / `_Preprocessors` / `_Postprocessors` files. The files are sorted by
full path, **descending**, case-insensitive; a file read later overwrites an earlier one
on the same key. So `11_ui.txt` beats `12_story.txt` and `13_names.txt`, and `00_*.txt`
files (`00_translators.txt`, `00_review_*.txt`, `00_arktr_*.txt`) are read last and win.
`14_new.txt` is read first: its keys are in no other file.

**Reading a line.** `key=value` with one unescaped `=`. Escapes: `\=` → `=`, `\\` → `\`,
`\n` → LF, `\r` → CR, `\uXXXX` → that character, `%3D` → `=`; any other `\x` stays as
written. `//` starts a comment — the line is ignored if it comes before the `=`. A line
whose start reads as a directive (`#set`, `#unset`, `#enable`…) is not a translation;
a key starting with `r:` / `sr:` is a regular expression. Empty keys and values are
ignored.

**Writing a line** (the export), so that the reader above gives back the very key and
value: `\` → `\\`, `=` → `\=`, LF → `\n`, CR → `\r`; the second `/` of `//` →
`\u002F` (else a comment would start); `%` before `3D` → `\u0025`; a `#` that would
make the line a directive → `\u0023`. A key starting with `r:` / `sr:`, an empty key or an
empty value cannot be written and is listed in the report. Files: UTF-8 without BOM, LF.

**The plugin's catalog.** `ArknightsLocalization.RichTextFix.dll` embeds a gzip-compressed
JSON object `{ role: { english: russian } }` (`story`, `skill`, `operator`, …,
`ambiguous:<role>` for texts it leaves in English, `id:*`). The import finds it by
scanning the DLL for gzip streams and keeping the one that decodes to that object; the
DLL is only read, never loaded. For a text with a role the plugin looks in its catalog
**before** the dictionaries, so an edit of such a text goes to `plugin_strings.tsv`. The
catalog is keyed by the plain English: tags and Arknights macros removed, literal
`\n` / `\r\n` turned into LF, spaces around LF removed, trimmed.

**Matching a story line to a dictionary entry.** Exactly first; then both sides
normalised: tags removed, every newline a space, `…` → `...`, every quote `"`, every dash
`-`, NBSP a space, spaces collapsed, `ё` → `е`, lower case.

**The glossary** (the team's Google Sheet, read as its xlsx export). Sheets are found by
name, case-insensitive (Sova's by its prefix: Excel cuts names at 31 characters); the
first row is a header; rows without Russian are skipped; cells are trimmed; an empty
sheet is not an error. `A / B` in a cell is several English keys for one Russian; on
Sova's sheet only, `A, B` → `X, Y` pair up when the counts match (else every key gets
the whole Russian); `Amiya (Guard)` is the key `Amiya` with the bracket shown beside it.
In a line, names, classes and races match only with the case as in the glossary, the
"various terms" sheet in any case; a longer term wins over a shorter one. The Russian is
checked by word stem: one ending cut, the stem at least 3 letters, `ё` = `е`, quotes and
`ООО` ignored; a multi-word term word by word in a row; an indeclinable name by prefix.

## Editing

Proposals, "keep as in the game" and comments are keyed by the English of the
dictionary entry the import found for a line (`story_lines.edit_key`): one decision
holds wherever the line stands; a proposal can also be made for one story line only, or
(outside the story) for one place of the game data. An entry may be pages long: the edit
tables find it by a hash index and keep one per md5 of the key, and a long entry's place
names it as `md5:<hex>` so that it fits an address.

The translators whose variants the import brings are listed with their authors' names in
`frontend/src/components/translators/translatorVariants.ts` (Sova is the first): a variant,
a proposal started from it and a comment show the author's name.
Storage test against a throwaway database:
`cargo test --test translators_edits_test -- --ignored`.

## Deciding in the Edits list

On the **Edits** tab a translator (or an admin) decides without opening the line: a click on a
proposal's state offers Approve / Reject (a reason is optional; the author sees it), Withdraw for
an approved one, Back to proposals for a rejected one. Tick boxes (Shift for a range, the header's
for every waiting proposal) and the "Selected: N · Approve · Reject" bar decide many at once
(`POST /api/translators/proposals/decisions`). A rejected proposal leaves the line and the
progress; its author rewriting it makes it a proposal again.

## The line's buttons

Each text carries its own buttons: the game's text (in the panel shown on hover), each proposal,
a translator's and the machine's variant. The main decisions have a word and an icon —
**Approve** (✓) and, on a proposal, **Reject** (✗); on the approved one **Withdraw**. A
translator's or the machine's variant is approved at once. "Approve" on the game's text keeps it
as in the game and asks first when the line has proposals. Propose, remark, note, history and
agree are gray icons after a rule; "?" says what each means. The approved block says what was
approved ("the text as in the game", "the variant by <author>") and marks a newer proposal.

## View

The **View** button next to the filters (Story, Search, Game sections) holds the display
settings: the variants, compare, Chinese and the service rows (Story), and **Only as in the game**
(`game=1` in the address): the game's text and the English, nothing else. A translator's variant
equal to the game's text is not shown.

## Problems

A line whose game text has a problem is yellow (an approved one never is); the status filter
names them with counts: a glossary form not found, a broken tag (left open or broken, a stray
closer, a substitution `{…}` lost or added, a tag the English lacks), no highlight (the English
has a tag the Russian lacks, the text whole), line breaks, a proposal noticeably longer than the
game's text, ты / вы. The page works them out (`checks.ts`, `markup.ts::tagHealth`); for the
search the backend keeps them per text (`texts.problems`, migration t019, `texts/problems.rs`,
redone after every import and glossary download; `status=problems|glossary|brokenTag|noHighlight|
newlines|longer`). `GET /api/translators/texts/summary` gives the overview above Search and Game
sections (texts, with Russian, checked) and each problem's count.
