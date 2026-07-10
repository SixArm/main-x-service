//! Repository layer: converts between the domain
//! [`Event`](crate::models::Event) and the SeaORM entities defined in
//! [`crate::db::models`].

use super::convert::{offset_to_ts, ts_to_offset};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, NotSet,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use time::OffsetDateTime;
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
    event_identifiers, event_links, event_locations, event_offers, event_parties, event_sub_events,
    event_text_values, events,
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
    /// Case-insensitive substring search over event names.
    ///
    /// # Errors
    ///
    /// Returns an error if the id query or any per-id reload fails.
    async fn search(&self, query: &str) -> Result<Vec<Event>>;
    /// List active events with `limit`/`offset` pagination.
    ///
    /// # Errors
    ///
    /// Returns an error if the id query or any per-id reload fails.
    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Event>>;
}

/// SeaORM-backed [`EventRepository`] with optional event publishing and
/// audit logging.
pub struct SeaOrmEventRepository {
    /// The database connection/pool.
    db: DatabaseConnection,
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

impl SeaOrmEventRepository {
    /// Construct a repository over `db` with no publisher or audit log,
    /// and the default [`EventTransport::Memory`] transport (behaviour
    /// unchanged from before the outbox landed).
    #[must_use]
    pub fn new(db: DatabaseConnection) -> Self {
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
    /// `&DatabaseTransaction` so the row commits with the entity write
    /// (the outbox atomicity guarantee). A no-op under
    /// [`EventTransport::Memory`].
    ///
    /// The `ConnectionTrait` generic lives here on a concrete method (not
    /// on the object-safe [`EventRepository`] trait), which is how a
    /// `dyn`-trait repository threads the outbox insert into its own
    /// transaction.
    async fn enqueue_outbox<C: ConnectionTrait>(
        &self,
        conn: &C,
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
// Conversions: domain Event ↔ SeaORM rows
// ---------------------------------------------------------------------------

/// One [`Event`] flattened into its parent row plus all child-table
/// rows, ready to insert under a single transaction.
struct ChildRows {
    /// The parent `events` row.
    event_row: events::ActiveModel,
    /// `event_identifiers` rows (one per external identifier).
    identifiers: Vec<event_identifiers::ActiveModel>,
    /// `event_locations` rows (one per [`Location`] in order).
    locations: Vec<event_locations::ActiveModel>,
    /// `event_parties` rows across all six role lists.
    parties: Vec<event_parties::ActiveModel>,
    /// `event_offers` rows (one per [`Offer`] in order).
    offers: Vec<event_offers::ActiveModel>,
    /// `event_links` rows (cross-event links).
    links: Vec<event_links::ActiveModel>,
    /// `event_sub_events` rows (sub-event id references in order).
    sub_events: Vec<event_sub_events::ActiveModel>,
    /// `event_text_values` rows (`alternate_name/image/same_as/keyword/in_language`).
    text_values: Vec<event_text_values::ActiveModel>,
}

/// All child-table rows loaded back for one event, in fixed order
/// (identifiers, locations, parties, offers, links, sub-events,
/// text-values), ready to feed into [`from_rows`].
struct LoadedChildRows {
    /// `event_identifiers` rows.
    identifiers: Vec<event_identifiers::Model>,
    /// `event_locations` rows.
    locations: Vec<event_locations::Model>,
    /// `event_parties` rows across all six role lists.
    parties: Vec<event_parties::Model>,
    /// `event_offers` rows.
    offers: Vec<event_offers::Model>,
    /// `event_links` rows.
    links: Vec<event_links::Model>,
    /// `event_sub_events` rows.
    sub_events: Vec<event_sub_events::Model>,
    /// `event_text_values` rows (ordered by `position`).
    text_values: Vec<event_text_values::Model>,
}

/// Flatten a domain [`Event`] into a [`ChildRows`] bundle of `SeaORM`
/// `ActiveModel`s. Stamps `created_at`/`updated_at` to "now"; mints
/// fresh `Uuid`s for child rows; serializes the JSONB array fields;
/// and fans the six party role-lists out via [`push_party_rows`].
/// Build the parent `events` [`ActiveModel`](events::ActiveModel) for a
/// fresh insert, stamping `created_at`/`updated_at` to `now`.
fn event_active_model(event: &Event, now: OffsetDateTime) -> events::ActiveModel {
    events::ActiveModel {
        id: Set(event.id),
        active: Set(event.active),
        name: Set(event.name.clone()),
        description: Set(event.description.clone()),
        disambiguating_description: Set(event.disambiguating_description.clone()),
        url: Set(event.url.clone()),
        start_date: Set(ts_to_offset(event.start_date)),
        end_date: Set(event.end_date.map(ts_to_offset)),
        door_time: Set(event.door_time.map(ts_to_offset)),
        duration: Set(event.duration.clone()),
        previous_start_date: Set(event.previous_start_date.map(ts_to_offset)),
        time_zone: Set(event.time_zone.clone()),
        all_day: Set(event.all_day),
        event_status: Set(enum_to_str(&event.event_status)),
        event_attendance_mode: Set(enum_to_str(&event.event_attendance_mode)),
        event_type: Set(enum_to_str(&event.event_type)),
        typical_age_range: Set(event.typical_age_range.clone()),
        is_accessible_for_free: Set(event.is_accessible_for_free),
        // Capacities are `u32` in the domain but stored as Postgres
        // `i32`; values above `i32::MAX` would wrap (not expected for
        // real attendee counts). Read-back casts `i32` → `u32`.
        maximum_attendee_capacity: Set(event.maximum_attendee_capacity.map(u32::cast_signed)),
        maximum_physical_attendee_capacity: Set(event
            .maximum_physical_attendee_capacity
            .map(u32::cast_signed)),
        maximum_virtual_attendee_capacity: Set(event
            .maximum_virtual_attendee_capacity
            .map(u32::cast_signed)),
        remaining_attendee_capacity: Set(event.remaining_attendee_capacity.map(u32::cast_signed)),
        super_event_id: Set(event.super_event),
        created_at: Set(now),
        updated_at: Set(now),
        created_by: Set(None),
        updated_by: Set(None),
        deleted_at: Set(None),
        deleted_by: Set(None),
    }
}

/// Build the `event_offers` rows (one per [`Offer`] in order). Price is
/// parsed from its decimal-string form into `BigDecimal`; an
/// unparseable string stores no price.
fn offer_rows(event: &Event, now: OffsetDateTime) -> Vec<event_offers::ActiveModel> {
    event
        .offers
        .iter()
        .enumerate()
        .map(|(pos, o)| event_offers::ActiveModel {
            id: Set(Uuid::new_v4()),
            event_id: Set(event.id),
            position: Set(i32::try_from(pos).unwrap_or(i32::MAX)),
            name: Set(o.name.clone()),
            price: Set(o
                .price
                .as_deref()
                .and_then(|s| s.parse::<bigdecimal::BigDecimal>().ok())),
            price_currency: Set(o.price_currency.clone()),
            url: Set(o.url.clone()),
            availability: Set(o.availability.as_ref().map(enum_to_str)),
            valid_from: Set(o.valid_from.map(ts_to_offset)),
            valid_through: Set(o.valid_through.map(ts_to_offset)),
            created_at: Set(now),
            updated_at: Set(now),
        })
        .collect()
}

/// Build the tagged `event_text_values` rows from the five string-list
/// properties (`alternate_name`/`image`/`same_as`/`keyword`/`in_language`).
fn text_value_rows(event: &Event) -> Vec<event_text_values::ActiveModel> {
    let mut text_values = Vec::new();
    for (field, values) in [
        ("alternate_name", &event.alternate_names),
        ("image", &event.image),
        ("same_as", &event.same_as),
        ("keyword", &event.keywords),
        ("in_language", &event.in_language),
    ] {
        for (pos, value) in values.iter().enumerate() {
            text_values.push(event_text_values::ActiveModel {
                id: Set(Uuid::new_v4()),
                event_id: Set(event.id),
                field: Set(field.to_string()),
                value: Set(value.clone()),
                position: Set(i32::try_from(pos).unwrap_or(i32::MAX)),
            });
        }
    }
    text_values
}

fn to_rows(event: &Event) -> ChildRows {
    let now = OffsetDateTime::now_utc();
    let event_row = event_active_model(event, now);

    let identifiers = event
        .identifiers
        .iter()
        .map(|id| event_identifiers::ActiveModel {
            id: Set(Uuid::new_v4()),
            event_id: Set(event.id),
            use_type: Set(id.use_type.as_ref().map(enum_to_str)),
            identifier_type: Set(enum_to_str(&id.identifier_type)),
            system: Set(id.system.clone()),
            value: Set(id.value.clone()),
            assigner: Set(id.assigner.clone()),
            created_at: Set(now),
            updated_at: Set(now),
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
        .map(|link| event_links::ActiveModel {
            id: Set(Uuid::new_v4()),
            event_id: Set(event.id),
            other_event_id: Set(link.other_event_id),
            link_type: Set(enum_to_str(&link.link_type)),
            created_at: Set(now),
            created_by: Set(None),
        })
        .collect();

    let sub_events = event
        .sub_events
        .iter()
        .enumerate()
        .map(|(pos, sub_id)| event_sub_events::ActiveModel {
            id: Set(Uuid::new_v4()),
            event_id: Set(event.id),
            sub_event_id: Set(*sub_id),
            position: Set(i32::try_from(pos).unwrap_or(i32::MAX)),
            created_at: Set(now),
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
    rows: &mut Vec<event_parties::ActiveModel>,
    event_id: Uuid,
    role: &str,
    parties: &[Party],
    now: OffsetDateTime,
) {
    for (pos, p) in parties.iter().enumerate() {
        rows.push(event_parties::ActiveModel {
            id: Set(Uuid::new_v4()),
            event_id: Set(event_id),
            position: Set(i32::try_from(pos).unwrap_or(i32::MAX)),
            role: Set(role.to_string()),
            party_kind: Set(enum_to_str(&p.kind)),
            party_id: Set(p.id),
            name: Set(p.name.clone()),
            email: Set(p.email.clone()),
            url: Set(p.url.clone()),
            created_at: Set(now),
            updated_at: Set(now),
        });
    }
}

/// Flatten one [`Location`] union variant into a single
/// `event_locations` row. The `kind` column (`"place"` /
/// `"postal_address"` / `"virtual"` / `"text"`) records which variant
/// was stored so [`row_to_location`] can reconstruct it; columns not
/// relevant to the variant are left `None`.
fn location_to_row(
    event_id: Uuid,
    pos: i32,
    loc: &Location,
    now: OffsetDateTime,
) -> event_locations::ActiveModel {
    let mut row = event_locations::ActiveModel {
        id: Set(Uuid::new_v4()),
        event_id: Set(event_id),
        position: Set(pos),
        kind: Set("text".into()),
        place_id: Set(None),
        name: Set(None),
        line1: Set(None),
        line2: Set(None),
        city: Set(None),
        state: Set(None),
        postal_code: Set(None),
        country: Set(None),
        latitude: Set(None),
        longitude: Set(None),
        url: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    match loc {
        Location::Place(p) => {
            row.kind = Set("place".into());
            row.place_id = Set(p.id);
            row.name = Set(Some(p.name.clone()));
            if let Some(addr) = &p.address {
                row.line1 = Set(addr.line1.clone());
                row.line2 = Set(addr.line2.clone());
                row.city = Set(addr.city.clone());
                row.state = Set(addr.state.clone());
                row.postal_code = Set(addr.postal_code.clone());
                row.country = Set(addr.country.clone());
            }
            row.latitude = Set(p.latitude);
            row.longitude = Set(p.longitude);
            row.url = Set(p.url.clone());
        }
        Location::PostalAddress(addr) => {
            row.kind = Set("postal_address".into());
            row.line1 = Set(addr.line1.clone());
            row.line2 = Set(addr.line2.clone());
            row.city = Set(addr.city.clone());
            row.state = Set(addr.state.clone());
            row.postal_code = Set(addr.postal_code.clone());
            row.country = Set(addr.country.clone());
        }
        Location::Virtual(v) => {
            row.kind = Set("virtual".into());
            row.name = Set(v.name.clone());
            row.url = Set(Some(v.url.clone()));
        }
        Location::Text { value } => {
            row.kind = Set("text".into());
            row.name = Set(Some(value.clone()));
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
fn text_values_of(rows: &[event_text_values::Model], field: &str) -> Vec<String> {
    rows.iter()
        .filter(|r| r.field == field)
        .map(|r| r.value.clone())
        .collect()
}

/// Reassemble the six party role-lists from the flat `event_parties`
/// rows, keyed by `role` and ordered by `position`.
fn parties_by_role(
    mut parties: Vec<event_parties::Model>,
) -> std::collections::HashMap<String, Vec<Party>> {
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
fn identifiers_from_rows(rows: Vec<event_identifiers::Model>) -> Vec<Identifier> {
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
fn locations_from_rows(mut rows: Vec<event_locations::Model>) -> Vec<Location> {
    rows.sort_by_key(|l| l.position);
    rows.iter().filter_map(row_to_location).collect()
}

/// Sort `event_offers` rows by `position` and map them back into domain
/// [`Offer`]s.
fn offers_from_rows(mut rows: Vec<event_offers::Model>) -> Vec<Offer> {
    rows.sort_by_key(|o| o.position);
    rows.into_iter()
        .map(|o| Offer {
            name: o.name,
            price: o.price.as_ref().map(std::string::ToString::to_string),
            price_currency: o.price_currency,
            url: o.url,
            availability: o.availability.as_deref().and_then(parse_offer_availability),
            valid_from: o.valid_from.map(offset_to_ts),
            valid_through: o.valid_through.map(offset_to_ts),
        })
        .collect()
}

/// Map `event_links` rows back into domain [`EventLink`]s.
fn links_from_rows(rows: Vec<event_links::Model>) -> Vec<EventLink> {
    rows.into_iter()
        .map(|l| EventLink {
            other_event_id: l.other_event_id,
            link_type: str_to_enum(&l.link_type, LinkType::Seealso),
        })
        .collect()
}

/// Sort `event_sub_events` rows by `position` and project their ids.
fn sub_event_ids_from_rows(mut rows: Vec<event_sub_events::Model>) -> Vec<Uuid> {
    rows.sort_by_key(|s| s.position);
    rows.into_iter().map(|s| s.sub_event_id).collect()
}

fn from_rows(event_row: events::Model, children: LoadedChildRows) -> Event {
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
        start_date: offset_to_ts(event_row.start_date),
        end_date: event_row.end_date.map(offset_to_ts),
        door_time: event_row.door_time.map(offset_to_ts),
        duration: event_row.duration,
        previous_start_date: event_row.previous_start_date.map(offset_to_ts),
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
        created_at: offset_to_ts(event_row.created_at),
        updated_at: offset_to_ts(event_row.updated_at),
    }
}

/// Reconstruct a [`Location`] union variant from one `event_locations`
/// row, dispatching on the `kind` column. Returns `None` for an
/// unrecognized `kind` or a `text` row missing its value, so callers
/// `filter_map` over the results.
fn row_to_location(row: &event_locations::Model) -> Option<Location> {
    match row.kind.as_str() {
        "place" => Some(Location::Place(Place {
            id: row.place_id,
            name: row.name.clone().unwrap_or_default(),
            address: address_from_row(row),
            latitude: row.latitude,
            longitude: row.longitude,
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
fn address_from_row(row: &event_locations::Model) -> Option<Address> {
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

/// Parse an `identifier_type` column string into an [`IdentifierType`],
/// defaulting to [`IdentifierType::Other`] for unknown values.
fn parse_identifier_type(s: &str) -> IdentifierType {
    str_to_enum(s, IdentifierType::Other)
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

/// Apply an event's parent-row update and wholesale child-row
/// replacement on `conn`. Shared by [`EventRepository::update`] and
/// [`EventRepository::merge`]: preserves `created_*`, stamps
/// `updated_at` to now, deletes every child row for the event, then
/// re-inserts them from the domain model. Runs on the caller's
/// connection/transaction so it shares that commit boundary.
async fn apply_update_rows<C: ConnectionTrait>(conn: &C, event: &Event) -> Result<()> {
    // Update the events row (preserving created_*; updating updated_*).
    let rows = to_rows(event);
    let mut row = rows.event_row;
    row.created_at = NotSet;
    row.created_by = NotSet;
    row.updated_at = Set(OffsetDateTime::now_utc());
    row.update(conn).await?;

    // Replace child rows wholesale.
    event_identifiers::Entity::delete_many()
        .filter(event_identifiers::Column::EventId.eq(event.id))
        .exec(conn)
        .await?;
    event_locations::Entity::delete_many()
        .filter(event_locations::Column::EventId.eq(event.id))
        .exec(conn)
        .await?;
    event_parties::Entity::delete_many()
        .filter(event_parties::Column::EventId.eq(event.id))
        .exec(conn)
        .await?;
    event_offers::Entity::delete_many()
        .filter(event_offers::Column::EventId.eq(event.id))
        .exec(conn)
        .await?;
    event_links::Entity::delete_many()
        .filter(event_links::Column::EventId.eq(event.id))
        .exec(conn)
        .await?;
    event_sub_events::Entity::delete_many()
        .filter(event_sub_events::Column::EventId.eq(event.id))
        .exec(conn)
        .await?;
    event_text_values::Entity::delete_many()
        .filter(event_text_values::Column::EventId.eq(event.id))
        .exec(conn)
        .await?;

    for r in rows.identifiers {
        r.insert(conn).await?;
    }
    for r in rows.locations {
        r.insert(conn).await?;
    }
    for r in rows.parties {
        r.insert(conn).await?;
    }
    for r in rows.offers {
        r.insert(conn).await?;
    }
    for r in rows.links {
        r.insert(conn).await?;
    }
    for r in rows.sub_events {
        r.insert(conn).await?;
    }
    for r in rows.text_values {
        r.insert(conn).await?;
    }
    Ok(())
}

/// Soft-delete the event `id` on `conn` by stamping `deleted_at` /
/// `deleted_by` and leaving every other column (and the child rows)
/// in place. Factored from [`EventRepository::delete`]'s soft-delete
/// so [`EventRepository::merge`] can reuse it on its own transaction.
async fn apply_soft_delete_row<C: ConnectionTrait>(conn: &C, id: &Uuid) -> Result<()> {
    let row = events::ActiveModel {
        id: Set(*id),
        deleted_at: Set(Some(OffsetDateTime::now_utc())),
        deleted_by: Set(Some("system".into())),
        ..Default::default()
    };
    row.update(conn).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// EventRepository impl
// ---------------------------------------------------------------------------

impl SeaOrmEventRepository {
    /// Load all child-row vectors for one event id in fixed order
    /// (identifiers, locations, parties, offers, links, sub-events,
    /// text-values), ready to feed into [`from_rows`]. The text-values
    /// query is ordered by `position` so list order is preserved; the
    /// other child rows are sorted inside [`from_rows`].
    ///
    /// # Errors
    ///
    /// Returns the `SeaORM` error if any of the per-child-table queries
    /// fails.
    async fn load_children(&self, event_id: &Uuid) -> Result<LoadedChildRows> {
        let identifiers = event_identifiers::Entity::find()
            .filter(event_identifiers::Column::EventId.eq(*event_id))
            .all(&self.db)
            .await?;
        let locations = event_locations::Entity::find()
            .filter(event_locations::Column::EventId.eq(*event_id))
            .all(&self.db)
            .await?;
        let parties = event_parties::Entity::find()
            .filter(event_parties::Column::EventId.eq(*event_id))
            .all(&self.db)
            .await?;
        let offers = event_offers::Entity::find()
            .filter(event_offers::Column::EventId.eq(*event_id))
            .all(&self.db)
            .await?;
        let links = event_links::Entity::find()
            .filter(event_links::Column::EventId.eq(*event_id))
            .all(&self.db)
            .await?;
        let sub_events = event_sub_events::Entity::find()
            .filter(event_sub_events::Column::EventId.eq(*event_id))
            .all(&self.db)
            .await?;
        let text_values = event_text_values::Entity::find()
            .filter(event_text_values::Column::EventId.eq(*event_id))
            .order_by_asc(event_text_values::Column::Position)
            .all(&self.db)
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
impl EventRepository for SeaOrmEventRepository {
    async fn create(&self, event: &Event) -> Result<Event> {
        // All parent + child inserts run under one transaction so a
        // partial failure leaves no orphan child rows.
        let txn = self.db.begin().await?;

        // Flatten the domain event into parent + child `ActiveModel`s,
        // then insert the parent first (children reference its id).
        let rows = to_rows(event);
        let inserted = rows.event_row.insert(&txn).await?;
        for r in rows.identifiers {
            r.insert(&txn).await?;
        }
        for r in rows.locations {
            r.insert(&txn).await?;
        }
        for r in rows.parties {
            r.insert(&txn).await?;
        }
        for r in rows.offers {
            r.insert(&txn).await?;
        }
        for r in rows.links {
            r.insert(&txn).await?;
        }
        for r in rows.sub_events {
            r.insert(&txn).await?;
        }
        for r in rows.text_values {
            r.insert(&txn).await?;
        }

        // Durable event bus (Phase 2): under the outbox transport, write
        // the `event_outbox` row **inside this same transaction**, before
        // the commit, so the entity rows and the event commit atomically
        // (or roll back together). A no-op under the memory transport,
        // which keeps the post-commit ring-buffer publish below.
        self.enqueue_outbox(&txn, event, EventKind::Created).await?;

        txn.commit().await?;

        // Reload children and reassemble the canonical domain `Event`
        // (positions/JSON round-tripped) to return to the caller.
        let children = self.load_children(&inserted.id).await?;
        let result = from_rows(inserted, children);

        // Fire-and-forget side effects: publish a `Created` stream event
        // (chrono `DateTime<Utc>` on the streaming side vs `time` on the row),
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
        // Soft-delete aware: a non-null `deleted_at` hides the row, so a
        // tombstoned event reads back as `None`.
        let event_row = events::Entity::find_by_id(*id)
            .filter(events::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;
        let Some(event_row) = event_row else {
            return Ok(None);
        };
        let children = self.load_children(id).await?;
        Ok(Some(from_rows(event_row, children)))
    }

    async fn update(&self, event: &Event) -> Result<Event> {
        // Snapshot the pre-update state first so the audit log can carry
        // an accurate `old_values` even though children are replaced.
        let old = self.get_by_id(&event.id).await?;
        let txn = self.db.begin().await?;

        // Parent-row update + wholesale child-row replacement, shared
        // with `merge` (see `apply_update_rows`).
        apply_update_rows(&txn, event).await?;

        // Outbox row shares the update transaction (see `create`).
        self.enqueue_outbox(&txn, event, EventKind::Updated).await?;

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
        let txn = self.db.begin().await?;

        // Apply the survivor's parent + child rows (shared with `update`).
        apply_update_rows(&txn, survivor).await?;

        // Soft-delete the duplicate (shared with `delete`).
        apply_soft_delete_row(&txn, duplicate_id).await?;

        // Durable event bus (Phase 2): a `Merged` outbox row for the
        // survivor (carrying the duplicate's pid via `merged_from`, so a
        // merge-repointing consumer can move edges off the duplicate) and a
        // `Deleted` outbox row for the duplicate — both inside this
        // transaction. A no-op under the memory transport.
        if self.transport.is_outbox() {
            OutboxInsert::for_merge(survivor, duplicate_id)?
                .insert_on(&txn)
                .await?;
            if let Some(dup) = old_duplicate.as_ref() {
                OutboxInsert::for_event(dup, EventKind::Deleted)?
                    .insert_on(&txn)
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
        // Soft delete: stamp `deleted_at`/`deleted_by` only. The other
        // fields stay `NotSet` (via `..Default::default()`) so this is a
        // partial UPDATE that leaves the rest of the row untouched and
        // the child rows in place.
        let row = events::ActiveModel {
            id: Set(*id),
            deleted_at: Set(Some(OffsetDateTime::now_utc())),
            deleted_by: Set(Some("system".into())),
            ..Default::default()
        };
        // Unlike create/update, the soft-delete is a single row update
        // with no existing transaction. Under the outbox transport we
        // open one here so the tombstone write and the `deleted` outbox
        // row commit atomically; the memory transport keeps the plain,
        // tx-free update.
        if self.transport.is_outbox() {
            let txn = self.db.begin().await?;
            row.update(&txn).await?;
            if let Some(old) = old.as_ref() {
                self.enqueue_outbox(&txn, old, EventKind::Deleted).await?;
            }
            txn.commit().await?;
        } else {
            row.update(&self.db).await?;
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

    async fn search(&self, query: &str) -> Result<Vec<Event>> {
        // Case-insensitive substring match: lower-case both sides and
        // wrap the query in `%…%`. The pattern is bound as `$1` so the
        // user text is never interpolated into the SQL string.
        let pattern = format!("%{}%", query.to_lowercase());
        // First pass: project just the matching ids (excluding
        // soft-deleted rows) to keep the scan cheap.
        let event_ids: Vec<Uuid> = events::Entity::find()
            .filter(events::Column::DeletedAt.is_null())
            .filter(Expr::cust_with_values("LOWER(name) LIKE $1", [pattern]))
            .select_only()
            .column(events::Column::Id)
            .into_tuple()
            .all(&self.db)
            .await?;
        // Second pass: hydrate each match (with children) into a full
        // domain `Event` via the soft-delete-aware `get_by_id`.
        let mut events = Vec::new();
        for id in event_ids {
            if let Some(e) = self.get_by_id(&id).await? {
                events.push(e);
            }
        }
        Ok(events)
    }

    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Event>> {
        let rows = events::Entity::find()
            .filter(events::Column::DeletedAt.is_null())
            .filter(events::Column::Active.eq(true))
            .limit(limit)
            .offset(offset)
            .all(&self.db)
            .await?;
        let mut events = Vec::new();
        for row in rows {
            if let Some(e) = self.get_by_id(&row.id).await? {
                events.push(e);
            }
        }
        Ok(events)
    }
}

/// DB-gated (`#[ignore]`) atomicity tests for the outbox write path.
/// They require a migrated `PostgreSQL` via `DATABASE_URL` and are skipped
/// by a bare `cargo test`; run with
/// `DATABASE_URL=… cargo test --lib -- --ignored`. They must COMPILE
/// under a bare `cargo test --lib`.
#[cfg(test)]
mod tests {
    use super::{EventRepository, SeaOrmEventRepository};
    use crate::db::models::event_outbox;
    use crate::models::Event;
    use crate::streaming::EventTransport;
    use chrono::{TimeZone, Utc};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    async fn connect() -> sea_orm::DatabaseConnection {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        sea_orm::Database::connect(&url)
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
        let repo = SeaOrmEventRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let start = Utc.with_ymd_and_hms(2026, 6, 1, 9, 0, 0).unwrap();
        let survivor = repo.create(&Event::new("Survivor", start)).await.unwrap();
        let duplicate = repo.create(&Event::new("Duplicate", start)).await.unwrap();

        let merged = repo.merge(&survivor, &duplicate.id).await.unwrap();
        assert_eq!(merged.id, survivor.id);

        // Survivor: exactly one `merged` row, carrying the duplicate pid.
        let merged_rows = event_outbox::Entity::find()
            .filter(event_outbox::Column::EntityPid.eq(survivor.id))
            .filter(event_outbox::Column::Kind.eq("merged"))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(merged_rows.len(), 1, "one merged outbox row for survivor");
        assert_eq!(
            merged_rows[0].payload["merged_from"],
            serde_json::json!(duplicate.id.to_string()),
            "merged row carries the duplicate's pid"
        );

        // Duplicate: a `deleted` row.
        let deleted_rows = event_outbox::Entity::find()
            .filter(event_outbox::Column::EntityPid.eq(duplicate.id))
            .filter(event_outbox::Column::Kind.eq("deleted"))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(
            deleted_rows.len(),
            1,
            "one deleted outbox row for duplicate"
        );
    }
}
