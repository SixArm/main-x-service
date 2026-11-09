//! Repository layer: converts between the domain
//! [`Event`] and the row structs defined in
//! [`crate::db::models`], and runs every statement against the event
//! tables as a `sqlx::query!` / `query_as!` literal.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::Result;
use crate::db::outbox::OutboxInsert;
use crate::models::{
    Address, Event, EventAttendanceMode, EventLink, EventStatus, EventType, Identifier,
    IdentifierType, IdentifierUse, LinkType, Location, Offer, OfferAvailability, Party, PartyKind,
    Place, VirtualLocation,
};
use crate::streaming::{EventKind, EventTransport};

use super::models::{
    EventRow, IdentifierRow, LinkRow, LocationRow, OfferRow, PartyRow, SubEventRow, TextValueRow,
};

pub use super::audit::AuditContext;

/// CRUD + simple search abstraction for [`Event`]. Object-safe (via
/// `async_trait`) so it can be held as `Arc<dyn EventRepository>`.
#[async_trait::async_trait]
pub trait EventRepository: Send + Sync {
    /// Insert a new event (and its child rows); returns the stored event.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction, any row insert, or the
    /// reload of child rows fails.
    async fn create(&self, event: &Event) -> Result<Event>;
    /// Fetch one non-deleted event by id, if it exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the parent or child-row queries fail.
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Event>>;
    /// Replace an event and its child rows; returns the stored event.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction or any query fails, or
    /// [`Error::Validation`](crate::Error::Validation) if the event
    /// cannot be re-read after the update.
    async fn update(&self, event: &Event) -> Result<Event>;
    /// Merge the `duplicate_id` record into `survivor`: apply the
    /// survivor's parent + child row updates and soft-delete the
    /// duplicate **in one transaction**, so the whole merge commits (or
    /// rolls back) atomically. Under the outbox transport this enqueues a
    /// `Merged` outbox row for the survivor (carrying the duplicate's pid
    /// via `merged_from`) and a `Deleted` outbox row for the duplicate on
    /// the same transaction. Returns the reloaded survivor.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction or any query fails, or
    /// [`Error::Validation`](crate::Error::Validation) if the survivor
    /// cannot be re-read after the merge.
    async fn merge(&self, survivor: &Event, duplicate_id: &Uuid) -> Result<Event>;
    /// Soft-delete an event by id.
    ///
    /// # Errors
    ///
    /// Returns an error if the update query fails.
    async fn delete(&self, id: &Uuid) -> Result<()>;
    /// List active events with `limit`/`offset` pagination.
    ///
    /// # Errors
    ///
    /// Returns an error if the id query or any per-id reload fails.
    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Event>>;
    /// List up to `limit` active events' names, in id order, starting
    /// after `after` (keyset paging). The value-frequency scan reads this
    /// rather than whole records (`matching::frequencies`).
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    async fn list_active_names(&self, limit: u64, after: Option<Uuid>) -> Result<Vec<NameRecord>>;
}

/// One active event's names, as [`EventRepository::list_active_names`]
/// returns them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameRecord {
    /// The event id.
    pub id: Uuid,
    /// The primary name.
    pub name: String,
    /// The alternate names, in stored order.
    pub alternate_names: Vec<String>,
}

/// sqlx-backed [`EventRepository`] with optional event publishing and
/// audit logging.
pub struct PgEventRepository {
    /// The connection pool.
    db: PgPool,
    /// Optional event-stream publisher for CRUD notifications.
    event_publisher: Option<std::sync::Arc<dyn crate::streaming::EventProducer>>,
    /// Optional audit-log repository for write trails.
    audit_log: Option<std::sync::Arc<super::audit::AuditLogRepository>>,
    /// Which event transport is active (durable event bus, Phase 2).
    /// [`EventTransport::Memory`] (default) keeps the legacy post-commit
    /// ring-buffer publish; [`EventTransport::Outbox`] additionally writes
    /// one `event_outbox` row **inside** each write's transaction.
    transport: EventTransport,
}

impl PgEventRepository {
    /// Construct a repository over `db` with no publisher or audit log,
    /// and the default [`EventTransport::Memory`] transport (behaviour
    /// unchanged from before the outbox landed).
    #[must_use]
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            event_publisher: None,
            audit_log: None,
            transport: EventTransport::Memory,
        }
    }

    /// Builder: select the event transport (see [`EventTransport`]).
    /// `AppState` wires this from `EVENT_EVENT_TRANSPORT` via
    /// [`crate::streaming::transport`].
    #[must_use]
    pub fn with_transport(mut self, transport: EventTransport) -> Self {
        self.transport = transport;
        self
    }

    /// When the outbox transport is active, enqueue one `event_outbox`
    /// row for `kind` applied to `event` **on `conn`** — pass the open
    /// transaction so the row commits with the entity write (the outbox
    /// atomicity guarantee). A no-op under [`EventTransport::Memory`].
    ///
    /// The connection parameter lives here on a concrete method (not on
    /// the object-safe [`EventRepository`] trait), which is how a
    /// `dyn`-trait repository threads the outbox insert into its own
    /// transaction.
    async fn enqueue_outbox(
        &self,
        conn: &mut PgConnection,
        event: &Event,
        kind: EventKind,
    ) -> Result<()> {
        if self.transport.is_outbox() {
            OutboxInsert::for_event(event, kind)?
                .insert_on(conn)
                .await?;
        }
        Ok(())
    }

    /// Builder: attach an event-stream publisher.
    #[must_use]
    pub fn with_event_publisher(
        mut self,
        publisher: std::sync::Arc<dyn crate::streaming::EventProducer>,
    ) -> Self {
        self.event_publisher = Some(publisher);
        self
    }

    /// Builder: attach an audit-log repository.
    #[must_use]
    pub fn with_audit_log(
        mut self,
        audit_log: std::sync::Arc<super::audit::AuditLogRepository>,
    ) -> Self {
        self.audit_log = Some(audit_log);
        self
    }

    /// Publish a streaming event if a publisher is attached, logging
    /// (but swallowing) any publish error.
    fn publish_event(&self, event: crate::streaming::EventEvent) {
        if let Some(ref publisher) = self.event_publisher
            && let Err(e) = publisher.publish(event)
        {
            tracing::error!("Failed to publish event: {}", e);
        }
    }

    /// Write one audit-log entry for `action` (`"CREATE"` / `"UPDATE"`
    /// / `"DELETE"`) on the given entity, dispatching to the matching
    /// [`AuditLogRepository`](super::audit::AuditLogRepository) method.
    /// A no-op when no audit log is attached; any log error is traced
    /// and swallowed so audit failures never abort the write.
    async fn log_audit(
        &self,
        action: &str,
        entity_id: Uuid,
        old_values: Option<serde_json::Value>,
        new_values: Option<serde_json::Value>,
        context: &AuditContext,
    ) {
        let Some(ref audit_log) = self.audit_log else {
            return;
        };
        let result = match action {
            "CREATE" => {
                audit_log
                    .log_create(
                        "Event",
                        entity_id,
                        new_values.unwrap_or(serde_json::Value::Null),
                        context,
                    )
                    .await
            }
            "UPDATE" => {
                audit_log
                    .log_update(
                        "Event",
                        entity_id,
                        old_values.unwrap_or(serde_json::Value::Null),
                        new_values.unwrap_or(serde_json::Value::Null),
                        context,
                    )
                    .await
            }
            "DELETE" => {
                audit_log
                    .log_delete(
                        "Event",
                        entity_id,
                        old_values.unwrap_or(serde_json::Value::Null),
                        context,
                    )
                    .await
            }
            _ => Ok(()),
        };
        if let Err(e) = result {
            tracing::error!("Failed to log audit: {}", e);
        }
    }
}

// ---------------------------------------------------------------------------
// Conversions: domain Event ↔ rows
// ---------------------------------------------------------------------------

