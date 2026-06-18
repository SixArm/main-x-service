-- Persist person fields previously dropped by the repository:
-- tax_id (scalar) plus normalized child tables for documents,
-- emergency_contacts (+ telecom), and photos.
ALTER TABLE persons ADD COLUMN IF NOT EXISTS tax_id VARCHAR(64);

CREATE TABLE IF NOT EXISTS person_documents (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    person_id           UUID         NOT NULL REFERENCES persons (id) ON DELETE CASCADE,
    document_type       VARCHAR(32)  NOT NULL,
    number              VARCHAR(128) NOT NULL,
    issuing_country     VARCHAR(64),
    issuing_authority   VARCHAR(500),
    issue_date          DATE,
    expiry_date         DATE,
    verified            BOOLEAN      NOT NULL DEFAULT FALSE,
    position            INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_person_documents_person ON person_documents (person_id);
CREATE INDEX IF NOT EXISTS idx_person_documents_number ON person_documents (document_type, number);

CREATE TABLE IF NOT EXISTS person_emergency_contacts (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    person_id           UUID         NOT NULL REFERENCES persons (id) ON DELETE CASCADE,
    name                VARCHAR(500) NOT NULL,
    relationship        VARCHAR(128) NOT NULL,
    is_primary          BOOLEAN      NOT NULL DEFAULT FALSE,
    address_use_type    VARCHAR(16),
    address_line1       VARCHAR(500),
    address_line2       VARCHAR(500),
    address_city        VARCHAR(256),
    address_state       VARCHAR(256),
    address_postal_code VARCHAR(32),
    address_country     VARCHAR(64),
    position            INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_person_emergency_contacts_person ON person_emergency_contacts (person_id);

CREATE TABLE IF NOT EXISTS person_emergency_contact_telecom (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    emergency_contact_id    UUID         NOT NULL REFERENCES person_emergency_contacts (id) ON DELETE CASCADE,
    system                  VARCHAR(16)  NOT NULL,
    value                   VARCHAR(512) NOT NULL,
    use_type                VARCHAR(16),
    position                INTEGER      NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_person_emergency_telecom_contact ON person_emergency_contact_telecom (emergency_contact_id);

CREATE TABLE IF NOT EXISTS person_photos (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    person_id   UUID          NOT NULL REFERENCES persons (id) ON DELETE CASCADE,
    url         VARCHAR(2048) NOT NULL,
    position    INTEGER       NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_person_photos_person ON person_photos (person_id);
