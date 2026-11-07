-- Relational children of `events`: identifiers, locations, parties,
-- offers, links, sub-events.

-- ---------------------------------------------------------------------------
-- Identifiers
-- ---------------------------------------------------------------------------

CREATE TABLE event_identifiers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    use_type VARCHAR(20) CHECK (use_type IN ('usual','official','temp','secondary','old')),
    identifier_type VARCHAR(32) NOT NULL CHECK (identifier_type IN (
        'BOOKING_NUMBER','CONFIRMATION_CODE','TICKET_NUMBER',
        'ENCOUNTER_ID','TRANSACTION_ID','EXTERNAL_REF','TAX','OTHER'
    )),
    system VARCHAR(255) NOT NULL,
    value VARCHAR(255) NOT NULL,
    assigner VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(event_id, identifier_type, system, value)
);

CREATE INDEX idx_event_identifiers_event_id ON event_identifiers(event_id);
CREATE INDEX idx_event_identifiers_type ON event_identifiers(identifier_type);
CREATE INDEX idx_event_identifiers_value ON event_identifiers(value);
CREATE INDEX idx_event_identifiers_lookup ON event_identifiers(identifier_type, system, value);

-- ---------------------------------------------------------------------------
-- Locations
--
-- `kind` discriminates the variant: 'place' / 'postal_address' /
-- 'virtual' / 'text'. Only the columns relevant to the variant are
-- populated. `position` preserves array order.
-- ---------------------------------------------------------------------------

CREATE TABLE event_locations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    kind VARCHAR(16) NOT NULL CHECK (kind IN ('place','postal_address','virtual','text')),
    place_id UUID,
    name VARCHAR(255),
    line1 VARCHAR(255),
    line2 VARCHAR(255),
    city VARCHAR(100),
    state VARCHAR(50),
    postal_code VARCHAR(20),
    country VARCHAR(100),
    latitude DOUBLE PRECISION,
    longitude DOUBLE PRECISION,
    url VARCHAR(2048),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT chk_loc_latitude CHECK (latitude IS NULL OR (latitude BETWEEN -90 AND 90)),
    CONSTRAINT chk_loc_longitude CHECK (longitude IS NULL OR (longitude BETWEEN -180 AND 180))
);

CREATE INDEX idx_event_locations_event_id ON event_locations(event_id);
CREATE INDEX idx_event_locations_kind ON event_locations(kind);
CREATE INDEX idx_event_locations_place_id ON event_locations(place_id);
CREATE INDEX idx_event_locations_postal_code ON event_locations(postal_code);
CREATE INDEX idx_event_locations_country ON event_locations(country);

-- ---------------------------------------------------------------------------
-- Parties (organizer / performer / attendee / sponsor / funder / contributor)
-- ---------------------------------------------------------------------------

CREATE TABLE event_parties (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    role VARCHAR(16) NOT NULL CHECK (role IN (
        'organizer','performer','attendee','sponsor','funder','contributor'
    )),
    party_kind VARCHAR(16) NOT NULL CHECK (party_kind IN ('person','organization')),
    party_id UUID,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255),
    url VARCHAR(2048),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_event_parties_event_id ON event_parties(event_id);
CREATE INDEX idx_event_parties_role ON event_parties(role);
CREATE INDEX idx_event_parties_party_id ON event_parties(party_id);
CREATE INDEX idx_event_parties_name_lower ON event_parties(LOWER(name));

-- ---------------------------------------------------------------------------
-- Offers (tickets / pricing tiers)
-- ---------------------------------------------------------------------------

CREATE TABLE event_offers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    name VARCHAR(255),
    price NUMERIC(12, 4),
    price_currency VARCHAR(3),
    url VARCHAR(2048),
    availability VARCHAR(16) CHECK (availability IN (
        'in_stock','sold_out','pre_order','out_of_stock','discontinued'
    )),
    valid_from TIMESTAMPTZ,
    valid_through TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT chk_offer_dates CHECK (
        valid_from IS NULL OR valid_through IS NULL OR valid_through >= valid_from
    )
);

CREATE INDEX idx_event_offers_event_id ON event_offers(event_id);
CREATE INDEX idx_event_offers_availability ON event_offers(availability);

-- ---------------------------------------------------------------------------
-- Links (cross-event references / merge / see-also)
-- ---------------------------------------------------------------------------

CREATE TABLE event_links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    other_event_id UUID NOT NULL,
    link_type VARCHAR(16) NOT NULL CHECK (link_type IN ('replaced_by','replaces','refer','seealso')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by VARCHAR(255),
    UNIQUE(event_id, other_event_id, link_type)
);

CREATE INDEX idx_event_links_event_id ON event_links(event_id);
CREATE INDEX idx_event_links_other_event_id ON event_links(other_event_id);
CREATE INDEX idx_event_links_type ON event_links(link_type);

-- ---------------------------------------------------------------------------
-- Sub-events (schema.org/subEvent list)
-- ---------------------------------------------------------------------------

CREATE TABLE event_sub_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    sub_event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(event_id, sub_event_id)
);

CREATE INDEX idx_event_sub_events_event_id ON event_sub_events(event_id);
CREATE INDEX idx_event_sub_events_sub_event_id ON event_sub_events(sub_event_id);