/// One [`Event`] flattened into its parent row plus all child-table
/// rows, ready to insert under a single transaction.
struct ChildRows {
    /// The parent `events` row.
    event_row: EventRow,
    /// `event_identifiers` rows (one per external identifier).
    identifiers: Vec<IdentifierRow>,
    /// `event_locations` rows (one per [`Location`] in order).
    locations: Vec<LocationRow>,
    /// `event_parties` rows across all six role lists.
    parties: Vec<PartyRow>,
    /// `event_offers` rows (one per [`Offer`] in order).
    offers: Vec<OfferRow>,
    /// `event_links` rows (cross-event links).
    links: Vec<LinkRow>,
    /// `event_sub_events` rows (sub-event id references in order).
    sub_events: Vec<SubEventRow>,
    /// `event_text_values` rows (`alternate_name/image/same_as/keyword/in_language`).
    text_values: Vec<TextValueRow>,
}

/// All child-table rows loaded back for one event, in fixed order
/// (identifiers, locations, parties, offers, links, sub-events,
/// text-values), ready to feed into [`from_rows`].
struct LoadedChildRows {
    /// `event_identifiers` rows.
    identifiers: Vec<IdentifierRow>,
    /// `event_locations` rows.
    locations: Vec<LocationRow>,
    /// `event_parties` rows across all six role lists.
    parties: Vec<PartyRow>,
    /// `event_offers` rows.
    offers: Vec<OfferRow>,
    /// `event_links` rows.
    links: Vec<LinkRow>,
    /// `event_sub_events` rows.
    sub_events: Vec<SubEventRow>,
    /// `event_text_values` rows (ordered by `position`).
    text_values: Vec<TextValueRow>,
}

/// Build the parent `events` [`EventRow`] for a fresh insert, stamping
/// `created_at`/`updated_at` to `now`.
fn event_row(event: &Event, now: DateTime<Utc>) -> EventRow {
    // No digests here. They cover the record as stored, which only exists
    // once the row and its children are written, so the write path stamps
    // them afterwards in the same transaction (`stamp_digests`, EVT-V1).
    EventRow {
        id: event.id,
        content_hash: None,
        content_hash_sha3: None,
        content_mac: None,
        active: event.active,
        name: event.name.clone(),
        description: event.description.clone(),
        disambiguating_description: event.disambiguating_description.clone(),
        url: event.url.clone(),
        start_date: event.start_date,
        end_date: event.end_date,
        door_time: event.door_time,
        duration: event.duration.clone(),
        previous_start_date: event.previous_start_date,
        time_zone: event.time_zone.clone(),
        all_day: event.all_day,
        event_status: enum_to_str(&event.event_status),
        event_attendance_mode: enum_to_str(&event.event_attendance_mode),
        event_type: enum_to_str(&event.event_type),
        typical_age_range: event.typical_age_range.clone(),
        is_accessible_for_free: event.is_accessible_for_free,
        // Capacities are `u32` in the domain but stored as Postgres
        // `i32`; values above `i32::MAX` would wrap (not expected for
        // real attendee counts). Read-back casts `i32` → `u32`.
        maximum_attendee_capacity: event.maximum_attendee_capacity.map(u32::cast_signed),
        maximum_physical_attendee_capacity: event
            .maximum_physical_attendee_capacity
            .map(u32::cast_signed),
        maximum_virtual_attendee_capacity: event
            .maximum_virtual_attendee_capacity
            .map(u32::cast_signed),
        remaining_attendee_capacity: event.remaining_attendee_capacity.map(u32::cast_signed),
        super_event_id: event.super_event,
        created_at: now,
        updated_at: now,
        created_by: None,
        updated_by: None,
        deleted_at: None,
        deleted_by: None,
    }
}

/// Build the `event_offers` rows (one per [`Offer`] in order). Price is
/// parsed from its decimal-string form into `BigDecimal`; an
/// unparseable string stores no price.
fn offer_rows(event: &Event, now: DateTime<Utc>) -> Vec<OfferRow> {
    event
        .offers
        .iter()
        .enumerate()
        .map(|(pos, o)| OfferRow {
            id: Uuid::new_v4(),
            event_id: event.id,
            position: i32::try_from(pos).unwrap_or(i32::MAX),
            name: o.name.clone(),
            price: o
                .price
                .as_deref()
                .and_then(|s| s.parse::<bigdecimal::BigDecimal>().ok()),
            price_currency: o.price_currency.clone(),
            url: o.url.clone(),
            availability: o.availability.as_ref().map(enum_to_str),
            valid_from: o.valid_from,
            valid_through: o.valid_through,
            created_at: now,
            updated_at: now,
        })
        .collect()
}

/// Build the tagged `event_text_values` rows from the five string-list
/// properties (`alternate_name`/`image`/`same_as`/`keyword`/`in_language`).
fn text_value_rows(event: &Event) -> Vec<TextValueRow> {
    let mut text_values = Vec::new();
    for (field, values) in [
        ("alternate_name", &event.alternate_names),
        ("image", &event.image),
        ("same_as", &event.same_as),
        ("keyword", &event.keywords),
        ("in_language", &event.in_language),
    ] {
        for (pos, value) in values.iter().enumerate() {
            text_values.push(TextValueRow {
                id: Uuid::new_v4(),
                event_id: event.id,
                field: field.to_string(),
                value: value.clone(),
                position: i32::try_from(pos).unwrap_or(i32::MAX),
            });
        }
    }
    text_values
}

/// Flatten a domain [`Event`] into a [`ChildRows`] bundle of rows.
/// Stamps `created_at`/`updated_at` to "now"; mints fresh `Uuid`s for
/// child rows; and fans the six party role-lists out via
/// [`push_party_rows`].
fn to_rows(event: &Event) -> ChildRows {
    let now = Utc::now();
    let event_row = event_row(event, now);

    let identifiers = event
        .identifiers
        .iter()
        .map(|id| IdentifierRow {
            id: Uuid::new_v4(),
            event_id: event.id,
            use_type: id.use_type.as_ref().map(enum_to_str),
            // The `Display` form (`BOOKING_NUMBER`), which the CHECK
            // constraint names; the serde form (`BOOKINGNUMBER`) is refused.
            identifier_type: id.identifier_type.to_string(),
            system: id.system.clone(),
            value: id.value.clone(),
            assigner: id.assigner.clone(),
            created_at: now,
            updated_at: now,
        })
        .collect();

    let locations = event
        .location
        .iter()
        .enumerate()
        .map(|(pos, loc)| {
            location_to_row(event.id, i32::try_from(pos).unwrap_or(i32::MAX), loc, now)
        })
        .collect();

    let mut parties = Vec::new();
    push_party_rows(&mut parties, event.id, "organizer", &event.organizers, now);
    push_party_rows(&mut parties, event.id, "performer", &event.performers, now);
    push_party_rows(&mut parties, event.id, "attendee", &event.attendees, now);
    push_party_rows(&mut parties, event.id, "sponsor", &event.sponsors, now);
    push_party_rows(&mut parties, event.id, "funder", &event.funders, now);
    push_party_rows(
        &mut parties,
        event.id,
        "contributor",
        &event.contributors,
        now,
    );

    let offers = offer_rows(event, now);

    let links = event
        .links
        .iter()
        .map(|link| LinkRow {
            id: Uuid::new_v4(),
            event_id: event.id,
            other_event_id: link.other_event_id,
            link_type: enum_to_str(&link.link_type),
            created_at: now,
            created_by: None,
        })
        .collect();

    let sub_events = event
        .sub_events
        .iter()
        .enumerate()
        .map(|(pos, sub_id)| SubEventRow {
            id: Uuid::new_v4(),
            event_id: event.id,
            sub_event_id: *sub_id,
            position: i32::try_from(pos).unwrap_or(i32::MAX),
            created_at: now,
        })
        .collect();

    // String-list properties → tagged `event_text_values` rows.
    let text_values = text_value_rows(event);

    ChildRows {
        event_row,
        identifiers,
        locations,
        parties,
        offers,
        links,
        sub_events,
        text_values,
    }
}

