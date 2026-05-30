# Database Schema Design

## Overview

This document describes the PostgreSQL database schema for the Main Event Service (MPI) system. The schema is designed to support millions of event records with high performance, HIPAA compliance, and full audit trail capabilities.

## Design Principles

1. **Normalization**: Properly normalized to 3NF to avoid data redundancy
2. **Audit Trail**: All tables include created_at, updated_at, created_by, updated_by
3. **Soft Delete**: Support for soft deletes with deleted_at, deleted_by columns
4. **UUIDs**: Use UUIDs for primary keys to support distributed systems
5. **Indexing**: Strategic indexes for common query patterns
6. **Referential Integrity**: Foreign keys with appropriate cascade rules
7. **HIPAA Compliance**: Audit logging and data integrity

## Core Tables

### events

Primary event record table.

```sql
CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    active BOOLEAN NOT NULL DEFAULT true,
    gender VARCHAR(20) NOT NULL CHECK (gender IN ('male', 'female', 'other', 'unknown')),
    birth_date DATE,
    deceased BOOLEAN NOT NULL DEFAULT false,
    deceased_datetime TIMESTAMPTZ,
    marital_status VARCHAR(50),
    multiple_birth BOOLEAN,
    managing_organization_id UUID REFERENCES organizations(id),

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by VARCHAR(255),
    updated_by VARCHAR(255),

    -- Soft delete
    deleted_at TIMESTAMPTZ,
    deleted_by VARCHAR(255),

    -- Indexes
    INDEX idx_events_birth_date (birth_date),
    INDEX idx_events_gender (gender),
    INDEX idx_events_active (active),
    INDEX idx_events_organization (managing_organization_id),
    INDEX idx_events_deleted_at (deleted_at)
);
```

### event_names

Stores multiple names per event (legal name, maiden name, aliases, etc.).

```sql
CREATE TABLE event_names (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    use_type VARCHAR(20) CHECK (use_type IN ('usual', 'official', 'temp', 'nickname', 'anonymous', 'old', 'maiden')),
    family VARCHAR(255) NOT NULL,
    given TEXT[] NOT NULL DEFAULT '{}',  -- Array of given names
    prefix TEXT[] NOT NULL DEFAULT '{}', -- Array of prefixes (Dr., Mr., etc.)
    suffix TEXT[] NOT NULL DEFAULT '{}', -- Array of suffixes (Jr., III, etc.)
    is_primary BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Indexes
    INDEX idx_event_names_event_id (event_id),
    INDEX idx_event_names_family (family),
    INDEX idx_event_names_is_primary (is_primary)
);
```

### event_identifiers

