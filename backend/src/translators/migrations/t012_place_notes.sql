-- What tells a text from the others at the same field of an object: a
-- talent at elite 2 and with potential 6, a skill's description at mastery
-- 3, a stage's challenge mode. Codes, written by the import
-- (`texts/gamedata.rs`, `GamePlace::note`): variants joined by `,`, the parts
-- of one by `+`.
ALTER TABLE translators.text_places ADD COLUMN IF NOT EXISTS note TEXT NOT NULL DEFAULT '';
