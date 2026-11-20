-- Stories the CN client has and the EN one does not yet ("Только в китайской
-- версии"): their lines stand in story_lines with no English (`cn_only`), each
-- line its own entry (`cn:<story>#<line>.<part>`), never exported; when the EN
-- client gets the story, the import moves what was proposed and approved onto
-- the EN lines by the story and the line, or leaves it for a check by hand
-- (`cn_transfers`).
ALTER TABLE translators.story_lines ADD COLUMN IF NOT EXISTS cn_only BOOLEAN NOT NULL DEFAULT FALSE;

-- A second translator's variant from arktr (the user's own corrections of the
-- machine's text): its author's name is the frontend's (translatorVariants.ts).
ALTER TABLE translators.story_lines ADD COLUMN IF NOT EXISTS artem TEXT;

CREATE TABLE IF NOT EXISTS translators.cn_only_stories (
    story_id TEXT PRIMARY KEY,
    story_txt TEXT NOT NULL,
    group_id TEXT NOT NULL,
    group_cn TEXT NOT NULL,
    group_ru TEXT,
    name_cn TEXT NOT NULL,
    name_ru TEXT,
    avg_tag TEXT,
    code TEXT,
    sort INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS translators.cn_transfers (
    old_key TEXT PRIMARY KEY,
    story_id TEXT NOT NULL,
    -- `moved` onto `new_key` at the EN place, or `manual`: no sure EN line.
    state TEXT NOT NULL CHECK (state IN ('moved', 'manual')),
    new_key TEXT,
    line INTEGER,
    part SMALLINT,
    cn TEXT,
    at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE translators.proposals DROP CONSTRAINT IF EXISTS proposals_source_check;
ALTER TABLE translators.proposals ADD CONSTRAINT proposals_source_check
    CHECK (source IN ('own', 'sova', 'artem', 'ai', 'kit'));
ALTER TABLE translators.comments DROP CONSTRAINT IF EXISTS comments_variant_check;
ALTER TABLE translators.comments ADD CONSTRAINT comments_variant_check
    CHECK (variant IN ('game', 'sova', 'artem', 'ai', 'proposal'));
ALTER TABLE translators.agreements DROP CONSTRAINT IF EXISTS agreements_variant;
ALTER TABLE translators.agreements ADD CONSTRAINT agreements_variant
    CHECK (variant IN ('game', 'sova', 'artem', 'ai', 'proposal'));
ALTER TABLE translators.approvals DROP CONSTRAINT IF EXISTS approvals_variant;
ALTER TABLE translators.approvals ADD CONSTRAINT approvals_variant
    CHECK (variant IN ('game', 'sova', 'artem', 'ai', 'proposal'));
