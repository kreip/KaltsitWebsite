-- A replacement made at once ("Replace «Доберман» → «Доберманн» in N more
-- lines", "Replace with the glossary's form"): its proposals are one edit in
-- the Edits list, approved or rejected whole, places taken out one by one.
CREATE TABLE IF NOT EXISTS translators.proposal_batches (
    id BIGSERIAL PRIMARY KEY,
    -- `word` (a word replaced after a proposal) or `glossary` (the glossary's form put in).
    kind TEXT NOT NULL CHECK (kind IN ('word', 'glossary')),
    word_from TEXT NOT NULL,
    word_to TEXT NOT NULL,
    author_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE translators.proposals
    ADD COLUMN IF NOT EXISTS batch_id BIGINT REFERENCES translators.proposal_batches (id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS proposals_batch ON translators.proposals (batch_id) WHERE batch_id IS NOT NULL;
