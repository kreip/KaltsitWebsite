-- An entry's English may be pages long (an operator's file, a module's
-- story): too long for a b-tree, which every table of line editing kept its
-- key in, so a proposal for such a text could not be saved. As in
-- `translators.texts`: an entry is found by a hash index, and "one per
-- entry" is kept by the key's md5.
DROP INDEX IF EXISTS translators.proposals_key;
CREATE INDEX IF NOT EXISTS proposals_key_hash ON translators.proposals USING hash (edit_key);
DROP INDEX IF EXISTS translators.proposals_one_per_author_scope;
CREATE UNIQUE INDEX IF NOT EXISTS proposals_one_per_author_scope
    ON translators.proposals (md5(edit_key), author_id,
        (CASE WHEN context THEN story_id || ':' || line || ':' || part ELSE '' END))
    WHERE deleted_at IS NULL;

ALTER TABLE translators.approvals DROP CONSTRAINT IF EXISTS approvals_pkey;
ALTER TABLE translators.approvals ALTER COLUMN edit_key SET NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS approvals_entry ON translators.approvals (md5(edit_key));
CREATE INDEX IF NOT EXISTS approvals_key_hash ON translators.approvals USING hash (edit_key);

DROP INDEX IF EXISTS translators.agreements_one_vote_v;
CREATE UNIQUE INDEX IF NOT EXISTS agreements_one_vote_v
    ON translators.agreements (md5(edit_key), variant, COALESCE(proposal_id, 0),
        COALESCE(story_id, ''), COALESCE(line, 0), COALESCE(part, 0), user_id);
CREATE INDEX IF NOT EXISTS agreements_key_hash ON translators.agreements USING hash (edit_key);

DROP INDEX IF EXISTS translators.comments_key;
CREATE INDEX IF NOT EXISTS comments_key_hash ON translators.comments USING hash (edit_key);

DROP INDEX IF EXISTS translators.release_items_key;
CREATE INDEX IF NOT EXISTS release_items_key_hash ON translators.release_items USING hash (edit_key);

DROP INDEX IF EXISTS translators.ru_history_key;
CREATE INDEX IF NOT EXISTS ru_history_key_hash ON translators.ru_history USING hash (edit_key);

CREATE INDEX IF NOT EXISTS approval_log_key_hash ON translators.approval_log USING hash (edit_key);
