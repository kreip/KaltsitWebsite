-- Every text of the game in one list, rebuilt by the import: a row per
-- entry (the English an edit is keyed by) with the Russian the game shows,
-- where it is kept, and the sections it belongs to. The search of the
-- section's "Game sections" and "Term" tabs reads it.
CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE IF NOT EXISTS translators.texts (
    id BIGSERIAL PRIMARY KEY,
    server TEXT NOT NULL,
    -- The key of an edit: the dictionary entry's English, else the text without tags.
    edit_key TEXT NOT NULL,
    -- The English as the game has it (tags kept).
    en TEXT NOT NULL,
    -- The Russian the game shows now.
    ru TEXT,
    -- The dictionary file holding it (`11_ui.txt`, ...), or `plugin:<role>`.
    file TEXT,
    -- The role of the RichTextFix plugin's catalog that also holds it, and
    -- the plugin's Russian when it differs from the dictionary's.
    plugin_role TEXT,
    ru_plugin TEXT,
    -- `story`, `operators`, `enemies`, `stages`, `items`, `archives`, `other`.
    sections TEXT[] NOT NULL,
    -- Story lines and game data places holding it.
    places INTEGER NOT NULL,
    -- What the search looks in: English and Russian, without tags, lower case, ё as е.
    fold TEXT NOT NULL
);

-- An entry's English may be pages long (an operator's file), too long for a
-- b-tree: one row per entry is kept by its hash, and it is found by a hash index.
CREATE UNIQUE INDEX IF NOT EXISTS texts_entry ON translators.texts (server, md5(edit_key));
CREATE INDEX IF NOT EXISTS texts_key ON translators.texts USING hash (edit_key);
CREATE INDEX IF NOT EXISTS texts_fold_trgm ON translators.texts USING gin (fold gin_trgm_ops);
CREATE INDEX IF NOT EXISTS texts_sections ON translators.texts USING gin (sections);

-- Where the game data shows a text: the object (an operator, an enemy, a
-- stage, an item; a table for `other`) and what of it the text is. The story
-- places are `story_lines`.
CREATE TABLE IF NOT EXISTS translators.text_places (
    server TEXT NOT NULL,
    edit_key TEXT NOT NULL,
    section TEXT NOT NULL,
    object_id TEXT NOT NULL,
    object_en TEXT NOT NULL,
    -- `name`, `trait`, `talent.description`, `skill.name`, `module.effect`, ...
    field TEXT NOT NULL,
    -- Which one of several (skill 2, talent 1), from 1; 0 when there is one.
    idx SMALLINT NOT NULL,
    -- The order inside the object, as the game data lists it.
    ord INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS text_places_key ON translators.text_places USING hash (edit_key);
CREATE INDEX IF NOT EXISTS text_places_object ON translators.text_places (server, section, object_id, ord);
