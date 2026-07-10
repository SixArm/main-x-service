-- Governance audit trail (spec §10.4 / design §10): one row per read or
-- write that touches a governed `subject_of` / `about` (case↔person)
-- edge, matching the case service's audit posture. Reads are audited
-- only when a governed edge is actually surfaced to the caller.
CREATE TABLE audit_log (
    id          UUID PRIMARY KEY,
    actor       TEXT,                      -- bearer sub, if any (NULL for bus-driven writes)
    action      TEXT NOT NULL,             -- read_edge | read_single_view | apply_linked
    edge_kind   TEXT,                      -- the governed edge kind
    from_ref    TEXT,
    to_ref      TEXT,
    occurred_at TIMESTAMPTZ NOT NULL,
    user_ip     TEXT,
    user_agent  TEXT
);
CREATE INDEX audit_log_occurred ON audit_log (occurred_at);
