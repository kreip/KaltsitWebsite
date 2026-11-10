-- Comparing versions. The Russian the game showed for an entry before an
-- import changed it (a new pack): the import adds a row for every entry whose
-- Russian differs from the one it replaces. The proposals' own old texts are
-- already kept (`proposal_versions`).
CREATE TABLE IF NOT EXISTS translators.ru_history (
    id BIGSERIAL PRIMARY KEY,
    server TEXT NOT NULL,
    edit_key TEXT NOT NULL,
    -- The Russian before the import.
    text TEXT NOT NULL,
    -- When the import that replaced it started.
    changed_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS ru_history_key ON translators.ru_history (server, edit_key);
