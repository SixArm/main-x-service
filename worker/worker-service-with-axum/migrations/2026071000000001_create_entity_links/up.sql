-- Cross-service entity-link WRITE side (agents/share/cross-service-linking.md
-- §4.1). Each row is one OUTBOUND edge this service originates from a
-- worker: for v1 the `same_identity` (person <-> worker) backbone edge
-- (§9), the federation link that resolves one human across the general
-- (person) and workforce (worker) registries. The write is optimistic --
-- it stores the assertion and emits a `linked` event; it never calls the
-- target service, and verification is the read-model aggregator's concern.

CREATE TABLE entity_links (
    id           UUID PRIMARY KEY,              -- edge id (also the event's edge_id); app-assigned
    from_pid     UUID NOT NULL,                 -- this service's originating worker record (its id)
    kind         TEXT NOT NULL,                 -- edge kind token (§9 registry), e.g. same_identity
    to_ref       TEXT NOT NULL,                 -- far record's EntityRef URN, e.g. person:<uuid>
    role         TEXT,                          -- optional role label (unused by same_identity)
    confidence   DOUBLE PRECISION,              -- 1.0 operator-asserted, <1 suggested
    provenance   TEXT NOT NULL,                 -- operator | import | matcher_suggested
    valid_from   DATE,                          -- affiliation start (nullable; same_identity is not temporal)
    valid_to     DATE,                          -- affiliation end ("former ...")
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at   TIMESTAMPTZ                    -- soft-delete (withdrawn edge); NULL while live
);

-- Idempotent-upsert key (§4.1). NULLS NOT DISTINCT so a null valid_from
-- (a non-temporal edge like same_identity) still conflicts on
-- re-assertion (Postgres 15+); otherwise each NULL is distinct and the
-- upsert would insert duplicates. This is what makes the write idempotent.
CREATE UNIQUE INDEX entity_links_upsert
    ON entity_links (from_pid, kind, to_ref, valid_from)
    NULLS NOT DISTINCT;

-- Inverse-lookup / list-active support.
CREATE INDEX entity_links_from
    ON entity_links (from_pid)
    WHERE deleted_at IS NULL;