Stores event identifiers (MRN, SSN, driver's license, etc.).

```sql
CREATE TABLE event_identifiers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    use_type VARCHAR(20) CHECK (use_type IN ('usual', 'official', 'temp', 'secondary', 'old')),
    identifier_type VARCHAR(10) NOT NULL CHECK (identifier_type IN ('MRN', 'SSN', 'DL', 'NPI', 'PPN', 'TAX', 'OTHER')),
    system VARCHAR(255) NOT NULL,  -- Namespace/system URI
    value VARCHAR(255) NOT NULL,   -- The actual identifier value
    assigner VARCHAR(255),         -- Organization that issued the identifier

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Unique constraint: one identifier per system
    UNIQUE(system, value),

    -- Indexes
    INDEX idx_event_identifiers_event_id (event_id),
    INDEX idx_event_identifiers_type (identifier_type),
    INDEX idx_event_identifiers_value (value),
    INDEX idx_event_identifiers_system_value (system, value)
);
```

### event_addresses

Stores multiple addresses per event.

```sql
CREATE TABLE event_addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    use_type VARCHAR(20) CHECK (use_type IN ('home', 'work', 'temp', 'old', 'billing')),
    line1 VARCHAR(255),
    line2 VARCHAR(255),
    city VARCHAR(100),
    state VARCHAR(50),
    postal_code VARCHAR(20),
    country VARCHAR(100) DEFAULT 'USA',
    is_primary BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Indexes
    INDEX idx_event_addresses_event_id (event_id),
    INDEX idx_event_addresses_postal_code (postal_code),
    INDEX idx_event_addresses_city_state (city, state),
    INDEX idx_event_addresses_is_primary (is_primary)
);
```

### event_contacts

Stores contact information (phone, email, etc.).

```sql
CREATE TABLE event_contacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    system VARCHAR(20) NOT NULL CHECK (system IN ('phone', 'fax', 'email', 'pager', 'url', 'sms', 'other')),
    value VARCHAR(255) NOT NULL,
    use_type VARCHAR(20) CHECK (use_type IN ('home', 'work', 'temp', 'old', 'mobile')),
    is_primary BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Indexes
    INDEX idx_event_contacts_event_id (event_id),
    INDEX idx_event_contacts_system (system),
    INDEX idx_event_contacts_value (value),
    INDEX idx_event_contacts_is_primary (is_primary)
);
```

### event_links

Links between event records (duplicates, merges, references).

```sql
CREATE TABLE event_links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    other_event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    link_type VARCHAR(20) NOT NULL CHECK (link_type IN ('replaced_by', 'replaces', 'refer', 'seealso')),

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by VARCHAR(255),

    -- Prevent self-links
    CHECK (event_id != other_event_id),

    -- Prevent duplicate links
    UNIQUE(event_id, other_event_id, link_type),

    -- Indexes
    INDEX idx_event_links_event_id (event_id),
    INDEX idx_event_links_other_event_id (other_event_id),
    INDEX idx_event_links_link_type (link_type)
);
```

### organizations

Healthcare organizations (hospitals, clinics, etc.).

```sql
CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    active BOOLEAN NOT NULL DEFAULT true,
    name VARCHAR(255) NOT NULL,
    alias TEXT[] NOT NULL DEFAULT '{}',  -- Array of alias names
    org_type TEXT[] NOT NULL DEFAULT '{}',  -- Array of organization types
    part_of UUID REFERENCES organizations(id),  -- Parent organization

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by VARCHAR(255),
    updated_by VARCHAR(255),

    -- Soft delete
    deleted_at TIMESTAMPTZ,
    deleted_by VARCHAR(255),

    -- Indexes
    INDEX idx_organizations_name (name),
    INDEX idx_organizations_active (active),
    INDEX idx_organizations_part_of (part_of),
    INDEX idx_organizations_deleted_at (deleted_at)
);
```

### organization_identifiers

Organization identifiers (NPI, Tax ID, etc.).

```sql
CREATE TABLE organization_identifiers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    use_type VARCHAR(20) CHECK (use_type IN ('usual', 'official', 'temp', 'secondary', 'old')),
    identifier_type VARCHAR(10) NOT NULL,
    system VARCHAR(255) NOT NULL,
    value VARCHAR(255) NOT NULL,
    assigner VARCHAR(255),

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Unique constraint
    UNIQUE(system, value),

    -- Indexes
    INDEX idx_org_identifiers_organization_id (organization_id),
    INDEX idx_org_identifiers_type (identifier_type),
    INDEX idx_org_identifiers_value (value)
);
```

### organization_addresses

Organization addresses.

```sql
CREATE TABLE organization_addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    use_type VARCHAR(20) CHECK (use_type IN ('home', 'work', 'temp', 'old', 'billing')),
    line1 VARCHAR(255),
    line2 VARCHAR(255),
    city VARCHAR(100),
    state VARCHAR(50),
    postal_code VARCHAR(20),
    country VARCHAR(100) DEFAULT 'USA',
    is_primary BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Indexes
    INDEX idx_org_addresses_organization_id (organization_id),
    INDEX idx_org_addresses_postal_code (postal_code)
);
```

### organization_contacts

Organization contact information.

```sql
CREATE TABLE organization_contacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    system VARCHAR(20) NOT NULL CHECK (system IN ('phone', 'fax', 'email', 'pager', 'url', 'sms', 'other')),
    value VARCHAR(255) NOT NULL,
    use_type VARCHAR(20) CHECK (use_type IN ('home', 'work', 'temp', 'old', 'mobile')),
    is_primary BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Indexes
    INDEX idx_org_contacts_organization_id (organization_id),
    INDEX idx_org_contacts_system (system)
);
```

## Audit Tables

### audit_log

Complete audit trail for HIPAA compliance.

```sql
CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_id VARCHAR(255),
    action VARCHAR(50) NOT NULL,  -- CREATE, UPDATE, DELETE, MERGE, LINK, etc.
    entity_type VARCHAR(50) NOT NULL,  -- event, organization, etc.
    entity_id UUID NOT NULL,
    old_values JSONB,
    new_values JSONB,
    ip_address INET,
    user_agent TEXT,

    -- Indexes
    INDEX idx_audit_log_timestamp (timestamp),
    INDEX idx_audit_log_entity (entity_type, entity_id),
    INDEX idx_audit_log_user_id (user_id),
    INDEX idx_audit_log_action (action)
);
```

## Matching Tables

### event_match_scores

Stores calculated match scores between event records.

```sql
CREATE TABLE event_match_scores (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    candidate_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    total_score DECIMAL(5,4) NOT NULL,  -- 0.0000 to 1.0000
    name_score DECIMAL(5,4),
    birth_date_score DECIMAL(5,4),
    gender_score DECIMAL(5,4),
    address_score DECIMAL(5,4),
    identifier_score DECIMAL(5,4),
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Prevent self-matching
    CHECK (event_id != candidate_id),

    -- Unique constraint
    UNIQUE(event_id, candidate_id),

    -- Indexes
    INDEX idx_match_scores_event_id (event_id),
    INDEX idx_match_scores_total_score (total_score DESC),
    INDEX idx_match_scores_calculated_at (calculated_at)
);
```

## Functions and Triggers

### Update timestamp trigger

```sql
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';
```

Apply to all tables with updated_at:

```sql
CREATE TRIGGER update_events_updated_at BEFORE UPDATE ON events
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organizations_updated_at BEFORE UPDATE ON organizations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Repeat for all tables with updated_at
```

## Performance Considerations

### Indexes

All tables include indexes for:

- Primary keys (automatic)
- Foreign keys
- Common search fields (name, birth_date, postal_code)
- Soft delete fields (deleted_at)
- Primary flags (is_primary)

### Partitioning (Future)

For very large deployments (10M+ events), consider:

- Partitioning `audit_log` by timestamp (monthly partitions)
- Partitioning `event_match_scores` if storing all calculated scores

### Statistics

```sql
-- Update statistics for query planner
ANALYZE events;
ANALYZE event_names;
ANALYZE event_identifiers;
```

## Security

### Row-Level Security (RLS)

Can be enabled for multi-tenant deployments:

```sql
ALTER TABLE events ENABLE ROW LEVEL SECURITY;
CREATE POLICY event_access ON events
    FOR ALL
    USING (managing_organization_id = current_setting('app.organization_id')::uuid);
```

## Migration Strategy

1. Create tables in dependency order (organizations before events)
2. Add indexes after initial data load for better performance
3. Enable triggers after bulk data import
4. Run ANALYZE after significant data changes

## Data Integrity Rules

1. **Cascading Deletes**: Child records (names, addresses) cascade when event deleted
2. **Referential Integrity**: All foreign keys enforced
3. **Check Constraints**: Enum values enforced at database level
4. **Unique Constraints**: Prevent duplicate identifiers
5. **Soft Deletes**: Never hard delete events (HIPAA requirement)

## Capacity Planning

Estimated storage for 10 million events:

| Table             | Rows | Size per Row | Total Size |
| ----------------- | ---- | ------------ | ---------- |
| events            | 10M  | 500 bytes    | 5 GB       |
| event_names       | 15M  | 300 bytes    | 4.5 GB     |
| event_identifiers | 30M  | 200 bytes    | 6 GB       |
| event_addresses   | 20M  | 250 bytes    | 5 GB       |
| event_contacts    | 30M  | 200 bytes    | 6 GB       |
| **Total**         |      |              | **~27 GB** |

Add 50% for indexes: **~40 GB total**
