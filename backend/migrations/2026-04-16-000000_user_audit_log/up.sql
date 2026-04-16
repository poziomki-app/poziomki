-- GDPR audit trail for user-initiated account actions (deletion, export,
-- password change). Intentionally stores only the user's public id so the
-- row survives the user's row being deleted. This is the record of
-- processing activity required by GDPR Art. 30.
CREATE TABLE user_audit_log (
    id         UUID PRIMARY KEY,
    user_pid   UUID NOT NULL,
    action     TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX user_audit_log_user_pid_idx ON user_audit_log (user_pid);
CREATE INDEX user_audit_log_created_at_idx ON user_audit_log (created_at);