/// Append one `event_parties` row per [`Party`] in `parties`, tagging
/// each with `role` (e.g. `"organizer"`) and its 0-based `position`
/// so insertion order can be restored on read.
fn push_party_rows(
    rows: &mut Vec<PartyRow>,
    event_id: Uuid,
    role: &str,
    parties: &[Party],
    now: DateTime<Utc>,
) {
    for (pos, p) in parties.iter().enumerate() {
        rows.push(PartyRow {
            id: Uuid::new_v4(),
            event_id,
            position: i32::try_from(pos).unwrap_or(i32::MAX),
            role: role.to_string(),
            party_kind: enum_to_str(&p.kind),
            party_id: p.id,
            name: p.name.clone(),
            email: p.email.clone(),
            url: p.url.clone(),
            created_at: now,
            updated_at: now,
        });
    }
}

/// Flatten one [`Location`] union variant into a single
/// `event_locations` row. The `kind` column (`"place"` /
/// `"postal_address"` / `"virtual"` / `"text"`) records which variant
/// was stored so [`row_to_location`] can reconstruct it; columns not
/// relevant to the variant are left `None`.
fn location_to_row(event_id: Uuid, pos: i32, loc: &Location, now: DateTime<Utc>) -> LocationRow {
    let mut row = LocationRow {
        id: Uuid::new_v4(),
        event_id,
        position: pos,
        kind: "text".into(),
        place_id: None,
        name: None,
        line1: None,
        line2: None,
        city: None,
        state: None,
        postal_code: None,
        country: None,
        latitude_as_decimal_degrees: None,
        longitude_as_decimal_degrees: None,
        url: None,
        created_at: now,
        updated_at: now,
    };
    match loc {
        Location::Place(p) => {
            row.kind = "place".into();
            row.place_id = p.id;
            row.name = Some(p.name.clone());
            if let Some(addr) = &p.address {
                row.line1.clone_from(&addr.line1);
                row.line2.clone_from(&addr.line2);
                row.city.clone_from(&addr.city);
                row.state.clone_from(&addr.state);
                row.postal_code.clone_from(&addr.postal_code);
                row.country.clone_from(&addr.country);
            }
            row.latitude_as_decimal_degrees
                .clone_from(&p.latitude_as_decimal_degrees);
            row.longitude_as_decimal_degrees
                .clone_from(&p.longitude_as_decimal_degrees);
            row.url.clone_from(&p.url);
        }
        Location::PostalAddress(addr) => {
            row.kind = "postal_address".into();
            row.line1.clone_from(&addr.line1);
            row.line2.clone_from(&addr.line2);
            row.city.clone_from(&addr.city);
            row.state.clone_from(&addr.state);
            row.postal_code.clone_from(&addr.postal_code);
            row.country.clone_from(&addr.country);
        }
        Location::Virtual(v) => {
            row.kind = "virtual".into();
            row.name.clone_from(&v.name);
            row.url = Some(v.url.clone());
        }
        Location::Text { value } => {
            row.kind = "text".into();
            row.name = Some(value.clone());
        }
    }
    row
}

/// Rebuild a domain [`Event`] from its parent row and the six child-row
/// vectors. Child rows are sorted by their stored `position` to restore
/// insertion order; JSONB arrays are deserialized; parties are bucketed
/// back into the six role lists by their `role` column; and `about` /
/// `works` are reset to empty (not yet persisted as child tables).
/// Project the `event_text_values` rows for `field` into an ordered
/// `Vec<String>` (rows already arrive ordered by `position`).
fn text_values_of(rows: &[TextValueRow], field: &str) -> Vec<String> {
    rows.iter()
        .filter(|r| r.field == field)
        .map(|r| r.value.clone())
        .collect()
}

/// Reassemble the six party role-lists from the flat `event_parties`
/// rows, keyed by `role` and ordered by `position`.
fn parties_by_role(mut parties: Vec<PartyRow>) -> std::collections::HashMap<String, Vec<Party>> {
    parties.sort_by_key(|p| p.position);
    let mut by_role: std::collections::HashMap<String, Vec<Party>> =
        std::collections::HashMap::new();
    for p in parties {
        let kind = if p.party_kind == "organization" {
            PartyKind::Organization
        } else {
            PartyKind::Person
        };
        by_role.entry(p.role.clone()).or_default().push(Party {
            kind,
            id: p.party_id,
            name: p.name,
            email: p.email,
            url: p.url,
        });
    }
    by_role
}

/// Map `event_identifiers` rows back into domain [`Identifier`]s.
fn identifiers_from_rows(rows: Vec<IdentifierRow>) -> Vec<Identifier> {
    rows.into_iter()
        .map(|id| Identifier {
            use_type: id.use_type.as_deref().and_then(parse_identifier_use),
            identifier_type: parse_identifier_type(&id.identifier_type),
            system: id.system,
            value: id.value,
            assigner: id.assigner,
        })
        .collect()
}

/// Sort `event_locations` rows by `position` and map them back into
/// domain [`Location`] variants, dropping unrecognized rows.
fn locations_from_rows(mut rows: Vec<LocationRow>) -> Vec<Location> {
    rows.sort_by_key(|l| l.position);
    rows.iter().filter_map(row_to_location).collect()
}

/// Sort `event_offers` rows by `position` and map them back into domain
/// [`Offer`]s.
fn offers_from_rows(mut rows: Vec<OfferRow>) -> Vec<Offer> {
    rows.sort_by_key(|o| o.position);
    rows.into_iter()
        .map(|o| Offer {
            name: o.name,
            price: o.price.as_ref().map(std::string::ToString::to_string),
            price_currency: o.price_currency,
            url: o.url,
            availability: o.availability.as_deref().and_then(parse_offer_availability),
            valid_from: o.valid_from,
            valid_through: o.valid_through,
        })
        .collect()
}

/// Map `event_links` rows back into domain [`EventLink`]s.
fn links_from_rows(rows: Vec<LinkRow>) -> Vec<EventLink> {
    rows.into_iter()
        .map(|l| EventLink {
            other_event_id: l.other_event_id,
            link_type: str_to_enum(&l.link_type, LinkType::Seealso),
        })
        .collect()
}

/// Sort `event_sub_events` rows by `position` and project their ids.
fn sub_event_ids_from_rows(mut rows: Vec<SubEventRow>) -> Vec<Uuid> {
    rows.sort_by_key(|s| s.position);
    rows.into_iter().map(|s| s.sub_event_id).collect()
}

