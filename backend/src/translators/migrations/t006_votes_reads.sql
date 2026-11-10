-- Part 3: "agree" with any variant, "read the
-- scene", progress.

-- An agreement names its variant: `game` (keep as in the game), `proposal`
-- (with `proposal_id`), `sova` or `ai`. Sova's and the AI's texts belong to a
-- place (they differ between places of one entry), so a vote for them keeps
-- the place and the text as it was.
ALTER TABLE translators.agreements ADD COLUMN IF NOT EXISTS variant TEXT;
UPDATE translators.agreements
SET variant = CASE WHEN proposal_id IS NULL THEN 'game' ELSE 'proposal' END
WHERE variant IS NULL;
ALTER TABLE translators.agreements ALTER COLUMN variant SET NOT NULL;
ALTER TABLE translators.agreements ADD CONSTRAINT agreements_variant
    CHECK (variant IN ('game', 'sova', 'ai', 'proposal'));
ALTER TABLE translators.agreements ADD COLUMN IF NOT EXISTS story_id TEXT;
ALTER TABLE translators.agreements ADD COLUMN IF NOT EXISTS line INTEGER;
ALTER TABLE translators.agreements ADD COLUMN IF NOT EXISTS part SMALLINT;
ALTER TABLE translators.agreements ADD COLUMN IF NOT EXISTS variant_text TEXT;
DROP INDEX IF EXISTS translators.agreements_one_vote;
CREATE UNIQUE INDEX IF NOT EXISTS agreements_one_vote_v
    ON translators.agreements (edit_key, variant, COALESCE(proposal_id, 0),
        COALESCE(story_id, ''), COALESCE(line, 0), COALESCE(part, 0), user_id);

-- "I have read this scene": who and when.
CREATE TABLE IF NOT EXISTS translators.story_reads (
    story_id TEXT NOT NULL,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    read_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (story_id, user_id)
);
