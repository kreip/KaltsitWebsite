-- A text with no letter (dots, dashes, numbers: «.....», «...?», «—», «3») has
-- nothing to translate: it counts as checked by the system — in the progress,
-- in the search's states — and never waits in a queue. The same rule as
-- `edits::status::editable`: tags, substitutions `{…}` and escapes `\n` aside.
CREATE OR REPLACE FUNCTION translators.lettered(key TEXT) RETURNS BOOLEAN
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    AS $$ SELECT regexp_replace(key, '<[^>]*>|\{[^}]*\}|\\[nrt]', '', 'g') ~ '[[:alpha:]]' $$;

ALTER TABLE translators.texts
    ADD COLUMN IF NOT EXISTS lettered BOOLEAN GENERATED ALWAYS AS (translators.lettered(edit_key)) STORED;
ALTER TABLE translators.story_lines
    ADD COLUMN IF NOT EXISTS lettered BOOLEAN GENERATED ALWAYS AS (translators.lettered(edit_key)) STORED;
