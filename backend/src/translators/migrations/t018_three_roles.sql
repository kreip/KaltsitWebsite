-- Three roles (Kolooo, 2026-11-13), one per member:
--   `user`       sees the section, proposes, agrees, remarks;
--   `translator` also approves and rejects, exports for the game;
--   `admin`      all of it, "went out in pack N" and the admin panel.
-- The old ones: maintainer (or admin) -> admin, editor -> translator,
-- every other member (a plain translator) -> user.
ALTER TABLE translators.members DROP CONSTRAINT IF EXISTS members_roles;
UPDATE translators.members SET roles = CASE
    WHEN roles && ARRAY['admin', 'maintainer'] THEN ARRAY['admin']
    WHEN roles && ARRAY['editor'] THEN ARRAY['translator']
    ELSE ARRAY['user']
END;
ALTER TABLE translators.members ALTER COLUMN roles SET DEFAULT ARRAY['user'];
ALTER TABLE translators.members ADD CONSTRAINT members_roles
    CHECK (cardinality(roles) = 1 AND roles <@ ARRAY['user', 'translator', 'admin']);

-- "Request access": a signed-in user of the site asks to join; an admin
-- accepts (with a role) or declines. One request per person; asking again
-- after a decline reopens it.
CREATE TABLE IF NOT EXISTS translators.access_requests (
    user_id UUID PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- `open`, `accepted`, `declined`
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'accepted', 'declined')),
    decided_by UUID REFERENCES users (id) ON DELETE SET NULL,
    decided_at TIMESTAMPTZ
);

-- Rejecting a proposal (a translator or an admin), with an optional reason
-- its author sees. A rejected proposal leaves the line; rewriting it makes
-- it a proposal again.
ALTER TABLE translators.proposals ADD COLUMN IF NOT EXISTS rejected_by UUID REFERENCES users (id) ON DELETE SET NULL;
ALTER TABLE translators.proposals ADD COLUMN IF NOT EXISTS rejected_at TIMESTAMPTZ;
ALTER TABLE translators.proposals ADD COLUMN IF NOT EXISTS reject_reason TEXT;
CREATE INDEX IF NOT EXISTS proposals_rejected ON translators.proposals (rejected_at) WHERE rejected_at IS NOT NULL;
