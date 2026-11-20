-- The operators' genders as their files state them (`import/genders.rs`), and
-- each story line's speaker's when the speaker is an operator: "check the
-- gender" compares the Russian's «я сделал / я сделала» with it.
CREATE TABLE IF NOT EXISTS translators.char_genders (
    char_id TEXT PRIMARY KEY,
    gender TEXT NOT NULL CHECK (gender IN ('f', 'm'))
);
ALTER TABLE translators.story_lines ADD COLUMN IF NOT EXISTS speaker_gender TEXT;
