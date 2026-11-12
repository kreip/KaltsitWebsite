-- "A separate translation is needed here", outside the story: the game shows
-- one translation wherever an English text stands, so a proposal for one
-- place of the game data (this operator's skill, this item's description) is
-- a request to the pack's maintainer, who alone can make the game tell the
-- places apart. It is a proposal for one place (`context`) whose place is a
-- field of a game object; it never goes to the dictionaries, only to the
-- export's `only_here.tsv`.
ALTER TABLE translators.proposals ADD COLUMN IF NOT EXISTS game_section TEXT;
ALTER TABLE translators.proposals ADD COLUMN IF NOT EXISTS game_object TEXT;
ALTER TABLE translators.proposals ADD COLUMN IF NOT EXISTS game_object_en TEXT;
ALTER TABLE translators.proposals ADD COLUMN IF NOT EXISTS game_field TEXT;
ALTER TABLE translators.proposals ADD COLUMN IF NOT EXISTS game_idx SMALLINT;

-- One live proposal per author, entry and scope: every place, or one place —
-- a story line, or a field of a game object.
DROP INDEX IF EXISTS translators.proposals_one_per_author_scope;
CREATE UNIQUE INDEX IF NOT EXISTS proposals_one_per_author_scope
    ON translators.proposals (md5(edit_key), author_id,
        (CASE WHEN context THEN story_id || ':' || line || ':' || part || ':' || COALESCE(game_section, '')
                  || '/' || COALESCE(game_object, '') || '/' || COALESCE(game_field, '') || '/' || COALESCE(game_idx, 0)
              ELSE '' END))
    WHERE deleted_at IS NULL;
