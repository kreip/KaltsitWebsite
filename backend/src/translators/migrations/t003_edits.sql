-- Line editing. Every decision is keyed by the
-- English text of the dictionary entry (`edit_key`): one entry is one
-- translation in the game, wherever the line stands. The place where it was
-- made (story, line, part) is kept to show and link it.

-- `translator` (default), `editor` (the one lead: approves), `admin` (roles).
ALTER TABLE translators.members
    ADD COLUMN IF NOT EXISTS role TEXT NOT NULL DEFAULT 'translator'
        CHECK (role IN ('translator', 'editor', 'admin'));

-- The dictionary entry the import found for the line, else the whole English
-- as the plugin sees it. Filled by the import; old rows fall back to `en`.
ALTER TABLE translators.story_lines ADD COLUMN IF NOT EXISTS edit_key TEXT;
UPDATE translators.story_lines SET edit_key = en WHERE edit_key IS NULL;
ALTER TABLE translators.story_lines ALTER COLUMN edit_key SET NOT NULL;
CREATE INDEX IF NOT EXISTS story_lines_edit_key ON translators.story_lines (server, edit_key);

-- One translator's proposed Russian for one entry: at most one live proposal
-- per author and key (rewriting it adds a version). Soft-deleted.
-- `outdated_at`: the key is gone from the story after an import.
CREATE TABLE IF NOT EXISTS translators.proposals (
    id BIGSERIAL PRIMARY KEY,
    edit_key TEXT NOT NULL,
    text TEXT NOT NULL,
    -- `kit`: reserved for the review kit's reports, unused for now.
    source TEXT NOT NULL CHECK (source IN ('own', 'sova', 'ai', 'kit')),
    author_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    story_id TEXT NOT NULL,
    line INTEGER NOT NULL,
    part SMALLINT NOT NULL,
    outdated_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS proposals_key ON translators.proposals (edit_key) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS proposals_author ON translators.proposals (author_id) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS proposals_place ON translators.proposals (story_id) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS proposals_one_per_author
    ON translators.proposals (edit_key, author_id) WHERE deleted_at IS NULL;

-- Every text a proposal ever had, the first one included: its history.
CREATE TABLE IF NOT EXISTS translators.proposal_versions (
    id BIGSERIAL PRIMARY KEY,
    proposal_id BIGINT NOT NULL REFERENCES translators.proposals (id) ON DELETE CASCADE,
    text TEXT NOT NULL,
    source TEXT NOT NULL,
    author_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS proposal_versions_proposal ON translators.proposal_versions (proposal_id);

-- "Agree": one vote per person and variant. `proposal_id` NULL = the
-- Russian as in the game ("keep as in the game").
CREATE TABLE IF NOT EXISTS translators.agreements (
    edit_key TEXT NOT NULL,
    proposal_id BIGINT REFERENCES translators.proposals (id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS agreements_one_vote
    ON translators.agreements (edit_key, COALESCE(proposal_id, 0), user_id);

-- The lead's decision for an entry: a proposal, or `proposal_id` NULL = the
-- game's Russian checked and kept (never exported). `text` is the approved
-- Russian at that moment.
CREATE TABLE IF NOT EXISTS translators.approvals (
    edit_key TEXT PRIMARY KEY,
    proposal_id BIGINT REFERENCES translators.proposals (id) ON DELETE SET NULL,
    text TEXT NOT NULL,
    approved_by UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    approved_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    story_id TEXT NOT NULL,
    line INTEGER NOT NULL,
    part SMALLINT NOT NULL,
    outdated_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS approvals_place ON translators.approvals (story_id);

-- Every approval and withdrawal, for the record.
CREATE TABLE IF NOT EXISTS translators.approval_log (
    id BIGSERIAL PRIMARY KEY,
    edit_key TEXT NOT NULL,
    proposal_id BIGINT,
    text TEXT,
    -- `approve` | `withdraw`
    action TEXT NOT NULL,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- A short note on an entry. Soft-deleted; "resolved" folds it away.
CREATE TABLE IF NOT EXISTS translators.comments (
    id BIGSERIAL PRIMARY KEY,
    edit_key TEXT NOT NULL,
    text TEXT NOT NULL,
    author_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    resolved_by UUID REFERENCES users (id) ON DELETE SET NULL,
    resolved_at TIMESTAMPTZ,
    story_id TEXT NOT NULL,
    line INTEGER NOT NULL,
    part SMALLINT NOT NULL,
    outdated_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS comments_key ON translators.comments (edit_key) WHERE deleted_at IS NULL;
