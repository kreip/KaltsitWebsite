-- "Suggest for the glossary": a term a member noticed while reading (a word
-- selected in any line), for the lead to decide. The team's sheet is never
-- written from the site: the lead reads the list and enters what is accepted
-- into the sheet by hand.
CREATE TABLE IF NOT EXISTS translators.glossary_suggestions (
    id BIGSERIAL PRIMARY KEY,
    en TEXT NOT NULL,
    ru TEXT NOT NULL,
    note TEXT,
    -- Where it was noticed: a story line, when it was one, and the line's English.
    story_id TEXT,
    line INTEGER,
    part SMALLINT,
    source_en TEXT,
    author_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- `open`, `accepted` (the lead took it into the sheet) or `declined`.
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'accepted', 'declined')),
    decided_by UUID REFERENCES users (id) ON DELETE SET NULL,
    decided_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ
);
