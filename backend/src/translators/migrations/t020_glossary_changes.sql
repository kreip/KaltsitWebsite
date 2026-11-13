-- When a term's Russian last changed in the team's sheet (NULL: not since the
-- site first saw it). A line approved after that date is checked for the term;
-- a change after the approval shows the term again, with a note.
ALTER TABLE translators.glossary_terms ADD COLUMN IF NOT EXISTS changed_at TIMESTAMPTZ;