fn from_rows(event_row: EventRow, children: LoadedChildRows) -> Event {
    let LoadedChildRows {
        identifiers,
        locations,
        parties,
        offers,
        links,
        sub_events,
        text_values,
    } = children;

    let alternate_names = text_values_of(&text_values, "alternate_name");
    let image = text_values_of(&text_values, "image");
    let same_as = text_values_of(&text_values, "same_as");
    let keywords = text_values_of(&text_values, "keyword");
    let in_language = text_values_of(&text_values, "in_language");

    let event_status = str_to_enum(&event_row.event_status, EventStatus::Scheduled);
    let event_attendance_mode = str_to_enum(
        &event_row.event_attendance_mode,
        EventAttendanceMode::Offline,
    );
    let event_type = str_to_enum(&event_row.event_type, EventType::Generic);

    let identifiers = identifiers_from_rows(identifiers);
    let location = locations_from_rows(locations);
    let mut by_role = parties_by_role(parties);
    let offers = offers_from_rows(offers);
    let links = links_from_rows(links);
    let sub_events_ids = sub_event_ids_from_rows(sub_events);

    Event {
        id: event_row.id,
        identifiers,
        active: event_row.active,
        name: event_row.name,
        alternate_names,
        description: event_row.description,
        disambiguating_description: event_row.disambiguating_description,
        url: event_row.url,
        image,
        same_as,
        keywords,
        start_date: event_row.start_date,
        end_date: event_row.end_date,
        door_time: event_row.door_time,
        duration: event_row.duration,
        previous_start_date: event_row.previous_start_date,
        time_zone: event_row.time_zone,
        all_day: event_row.all_day,
        event_status,
        event_attendance_mode,
        event_type,
        typical_age_range: event_row.typical_age_range,
        in_language,
        is_accessible_for_free: event_row.is_accessible_for_free,
        maximum_attendee_capacity: event_row.maximum_attendee_capacity.map(i32::cast_unsigned),
        maximum_physical_attendee_capacity: event_row
            .maximum_physical_attendee_capacity
            .map(i32::cast_unsigned),
        maximum_virtual_attendee_capacity: event_row
            .maximum_virtual_attendee_capacity
            .map(i32::cast_unsigned),
        remaining_attendee_capacity: event_row
            .remaining_attendee_capacity
            .map(i32::cast_unsigned),
        location,
        organizers: by_role.remove("organizer").unwrap_or_default(),
        performers: by_role.remove("performer").unwrap_or_default(),
        attendees: by_role.remove("attendee").unwrap_or_default(),
        sponsors: by_role.remove("sponsor").unwrap_or_default(),
        funders: by_role.remove("funder").unwrap_or_default(),
        contributors: by_role.remove("contributor").unwrap_or_default(),
        about: Vec::new(),
        works: Vec::new(),
        super_event: event_row.super_event_id,
        sub_events: sub_events_ids,
        offers,
        links,
        created_at: event_row.created_at,
        updated_at: event_row.updated_at,
    }
}

/// Reconstruct a [`Location`] union variant from one `event_locations`
/// row, dispatching on the `kind` column. Returns `None` for an
/// unrecognized `kind` or a `text` row missing its value, so callers
/// `filter_map` over the results.
fn row_to_location(row: &LocationRow) -> Option<Location> {
    match row.kind.as_str() {
        "place" => Some(Location::Place(Place {
            id: row.place_id,
            name: row.name.clone().unwrap_or_default(),
            address: address_from_row(row),
            latitude_as_decimal_degrees: row.latitude_as_decimal_degrees.clone(),
            longitude_as_decimal_degrees: row.longitude_as_decimal_degrees.clone(),
            url: row.url.clone(),
        })),
        "postal_address" => address_from_row(row).map(Location::PostalAddress),
        "virtual" => Some(Location::Virtual(VirtualLocation {
            name: row.name.clone(),
            url: row.url.clone().unwrap_or_default(),
        })),
        "text" => row.name.clone().map(|value| Location::Text { value }),
        _ => None,
    }
}

/// Assemble an [`Address`] from the address columns of an
/// `event_locations` row, returning `None` when every address column
/// is empty (so a bare place/virtual row doesn't yield a blank address).
fn address_from_row(row: &LocationRow) -> Option<Address> {
    let any = row.line1.is_some()
        || row.line2.is_some()
        || row.city.is_some()
        || row.state.is_some()
        || row.postal_code.is_some()
        || row.country.is_some();
    if !any {
        return None;
    }
    Some(Address {
        use_type: None,
        line1: row.line1.clone(),
        line2: row.line2.clone(),
        city: row.city.clone(),
        state: row.state.clone(),
        postal_code: row.postal_code.clone(),
        country: row.country.clone(),
    })
}

/// Serialize a serde enum to its string column representation by
/// round-tripping through [`serde_json`] and taking the JSON string
/// body; non-string serializations collapse to `""`.
fn enum_to_str<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(std::string::ToString::to_string))
        .unwrap_or_default()
}

/// Inverse of [`enum_to_str`]: deserialize a column string back into a
/// serde enum, falling back to `default` on any unrecognized value.
fn str_to_enum<T: serde::de::DeserializeOwned>(s: &str, default: T) -> T {
    serde_json::from_value::<T>(serde_json::Value::String(s.to_string())).unwrap_or(default)
}

/// Parse an `identifier_type` column string, stored in its `Display`
/// form, into an [`IdentifierType`], defaulting to
/// [`IdentifierType::Other`] for unknown values.
fn parse_identifier_type(s: &str) -> IdentifierType {
    IdentifierType::from_stored(s).unwrap_or(IdentifierType::Other)
}

/// Parse a `use_type` column string into an optional [`IdentifierUse`];
/// `None` when the value is absent or unrecognized.
fn parse_identifier_use(s: &str) -> Option<IdentifierUse> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
}

/// Parse an `availability` column string into an optional
/// [`OfferAvailability`]; `None` when absent or unrecognized.
fn parse_offer_availability(s: &str) -> Option<OfferAvailability> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
}

/// `INSERT` one `events` row, every column, and read it back as stored
/// (`RETURNING`), so the caller rebuilds the domain event from the
/// database's own values.
async fn insert_event_row(conn: &mut PgConnection, row: &EventRow) -> Result<EventRow> {
    let stored = sqlx::query_as!(
        EventRow,
        "INSERT INTO events
             (id, active, name, content_hash, content_hash_sha3, content_mac, description,
              disambiguating_description, url, start_date, end_date, door_time, duration,
              previous_start_date, time_zone, all_day, event_status, event_attendance_mode,
              event_type, typical_age_range, is_accessible_for_free, maximum_attendee_capacity,
              maximum_physical_attendee_capacity, maximum_virtual_attendee_capacity,
              remaining_attendee_capacity, super_event_id, created_at, updated_at, created_by,
              updated_by, deleted_at, deleted_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                 $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32)
         RETURNING id, active, name, content_hash, content_hash_sha3, content_mac, description,
                   disambiguating_description, url, start_date, end_date, door_time, duration,
                   previous_start_date, time_zone, all_day, event_status,
                   event_attendance_mode, event_type, typical_age_range,
                   is_accessible_for_free, maximum_attendee_capacity,
                   maximum_physical_attendee_capacity, maximum_virtual_attendee_capacity,
                   remaining_attendee_capacity, super_event_id, created_at, updated_at,
                   created_by, updated_by, deleted_at, deleted_by",
        row.id,
        row.active,
        row.name,
        row.content_hash,
        row.content_hash_sha3,
        row.content_mac,
        row.description,
        row.disambiguating_description,
        row.url,
        row.start_date,
        row.end_date,
        row.door_time,
        row.duration,
        row.previous_start_date,
        row.time_zone,
        row.all_day,
        row.event_status,
        row.event_attendance_mode,
        row.event_type,
        row.typical_age_range,
        row.is_accessible_for_free,
        row.maximum_attendee_capacity,
        row.maximum_physical_attendee_capacity,
        row.maximum_virtual_attendee_capacity,
        row.remaining_attendee_capacity,
        row.super_event_id,
        row.created_at,
        row.updated_at,
        row.created_by,
        row.updated_by,
        row.deleted_at,
        row.deleted_by,
    )
    .fetch_one(conn)
    .await?;
    Ok(stored)
}

