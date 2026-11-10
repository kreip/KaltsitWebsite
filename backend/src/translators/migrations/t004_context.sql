-- Review of part 1: proposals for one place only,
-- comments on a fragment, the glossary's computed columns, character names
-- for the Service rows.

-- A proposal for this place only (story, line, part): shown there, never in
-- the dictionary export; listed for a context translation of the plugin.
ALTER TABLE translators.proposals ADD COLUMN IF NOT EXISTS context BOOLEAN NOT NULL DEFAULT FALSE;
DROP INDEX IF EXISTS translators.proposals_one_per_author;
CREATE UNIQUE INDEX IF NOT EXISTS proposals_one_per_author_scope
    ON translators.proposals (edit_key, author_id,
        (CASE WHEN context THEN story_id || ':' || line || ':' || part ELSE '' END))
    WHERE deleted_at IS NULL;

-- The fragment of the line a comment is about, as selected.
ALTER TABLE translators.comments ADD COLUMN IF NOT EXISTS quote TEXT;

-- Computed over the story after each download and import (the sheet is never changed):
-- the Russian the game uses for the term (`[{"ru": "Урсус", "lines": 12, "glossary": false}]`),
-- how many of its lines have a Russian to check and how many hold the glossary's form,
-- and its Chinese from the game's speakers.
ALTER TABLE translators.glossary_terms ADD COLUMN IF NOT EXISTS game_variants JSONB NOT NULL DEFAULT '[]';
ALTER TABLE translators.glossary_terms ADD COLUMN IF NOT EXISTS checked_lines INTEGER NOT NULL DEFAULT 0;
ALTER TABLE translators.glossary_terms ADD COLUMN IF NOT EXISTS found_lines INTEGER NOT NULL DEFAULT 0;
ALTER TABLE translators.glossary_terms ADD COLUMN IF NOT EXISTS cn_game TEXT;

-- Who a story sprite (`char_002_amiya`) is: the speaker who talks most often
-- right after it comes on stage. Rebuilt by the import.
CREATE TABLE IF NOT EXISTS translators.char_names (
    char_id TEXT PRIMARY KEY,
    name_en TEXT NOT NULL,
    name_ru TEXT,
    votes INTEGER NOT NULL,
    total INTEGER NOT NULL
);

-- Two kinds of comment: a remark (a question or a problem; "resolved" folds
-- it away) and a translator's note (why it is translated so, a reference, a
-- fact; never resolved, always shown, also after approval and when outdated).
-- A note may be about one variant: `variant` = `game` | `sova` | `ai` |
-- `proposal` (then `proposal_id`), with the variant's text as it was. Nothing
-- ever deletes a comment or note on approval, a change of variant or outdating.
ALTER TABLE translators.comments ADD COLUMN IF NOT EXISTS kind TEXT NOT NULL DEFAULT 'remark'
    CHECK (kind IN ('remark', 'note'));
ALTER TABLE translators.comments ADD COLUMN IF NOT EXISTS variant TEXT
    CHECK (variant IN ('game', 'sova', 'ai', 'proposal'));
ALTER TABLE translators.comments ADD COLUMN IF NOT EXISTS proposal_id BIGINT
    REFERENCES translators.proposals (id) ON DELETE SET NULL;
ALTER TABLE translators.comments ADD COLUMN IF NOT EXISTS variant_text TEXT;

-- Sova's footnotes to a chapter (arktr `stage_*.json` `footnotes`), imported
-- as notes by "Sova" for every story of the chapter; members only, like her
-- texts. Rebuilt by the import.
CREATE TABLE IF NOT EXISTS translators.sova_notes (
    story_id TEXT NOT NULL,
    idx INTEGER NOT NULL,
    text TEXT NOT NULL,
    PRIMARY KEY (story_id, idx)
);
