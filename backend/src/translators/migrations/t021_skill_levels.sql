-- A dictionary entry with a skill level's numbers put in ("dealing 230% ATK
-- …") stands at the operator's card under the skill's description
-- (`text_places.field = 'skill.level'`): `of_key` is the entry of the
-- description template it was made from ("Apply to the levels" fills them).
ALTER TABLE translators.text_places ADD COLUMN IF NOT EXISTS of_key TEXT;
CREATE INDEX IF NOT EXISTS text_places_of_key ON translators.text_places USING hash (of_key);