/// `INSERT` every child row of one event, table by table in the fixed
/// order identifiers, locations, parties, offers, links, sub-events,
/// text-values, on the caller's connection/transaction.
async fn insert_children(conn: &mut PgConnection, rows: &ChildRows) -> Result<()> {
    for r in &rows.identifiers {
        sqlx::query!(
            "INSERT INTO event_identifiers
                 (id, event_id, use_type, identifier_type, system, value, assigner,
                  created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            r.id,
            r.event_id,
            r.use_type,
            r.identifier_type,
            r.system,
            r.value,
            r.assigner,
            r.created_at,
            r.updated_at,
        )
        .execute(&mut *conn)
        .await?;
    }
    for r in &rows.locations {
        sqlx::query!(
            "INSERT INTO event_locations
                 (id, event_id, position, kind, place_id, name, line1, line2, city, state,
                  postal_code, country, latitude_as_decimal_degrees,
                  longitude_as_decimal_degrees, url, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                     $17)",
            r.id,
            r.event_id,
            r.position,
            r.kind,
            r.place_id,
            r.name,
            r.line1,
            r.line2,
            r.city,
            r.state,
            r.postal_code,
            r.country,
            r.latitude_as_decimal_degrees,
            r.longitude_as_decimal_degrees,
            r.url,
            r.created_at,
            r.updated_at,
        )
        .execute(&mut *conn)
        .await?;
    }
    for r in &rows.parties {
        sqlx::query!(
            "INSERT INTO event_parties
                 (id, event_id, position, role, party_kind, party_id, name, email, url,
                  created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            r.id,
            r.event_id,
            r.position,
            r.role,
            r.party_kind,
            r.party_id,
            r.name,
            r.email,
            r.url,
            r.created_at,
            r.updated_at,
        )
        .execute(&mut *conn)
        .await?;
    }
    insert_remaining_children(conn, rows).await
}

/// The second half of [`insert_children`]: offers, links, sub-events and
/// text-values, in that order.
async fn insert_remaining_children(conn: &mut PgConnection, rows: &ChildRows) -> Result<()> {
    for r in &rows.offers {
        sqlx::query!(
            "INSERT INTO event_offers
                 (id, event_id, position, name, price, price_currency, url, availability,
                  valid_from, valid_through, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            r.id,
            r.event_id,
            r.position,
            r.name,
            r.price,
            r.price_currency,
            r.url,
            r.availability,
            r.valid_from,
            r.valid_through,
            r.created_at,
            r.updated_at,
        )
        .execute(&mut *conn)
        .await?;
    }
    for r in &rows.links {
        sqlx::query!(
            "INSERT INTO event_links (id, event_id, other_event_id, link_type, created_at, created_by)
             VALUES ($1, $2, $3, $4, $5, $6)",
            r.id,
            r.event_id,
            r.other_event_id,
            r.link_type,
            r.created_at,
            r.created_by,
        )
        .execute(&mut *conn)
        .await?;
    }
    for r in &rows.sub_events {
        sqlx::query!(
            "INSERT INTO event_sub_events (id, event_id, sub_event_id, position, created_at)
             VALUES ($1, $2, $3, $4, $5)",
            r.id,
            r.event_id,
            r.sub_event_id,
            r.position,
            r.created_at,
        )
        .execute(&mut *conn)
        .await?;
    }
    for r in &rows.text_values {
        sqlx::query!(
            "INSERT INTO event_text_values (id, event_id, field, value, position)
             VALUES ($1, $2, $3, $4, $5)",
            r.id,
            r.event_id,
            r.field,
            r.value,
            r.position,
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Apply an event's parent-row update and wholesale child-row
/// replacement on `conn`. Shared by [`EventRepository::update`] and
/// [`EventRepository::merge`]: preserves `created_*`, stamps
/// `updated_at` to now, deletes every child row for the event, then
/// re-inserts them from the domain model. Runs on the caller's
/// connection/transaction so it shares that commit boundary.
///
/// Every other column is written, `deleted_at`, `deleted_by` and
/// `updated_by` included (all `NULL` from [`event_row`]), exactly as the
/// whole-row update this replaced did.
async fn apply_update_rows(conn: &mut PgConnection, event: &Event) -> Result<()> {
    // Update the events row (preserving created_*; updating updated_*).
    let rows = to_rows(event);
    let row = &rows.event_row;
    let done = sqlx::query!(
        "UPDATE events SET
             active = $2, name = $3, content_hash = $4, content_hash_sha3 = $5,
             content_mac = $6, description = $7, disambiguating_description = $8, url = $9,
             start_date = $10, end_date = $11, door_time = $12, duration = $13,
             previous_start_date = $14, time_zone = $15, all_day = $16, event_status = $17,
             event_attendance_mode = $18, event_type = $19, typical_age_range = $20,
             is_accessible_for_free = $21, maximum_attendee_capacity = $22,
             maximum_physical_attendee_capacity = $23,
             maximum_virtual_attendee_capacity = $24, remaining_attendee_capacity = $25,
             super_event_id = $26, updated_at = $27, updated_by = $28, deleted_at = $29,
             deleted_by = $30
         WHERE id = $1",
        row.id,
        row.active,
        row.name,
        row.content_hash,
        row.content_hash_sha3,
        row.content_mac,
        row.description,
        row.disambiguating_description,
        row.url,
        row.start_date,
        row.end_date,
        row.door_time,
        row.duration,
        row.previous_start_date,
        row.time_zone,
        row.all_day,
        row.event_status,
        row.event_attendance_mode,
        row.event_type,
        row.typical_age_range,
        row.is_accessible_for_free,
        row.maximum_attendee_capacity,
        row.maximum_physical_attendee_capacity,
        row.maximum_virtual_attendee_capacity,
        row.remaining_attendee_capacity,
        row.super_event_id,
        Utc::now(),
        row.updated_by,
        row.deleted_at,
        row.deleted_by,
    )
    .execute(&mut *conn)
    .await?;
    // A row that does not exist is an error, worded as the ORM's
    // "no record updated" was.
    if done.rows_affected() == 0 {
        return Err(not_updated());
    }

    // Replace child rows wholesale.
    sqlx::query!(
        "DELETE FROM event_identifiers WHERE event_id = $1",
        event.id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!("DELETE FROM event_locations WHERE event_id = $1", event.id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM event_parties WHERE event_id = $1", event.id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM event_offers WHERE event_id = $1", event.id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM event_links WHERE event_id = $1", event.id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM event_sub_events WHERE event_id = $1", event.id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!(
        "DELETE FROM event_text_values WHERE event_id = $1",
        event.id
    )
    .execute(&mut *conn)
    .await?;

    insert_children(conn, &rows).await?;
    // Digests over the record as stored, in the caller's transaction
    // (EVT-V1). `update` and `merge` both come through here.
    PgEventRepository::stamp_digests(conn, &event.id).await
}

/// Lock a merge's two rows `FOR UPDATE`, in id order so two merges of the
/// same pair in opposite directions cannot deadlock, then require both to
/// be active. Returns [`crate::Error::Conflict`] when either was deleted
/// since the caller read it, and the caller's transaction then writes
/// nothing.
///
/// # Errors
/// [`crate::Error::Conflict`] as above, or a database error.
async fn lock_merge_participants(
    conn: &mut PgConnection,
    survivor_id: Uuid,
    duplicate_id: Uuid,
) -> Result<()> {
    let mut ids = [survivor_id, duplicate_id];
    ids.sort();
    for id in ids {
        sqlx::query_scalar!("SELECT id FROM events WHERE id = $1 FOR UPDATE", id)
            .fetch_optional(&mut *conn)
            .await?;
    }
    let active = sqlx::query_scalar!(
        r#"SELECT count(*) AS "active!" FROM events WHERE id = ANY($1) AND deleted_at IS NULL"#,
        &ids[..],
    )
    .fetch_one(&mut *conn)
    .await?;
    if active == 2 {
        Ok(())
    } else {
        Err(crate::Error::Conflict(format!(
            "event {survivor_id} or {duplicate_id} is no longer active (merged or deleted meanwhile)"
        )))
    }
}

/// Soft-delete the event `id` on `conn` by stamping `deleted_at` /
/// `deleted_by` and leaving every other column (and the child rows)
/// in place. Shared by [`EventRepository::delete`] and
/// [`EventRepository::merge`], which runs it on its own transaction.
async fn apply_soft_delete_row(conn: &mut PgConnection, id: &Uuid) -> Result<()> {
    let done = sqlx::query!(
        "UPDATE events SET deleted_at = $2, deleted_by = 'system' WHERE id = $1",
        id,
        Utc::now(),
    )
    .execute(conn)
    .await?;
    // As under the ORM, an update that touches no row is an error.
    if done.rows_affected() == 0 {
        return Err(not_updated());
    }
    Ok(())
}

/// The error an update or soft delete that matched no row returns,
/// with the message the ORM used for it.
fn not_updated() -> crate::Error {
    crate::Error::Database("None of the records are updated".into())
}

/// A `LIMIT` / `OFFSET` bound: Postgres takes `BIGINT`, the callers count
/// in `u64`.
fn to_i64(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// The `events` rows `GET /api/records/verify` reassembles and
/// recomputes, newest update first, soft-deleted ones included.
///
/// # Errors
///
/// Returns the database error if the query fails.
pub async fn rows_for_verification(db: &PgPool, limit: u64) -> Result<Vec<EventRow>> {
    let rows = sqlx::query_as!(
        EventRow,
        "SELECT id, active, name, content_hash, content_hash_sha3, content_mac, description,
                disambiguating_description, url, start_date, end_date, door_time, duration,
                previous_start_date, time_zone, all_day, event_status, event_attendance_mode,
                event_type, typical_age_range, is_accessible_for_free,
                maximum_attendee_capacity, maximum_physical_attendee_capacity,
                maximum_virtual_attendee_capacity, remaining_attendee_capacity,
                super_event_id, created_at, updated_at, created_by, updated_by, deleted_at,
                deleted_by
         FROM events
         ORDER BY updated_at DESC
         LIMIT $1",
        to_i64(limit),
    )
    .fetch_all(db)
    .await?;
    Ok(rows)
}

// ---------------------------------------------------------------------------
// EventRepository impl
// ---------------------------------------------------------------------------

impl PgEventRepository {
    /// Load all child-row vectors for one event id in fixed order
    /// (identifiers, locations, parties, offers, links, sub-events,
    /// text-values), ready to feed into [`from_rows`]. The text-values
    /// query is ordered by `position` so list order is preserved; the
    /// other child rows are sorted inside [`from_rows`].
    ///
    /// # Errors
    ///
    /// Returns the database error if any of the per-child-table queries
    /// fails.
    /// The assembled record for `id` as stored, read on `conn`, or `None`
    /// when it is absent or soft-deleted. `get_by_id` and the digest stamp
    /// both read through this, so the record a digest covers is exactly
    /// the record `GET /api/records/verify` reassembles.
    async fn read_event(conn: &mut PgConnection, id: &Uuid) -> Result<Option<Event>> {
        // Soft-delete aware: a non-null `deleted_at` hides the row, so a
        // tombstoned event reads back as `None`.
        let event_row = sqlx::query_as!(
            EventRow,
            "SELECT id, active, name, content_hash, content_hash_sha3, content_mac,
                    description, disambiguating_description, url, start_date, end_date,
                    door_time, duration, previous_start_date, time_zone, all_day,
                    event_status, event_attendance_mode, event_type, typical_age_range,
                    is_accessible_for_free, maximum_attendee_capacity,
                    maximum_physical_attendee_capacity, maximum_virtual_attendee_capacity,
                    remaining_attendee_capacity, super_event_id, created_at, updated_at,
                    created_by, updated_by, deleted_at, deleted_by
             FROM events
             WHERE id = $1 AND deleted_at IS NULL",
            id,
        )
        .fetch_optional(&mut *conn)
        .await?;
        let Some(event_row) = event_row else {
            return Ok(None);
        };
        let children = Self::load_children(conn, id).await?;
        Ok(Some(from_rows(event_row, children)))
    }

    /// Stamp `id`'s three digests over the record **as stored**, on the
    /// write's own connection, so they commit with the write.
    ///
    /// Digesting the domain value a caller passed in is what made every
    /// fresh record fail verification (tasks.md EVT-V1): the stored row
    /// takes the server's timestamps, and Postgres keeps microseconds, so
    /// the value read back was never the value hashed. Reading the record
    /// back inside the transaction makes the digest cover what
    /// verification will recompute, whatever the database normalised.
    ///
    /// # Errors
    /// When the record cannot be read back or the stamp fails.
    async fn stamp_digests(conn: &mut PgConnection, id: &Uuid) -> Result<()> {
        let mut stored = Self::read_event(conn, id).await?.ok_or_else(|| {
            crate::Error::Validation("event not found while stamping digests".into())
        })?;
        // The stamp is itself an UPDATE, so the `update_events_updated_at`
        // trigger sets `updated_at` to `CURRENT_TIMESTAMP` as it lands.
        // That value is fixed for the transaction, so the digest covers it,
        // read from the database rather than guessed from this clock.
        let stamped_at = sqlx::query_scalar!(r#"SELECT CURRENT_TIMESTAMP AS "now!""#)
            .fetch_one(&mut *conn)
            .await?;
        stored.updated_at = stamped_at;
        let digests = crate::compliance::record_integrity::digests(
            &crate::compliance::record_integrity::RecordInput {
                id: stored.id,
                event: &stored,
                active: stored.active,
            },
        );
        let written_at = sqlx::query_scalar!(
            "UPDATE events SET content_hash = $2, content_hash_sha3 = $3, content_mac = $4
             WHERE id = $1
             RETURNING updated_at",
            id,
            digests.sha256,
            digests.sha3,
            digests.mac,
        )
        .fetch_one(&mut *conn)
        .await?;
        // Refuse rather than store a digest that would never verify, should
        // the trigger ever write something other than the transaction time.
        if written_at != stamped_at {
            return Err(crate::Error::Validation(format!(
                "event {id}: updated_at became {written_at}, not {stamped_at}, so its digests would not verify"
            )));
        }
        Ok(())
    }

    async fn load_children(conn: &mut PgConnection, event_id: &Uuid) -> Result<LoadedChildRows> {
        let identifiers = sqlx::query_as!(
            IdentifierRow,
            "SELECT id, event_id, use_type, identifier_type, system, value, assigner,
                    created_at, updated_at
             FROM event_identifiers
             WHERE event_id = $1",
            event_id,
        )
        .fetch_all(&mut *conn)
        .await?;
        let locations = sqlx::query_as!(
            LocationRow,
            "SELECT id, event_id, position, kind, place_id, name, line1, line2, city, state,
                    postal_code, country, latitude_as_decimal_degrees,
                    longitude_as_decimal_degrees, url, created_at, updated_at
             FROM event_locations
             WHERE event_id = $1",
            event_id,
        )
        .fetch_all(&mut *conn)
        .await?;
        let parties = sqlx::query_as!(
            PartyRow,
            "SELECT id, event_id, position, role, party_kind, party_id, name, email, url,
                    created_at, updated_at
             FROM event_parties
             WHERE event_id = $1",
            event_id,
        )
        .fetch_all(&mut *conn)
        .await?;
        let offers = sqlx::query_as!(
            OfferRow,
            "SELECT id, event_id, position, name, price, price_currency, url, availability,
                    valid_from, valid_through, created_at, updated_at
             FROM event_offers
             WHERE event_id = $1",
            event_id,
        )
        .fetch_all(&mut *conn)
        .await?;
        let links = sqlx::query_as!(
            LinkRow,
            "SELECT id, event_id, other_event_id, link_type, created_at, created_by
             FROM event_links
             WHERE event_id = $1",
            event_id,
        )
        .fetch_all(&mut *conn)
        .await?;
        let sub_events = sqlx::query_as!(
            SubEventRow,
            "SELECT id, event_id, sub_event_id, position, created_at
             FROM event_sub_events
             WHERE event_id = $1",
            event_id,
        )
        .fetch_all(&mut *conn)
        .await?;
        let text_values = sqlx::query_as!(
            TextValueRow,
            "SELECT id, event_id, field, value, position
             FROM event_text_values
             WHERE event_id = $1
             ORDER BY position ASC",
            event_id,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(LoadedChildRows {
            identifiers,
            locations,
            parties,
            offers,
            links,
            sub_events,
            text_values,
        })
    }
}

#[async_trait::async_trait]
impl EventRepository for PgEventRepository {
    async fn create(&self, event: &Event) -> Result<Event> {
        // All parent + child inserts run under one transaction so a
        // partial failure leaves no orphan child rows.
        let mut txn = self.db.begin().await?;

        // Flatten the domain event into parent + child rows, then insert
        // the parent first (children reference its id).
        let rows = to_rows(event);
        let inserted = insert_event_row(&mut txn, &rows.event_row).await?;
        insert_children(&mut txn, &rows).await?;
        // Digests over the record as stored, in this transaction (EVT-V1).
        Self::stamp_digests(&mut txn, &inserted.id).await?;

        // Durable event bus (Phase 2): under the outbox transport, write
        // the `event_outbox` row **inside this same transaction**, before
        // the commit, so the entity rows and the event commit atomically
        // (or roll back together). A no-op under the memory transport,
        // which keeps the post-commit ring-buffer publish below.
        self.enqueue_outbox(&mut txn, event, EventKind::Created)
            .await?;

        txn.commit().await?;

        // Reload and reassemble the canonical domain `Event` (positions
        // round-tripped) to return to the caller.
        let result = self
            .get_by_id(&inserted.id)
            .await?
            .ok_or_else(|| crate::Error::Validation("Event not found after create".into()))?;

        // Fire-and-forget side effects: publish a `Created` stream event,
        // then record a CREATE audit row with only a `new_values` snapshot.
        self.publish_event(crate::streaming::EventEvent::Created {
            event: result.clone(),
            timestamp: chrono::Utc::now(),
        });
        if let Ok(json) = serde_json::to_value(&result) {
            self.log_audit(
                "CREATE",
                result.id,
                None,
                Some(json),
                &AuditContext::default(),
            )
            .await;
        }
        Ok(result)
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Event>> {
        let mut conn = self.db.acquire().await?;
        Self::read_event(&mut conn, id).await
    }

    async fn update(&self, event: &Event) -> Result<Event> {
        // Snapshot the pre-update state first so the audit log can carry
        // an accurate `old_values` even though children are replaced.
        let old = self.get_by_id(&event.id).await?;
        let mut txn = self.db.begin().await?;

        // Parent-row update + wholesale child-row replacement, shared
        // with `merge` (see `apply_update_rows`).
        apply_update_rows(&mut txn, event).await?;

        // Outbox row shares the update transaction (see `create`).
        self.enqueue_outbox(&mut txn, event, EventKind::Updated)
            .await?;

        txn.commit().await?;

        let result = self
            .get_by_id(&event.id)
            .await?
            .ok_or_else(|| crate::Error::Validation("Event not found after update".into()))?;

        self.publish_event(crate::streaming::EventEvent::Updated {
            event: result.clone(),
            timestamp: chrono::Utc::now(),
        });
        // Record an UPDATE audit row only when both the pre-image and
        // both JSON serializations succeed; otherwise skip the trail
        // rather than write a misleading half-diff.
        if let (Some(old), Ok(new_json)) = (old, serde_json::to_value(&result))
            && let Ok(old_json) = serde_json::to_value(&old)
        {
            self.log_audit(
                "UPDATE",
                result.id,
                Some(old_json),
                Some(new_json),
                &AuditContext::default(),
            )
            .await;
        }
        Ok(result)
    }

    async fn merge(&self, survivor: &Event, duplicate_id: &Uuid) -> Result<Event> {
        // Snapshots for the audit trail: the survivor's pre-image for the
        // UPDATE row, the duplicate's pre-image for the DELETE row (and to
        // build the duplicate's `Deleted` outbox envelope).
        let old_survivor = self.get_by_id(&survivor.id).await?;
        let old_duplicate = self.get_by_id(duplicate_id).await?;

        // The whole merge — survivor row updates + duplicate soft-delete +
        // both outbox rows — commits (or rolls back) under one transaction.
        let mut txn = self.db.begin().await?;

        // SEC-B5: the merged payload was computed from unlocked reads, so
        // lock both rows before writing and check under the lock that both
        // are still active. Otherwise two concurrent merges of one
        // duplicate both apply, and "B into A" racing "A into B" deletes
        // both. The loser gets `Conflict` and writes nothing.
        lock_merge_participants(&mut txn, survivor.id, *duplicate_id).await?;

        // Apply the survivor's parent + child rows (shared with `update`).
        apply_update_rows(&mut txn, survivor).await?;

        // Soft-delete the duplicate (shared with `delete`).
        apply_soft_delete_row(&mut txn, duplicate_id).await?;

        // Durable event bus (Phase 2): a `Merged` outbox row for the
        // survivor (carrying the duplicate's pid via `merged_from`, so a
        // merge-repointing consumer can move edges off the duplicate) and a
        // `Deleted` outbox row for the duplicate — both inside this
        // transaction. A no-op under the memory transport.
        if self.transport.is_outbox() {
            OutboxInsert::for_merge(survivor, duplicate_id)?
                .insert_on(&mut txn)
                .await?;
            if let Some(dup) = old_duplicate.as_ref() {
                OutboxInsert::for_event(dup, EventKind::Deleted)?
                    .insert_on(&mut txn)
                    .await?;
            }
        }

        txn.commit().await?;

        let result = self
            .get_by_id(&survivor.id)
            .await?
            .ok_or_else(|| crate::Error::Validation("Event not found after merge".into()))?;

        // Post-commit in-memory stream (memory transport keeps this): a
        // `Merged` for the survivor and a `Deleted` for the duplicate.
        self.publish_event(crate::streaming::EventEvent::Merged {
            source_id: *duplicate_id,
            target_id: result.id,
            timestamp: chrono::Utc::now(),
        });
        self.publish_event(crate::streaming::EventEvent::Deleted {
            event_id: *duplicate_id,
            timestamp: chrono::Utc::now(),
        });

        // Audit rows, as the old flow did: an UPDATE trail for the survivor
        // and a DELETE trail for the duplicate.
        if let (Some(old), Ok(new_json)) = (old_survivor, serde_json::to_value(&result))
            && let Ok(old_json) = serde_json::to_value(&old)
        {
            self.log_audit(
                "UPDATE",
                result.id,
                Some(old_json),
                Some(new_json),
                &AuditContext::default(),
            )
            .await;
        }
        if let Some(dup) = old_duplicate
            && let Ok(dup_json) = serde_json::to_value(&dup)
        {
            self.log_audit(
                "DELETE",
                *duplicate_id,
                Some(dup_json),
                None,
                &AuditContext::default(),
            )
            .await;
        }
        Ok(result)
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        // Capture the pre-image for the DELETE audit `old_values`.
        let old = self.get_by_id(id).await?;
        // Soft delete: stamp `deleted_at`/`deleted_by` only, a partial
        // UPDATE that leaves the rest of the row untouched and the child
        // rows in place.
        // Unlike create/update, the soft-delete is a single row update
        // with no existing transaction. Under the outbox transport we
        // open one here so the tombstone write and the `deleted` outbox
        // row commit atomically; the memory transport keeps the plain,
        // tx-free update.
        if self.transport.is_outbox() {
            let mut txn = self.db.begin().await?;
            apply_soft_delete_row(&mut txn, id).await?;
            if let Some(old) = old.as_ref() {
                self.enqueue_outbox(&mut txn, old, EventKind::Deleted)
                    .await?;
            }
            txn.commit().await?;
        } else {
            let mut conn = self.db.acquire().await?;
            apply_soft_delete_row(&mut conn, id).await?;
        }
        self.publish_event(crate::streaming::EventEvent::Deleted {
            event_id: *id,
            timestamp: chrono::Utc::now(),
        });
        if let Some(old) = old
            && let Ok(old_json) = serde_json::to_value(&old)
        {
            self.log_audit(
                "DELETE",
                *id,
                Some(old_json),
                None,
                &AuditContext::default(),
            )
            .await;
        }
        Ok(())
    }

    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Event>> {
        let ids = sqlx::query_scalar!(
            "SELECT id FROM events
             WHERE deleted_at IS NULL AND active = true
             LIMIT $1 OFFSET $2",
            to_i64(limit),
            to_i64(offset),
        )
        .fetch_all(&self.db)
        .await?;
        let mut events = Vec::new();
        for id in ids {
            if let Some(e) = self.get_by_id(&id).await? {
                events.push(e);
            }
        }
        Ok(events)
    }

    async fn list_active_names(&self, limit: u64, after: Option<Uuid>) -> Result<Vec<NameRecord>> {
        let rows = sqlx::query!(
            r#"SELECT e.id, e.name,
                      COALESCE(
                          array_agg(t.value ORDER BY t.position)
                              FILTER (WHERE t.value IS NOT NULL),
                          '{}'
                      ) AS "alternate_names!"
               FROM events e
               LEFT JOIN event_text_values t
                 ON t.event_id = e.id AND t.field = 'alternate_name'
               WHERE e.deleted_at IS NULL AND e.active = true
                 AND ($2::uuid IS NULL OR e.id > $2)
               GROUP BY e.id
               ORDER BY e.id
               LIMIT $1"#,
            to_i64(limit),
            after,
        )
        .fetch_all(&self.db)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| NameRecord {
                id: r.id,
                name: r.name,
                alternate_names: r.alternate_names,
            })
            .collect())
    }
}

/// DB-gated (`#[ignore]`) atomicity tests for the outbox write path.
/// They require a migrated `PostgreSQL` via `DATABASE_URL` and are skipped
/// by a bare `cargo test`; run with
/// `DATABASE_URL=… cargo test --lib -- --ignored`. They must COMPILE
/// under a bare `cargo test --lib`.
#[cfg(test)]
mod tests {
    use super::{EventRepository, PgEventRepository};

    use crate::db::outbox;
    use crate::models::Event;
    use crate::streaming::EventTransport;
    use chrono::{TimeZone, Utc};

    async fn connect() -> sqlx::PgPool {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        sqlx::PgPool::connect(&url)
            .await
            .expect("connect to DATABASE_URL")
    }

    /// Merge writes, in one transaction: a `merged` outbox row for the
    /// survivor carrying the duplicate's pid in `merged_from`, plus a
    /// `deleted` outbox row for the duplicate.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn merge_enqueues_merged_with_merged_from_and_deleted() {
        let db = connect().await;
        let repo = PgEventRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let start = Utc.with_ymd_and_hms(2026, 6, 1, 9, 0, 0).unwrap();
        let survivor = repo.create(&Event::new("Survivor", start)).await.unwrap();
        let duplicate = repo.create(&Event::new("Duplicate", start)).await.unwrap();

        let merged = repo.merge(&survivor, &duplicate.id).await.unwrap();
        assert_eq!(merged.id, survivor.id);

        // Survivor: exactly one `merged` row, carrying the duplicate pid.
        let merged_rows = outbox::rows_for(&db, survivor.id, "merged").await.unwrap();
        assert_eq!(merged_rows.len(), 1, "one merged outbox row for survivor");
        assert_eq!(
            merged_rows[0].payload["merged_from"],
            serde_json::json!(duplicate.id.to_string()),
            "merged row carries the duplicate's pid"
        );

        // Duplicate: a `deleted` row.
        let deleted_rows = outbox::rows_for(&db, duplicate.id, "deleted")
            .await
            .unwrap();
        assert_eq!(
            deleted_rows.len(),
            1,
            "one deleted outbox row for duplicate"
        );
    }

    /// SEC-B5: a merge locks both rows and rechecks them under the lock.
    /// Here another transaction is soft-deleting the duplicate and holds
    /// its row; the merge waits for it, then finds the duplicate gone and
    /// answers `Conflict` without writing the survivor. Without the lock
    /// it applied the survivor and "deleted" an already-deleted row.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn a_merge_waits_for_a_concurrent_delete_and_then_refuses() {
        let db = connect().await;
        let repo = PgEventRepository::new(db.clone());
        let start = Utc.with_ymd_and_hms(2026, 6, 1, 9, 0, 0).unwrap();
        let survivor = repo
            .create(&Event::new("Race Survivor", start))
            .await
            .unwrap();
        let duplicate = repo
            .create(&Event::new("Race Duplicate", start))
            .await
            .unwrap();

        let mut other = db.begin().await.unwrap();
        super::apply_soft_delete_row(&mut other, &duplicate.id)
            .await
            .unwrap();

        let mut renamed = survivor.clone();
        renamed.name = "Race Survivor, merged".into();
        let merger = PgEventRepository::new(db.clone());
        let dup_id = duplicate.id;
        let merge = tokio::spawn(async move { merger.merge(&renamed, &dup_id).await });

        // Wait, up to ten seconds, until Postgres reports a backend waiting
        // on a lock held by `other`.
        let mut blocked = false;
        for _ in 0..200 {
            let waiting: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM pg_stat_activity
                 WHERE datname = current_database() AND wait_event_type = 'Lock'",
            )
            .fetch_one(&db)
            .await
            .unwrap();
            if waiting > 0 {
                blocked = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        other.commit().await.unwrap();

        let result = merge.await.unwrap();
        assert!(
            blocked,
            "the merge never waited on the duplicate's row lock"
        );
        assert!(
            matches!(result, Err(crate::Error::Conflict(_))),
            "expected Conflict, got {result:?}"
        );
        let stored = repo.get_by_id(&survivor.id).await.unwrap().unwrap();
        assert_eq!(stored.name, "Race Survivor", "the survivor was not written");
    }

    /// The frequency scan's name listing pages by id, carries alternate
    /// names in order, and skips deleted events.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn list_active_names_pages_by_id_with_alternates() {
        let db = connect().await;
        let repo = PgEventRepository::new(db.clone());
        let start = Utc.with_ymd_and_hms(2026, 6, 1, 9, 0, 0).unwrap();
        let tag = uuid::Uuid::new_v4();
        let mut with_alts = Event::new(format!("Names {tag}"), start);
        with_alts.alternate_names = vec!["First Alt".into(), "Second Alt".into()];
        let a = repo.create(&with_alts).await.unwrap();
        let b = repo
            .create(&Event::new(format!("Names {tag} bare"), start))
            .await
            .unwrap();
        let gone = repo
            .create(&Event::new(format!("Names {tag} gone"), start))
            .await
            .unwrap();
        repo.delete(&gone.id).await.unwrap();

        // Walk every page; ids strictly increase across pages.
        let mut all = Vec::new();
        let mut after = None;
        loop {
            let page = repo.list_active_names(2, after).await.unwrap();
            let Some(last) = page.last() else { break };
            after = Some(last.id);
            all.extend(page);
        }
        assert!(all.windows(2).all(|w| w[0].id < w[1].id));
        let found_a = all.iter().find(|r| r.id == a.id).expect("a listed");
        assert_eq!(found_a.alternate_names, vec!["First Alt", "Second Alt"]);
        let found_b = all.iter().find(|r| r.id == b.id).expect("b listed");
        assert!(found_b.alternate_names.is_empty());
        assert!(
            all.iter().all(|r| r.id != gone.id),
            "deleted events are skipped"
        );
    }

    /// A scan over fewer than `MIN_FREQUENCY_POPULATION` records knows no
    /// shares.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn a_small_population_scans_to_no_frequencies() {
        let db = connect().await;
        let repo = PgEventRepository::new(db.clone());
        let total = repo.list_active_names(10_000, None).await.unwrap().len();
        let source = crate::matching::frequencies::scan(&repo).await.unwrap();
        assert_eq!(
            source.is_none(),
            (total as u64) < crate::matching::MIN_FREQUENCY_POPULATION,
            "{total} records"
        );
    }
}
