-- Part 4: approving and the export.

-- Which variant the lead approved for an entry: `game` (the game's Russian,
-- checked; never exported), `proposal` (with `proposal_id`), `sova` or `ai`
-- (their text at the place it was approved from).
ALTER TABLE translators.approvals ADD COLUMN IF NOT EXISTS variant TEXT;
UPDATE translators.approvals
SET variant = CASE WHEN proposal_id IS NULL THEN 'game' ELSE 'proposal' END
WHERE variant IS NULL;
ALTER TABLE translators.approvals ALTER COLUMN variant SET NOT NULL;
ALTER TABLE translators.approvals ADD CONSTRAINT approvals_variant
    CHECK (variant IN ('game', 'sova', 'ai', 'proposal'));
ALTER TABLE translators.approval_log ADD COLUMN IF NOT EXISTS variant TEXT;
ALTER TABLE translators.approval_log ADD COLUMN IF NOT EXISTS story_id TEXT;
ALTER TABLE translators.approval_log ADD COLUMN IF NOT EXISTS line INTEGER;
ALTER TABLE translators.approval_log ADD COLUMN IF NOT EXISTS part SMALLINT;

-- A proposal for one place is approved for that place only (not the entry).
ALTER TABLE translators.proposals ADD COLUMN IF NOT EXISTS approved_by UUID REFERENCES users (id) ON DELETE SET NULL;
ALTER TABLE translators.proposals ADD COLUMN IF NOT EXISTS approved_at TIMESTAMPTZ;
