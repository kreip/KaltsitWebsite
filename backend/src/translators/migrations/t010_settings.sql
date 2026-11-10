-- The section's shared settings, one row each. `recommend_rule`: which rule
-- marks a line's recommended variant, `A` or `B` (the lead's switch; `B`
-- when unset).
CREATE TABLE IF NOT EXISTS translators.settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_by UUID REFERENCES users (id) ON DELETE SET NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
