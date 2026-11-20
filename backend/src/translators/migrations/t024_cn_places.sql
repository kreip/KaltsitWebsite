-- The Chinese original at each place of the game data, from the CN client's
-- tables (the import's `TRANSLATORS_CN_DIR`): one English text may stand
-- with different Chinese at different places, so it is kept per place.
ALTER TABLE translators.text_places ADD COLUMN IF NOT EXISTS cn TEXT;
