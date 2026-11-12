-- What is wrong with the game's Russian of each text, worked out after every
-- import and glossary download (`texts/problems.rs`): `glossary`,
-- `brokenTag`, `noHighlight`, `newlines`. The search filters by it.
ALTER TABLE translators.texts ADD COLUMN IF NOT EXISTS problems TEXT[] NOT NULL DEFAULT '{}';
CREATE INDEX IF NOT EXISTS texts_problems ON translators.texts USING gin (problems) WHERE problems <> '{}';
