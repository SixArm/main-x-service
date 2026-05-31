//! Repository layer: converts between the domain [`Event`] and the
//! SeaORM entities defined in [`crate::db::models`].

use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::*;
use uuid::Uuid;

use crate::models::{
    Address, Event, EventAttendanceMode, EventLink, EventStatus, EventType, Identifier,
    IdentifierType, IdentifierUse, LinkType, Location, Offer, OfferAvailability, Party,
    PartyKind, Place, VirtualLocation,
};
use crate::Result;

use super::models::*;

/// Per-request audit context.
#[derive(Debug, Clone)]
pub struct AuditContext {
    pub user_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl Default for AuditContext {
    fn default() -> Self {
        Self {
            user_id: Some("system".into()),
            ip_address: None,
            user_agent: None,
        }
    }
}

/// CRUD + simple search for [`Event`].
#[async_trait::async_trait]
pub trait EventRepository: Send + Sync {
    async fn create(&self, event: &Event) -> Result<Event>;
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Event>>;
    async fn update(&self, event: &Event) -> Result<Event>;
    async fn delete(&self, id: &Uuid) -> Result<()>;
    async fn search(&self, query: &str) -> Result<Vec<Event>>;
    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Event>>;
}

/// SeaORM-backed implementation.
pub struct SeaOrmEventRepository {
    db: DatabaseConnection,
    event_publisher: Option<std::sync::Arc<dyn crate::streaming::EventProducer>>,
    audit_log: Option<std::sync::Arc<super::audit::AuditLogRepository>>,
}

impl SeaOrmEventRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            event_publisher: None,
            audit_log: None,
        }
    }

    pub fn with_event_publisher(
        mut self,
        publisher: std::sync::Arc<dyn crate::streaming::EventProducer>,
    ) -> Self {
        self.event_publisher = Some(publisher);
        self
    }

    pub fn with_audit_log(
        mut self,
        audit_log: std::sync::Arc<super::audit::AuditLogRepository>,
    ) -> Self {
        self.audit_log = Some(audit_log);
        self
    }

    fn publish_event(&self, event: crate::streaming::EventEvent) {
        if let Some(ref publisher) = self.event_publisher {
            if let Err(e) = publisher.publish(event) {
                tracing::error!("Failed to publish event: {}", e);
            }
        }
    }

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
                        context.user_id.clone(),
                        context.ip_address.clone(),
                        context.user_agent.clone(),
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
                        context.user_id.clone(),
                        context.ip_address.clone(),
                        context.user_agent.clone(),
                    )
                    .await
            }
            "DELETE" => {
                audit_log
                    .log_delete(
                        "Event",
                        entity_id,
                        old_values.unwrap_or(serde_json::Value::Null),
                        context.user_id.clone(),
                        context.ip_address.clone(),
                        context.user_agent.clone(),
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

struct ChildRows {
    event_row: events::ActiveModel,
    identifiers: Vec<event_identifiers::ActiveModel>,
    locations: Vec<event_locations::ActiveModel>,
    parties: Vec<event_parties::ActiveModel>,
    offers: Vec<event_offers::ActiveModel>,
    links: Vec<event_links::ActiveModel>,
    sub_events: Vec<event_sub_events::ActiveModel>,
}

fn to_rows(event: &Event) -> ChildRows {
    let now = Utc::now();
    let event_row = events::ActiveModel {
        id: Set(event.id),
        active: Set(event.active),
        name: Set(event.name.clone()),
        description: Set(event.description.clone()),
        disambiguating_description: Set(event.disambiguating_description.clone()),
        url: Set(event.url.clone()),
        alternate_names: Set(serde_json::to_value(&event.alternate_names)
            .unwrap_or(serde_json::Value::Array(vec![]))),
        image: Set(serde_json::to_value(&event.image).unwrap_or(serde_json::Value::Array(vec![]))),
        same_as: Set(serde_json::to_value(&event.same_as).unwrap_or(serde_json::Value::Array(vec![]))),
        keywords: Set(serde_json::to_value(&event.keywords).unwrap_or(serde_json::Value::Array(vec![]))),
        in_language: Set(serde_json::to_value(&event.in_language)
            .unwrap_or(serde_json::Value::Array(vec![]))),
        start_date: Set(event.start_date),
        end_date: Set(event.end_date),
        door_time: Set(event.door_time),
        duration: Set(event.duration.clone()),
        previous_start_date: Set(event.previous_start_date),
        time_zone: Set(event.time_zone.clone()),
        all_day: Set(event.all_day),
        event_status: Set(enum_to_str(&event.event_status)),
        event_attendance_mode: Set(enum_to_str(&event.event_attendance_mode)),
        event_type: Set(enum_to_str(&event.event_type)),
        typical_age_range: Set(event.typical_age_range.clone()),
        is_accessible_for_free: Set(event.is_accessible_for_free),
        maximum_attendee_capacity: Set(event.maximum_attendee_capacity.map(|v| v as i32)),
        maximum_physical_attendee_capacity: Set(event
            .maximum_physical_attendee_capacity
            .map(|v| v as i32)),
        maximum_virtual_attendee_capacity: Set(event
            .maximum_virtual_attendee_capacity
            .map(|v| v as i32)),
        remaining_attendee_capacity: Set(event.remaining_attendee_capacity.map(|v| v as i32)),
        super_event_id: Set(event.super_event),
        created_at: Set(now),
        updated_at: Set(now),
        created_by: Set(None),
        updated_by: Set(None),
        deleted_at: Set(None),
        deleted_by: Set(None),
    };

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
        .map(|(pos, loc)| location_to_row(event.id, pos as i32, loc, now))
        .collect();

    let mut parties = Vec::new();
    push_party_rows(&mut parties, event.id, "organizer", &event.organizers, now);
    push_party_rows(&mut parties, event.id, "performer", &event.performers, now);
    push_party_rows(&mut parties, event.id, "attendee", &event.attendees, now);
    push_party_rows(&mut parties, event.id, "sponsor", &event.sponsors, now);
    push_party_rows(&mut parties, event.id, "funder", &event.funders, now);
    push_party_rows(&mut parties, event.id, "contributor", &event.contributors, now);

    let offers = event
        .offers
        .iter()
        .enumerate()
        .map(|(pos, o)| event_offers::ActiveModel {
            id: Set(Uuid::new_v4()),
            event_id: Set(event.id),
            position: Set(pos as i32),
            name: Set(o.name.clone()),
            price: Set(o
                .price
                .as_deref()
                .and_then(|s| s.parse::<bigdecimal::BigDecimal>().ok())),
            price_currency: Set(o.price_currency.clone()),
            url: Set(o.url.clone()),
            availability: Set(o.availability.as_ref().map(enum_to_str)),
            valid_from: Set(o.valid_from),
            valid_through: Set(o.valid_through),
            created_at: Set(now),
            updated_at: Set(now),
        })
        .collect();

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
            position: Set(pos as i32),
            created_at: Set(now),
        })
        .collect();

    ChildRows {
        event_row,
        identifiers,
        locations,
        parties,
        offers,
        links,
        sub_events,
    }
}

fn push_party_rows(
    rows: &mut Vec<event_parties::ActiveModel>,
    event_id: Uuid,
    role: &str,
    parties: &[Party],
    now: chrono::DateTime<Utc>,
) {
    for (pos, p) in parties.iter().enumerate() {
        rows.push(event_parties::ActiveModel {
            id: Set(Uuid::new_v4()),
            event_id: Set(event_id),
            position: Set(pos as i32),
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

fn location_to_row(
    event_id: Uuid,
    pos: i32,
    loc: &Location,
    now: chrono::DateTime<Utc>,
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

fn from_rows(
    event_row: events::Model,
    identifiers: Vec<event_identifiers::Model>,
    locations: Vec<event_locations::Model>,
    parties: Vec<event_parties::Model>,
    offers: Vec<event_offers::Model>,
    links: Vec<event_links::Model>,
    sub_events: Vec<event_sub_events::Model>,
) -> Event {
    let alternate_names: Vec<String> =
        serde_json::from_value(event_row.alternate_names).unwrap_or_default();
    let image: Vec<String> = serde_json::from_value(event_row.image).unwrap_or_default();
    let same_as: Vec<String> = serde_json::from_value(event_row.same_as).unwrap_or_default();
    let keywords: Vec<String> = serde_json::from_value(event_row.keywords).unwrap_or_default();
    let in_language: Vec<String> =
        serde_json::from_value(event_row.in_language).unwrap_or_default();

    let event_status = str_to_enum(&event_row.event_status, EventStatus::Scheduled);
    let event_attendance_mode =
        str_to_enum(&event_row.event_attendance_mode, EventAttendanceMode::Offline);
    let event_type = str_to_enum(&event_row.event_type, EventType::Generic);

    let identifiers = identifiers
        .into_iter()
        .map(|id| Identifier {
            use_type: id.use_type.as_deref().and_then(parse_identifier_use),
            identifier_type: parse_identifier_type(&id.identifier_type),
            system: id.system,
            value: id.value,
            assigner: id.assigner,
        })
        .collect();

    let mut sorted_locations = locations;
    sorted_locations.sort_by_key(|l| l.position);
    let location = sorted_locations.into_iter().filter_map(row_to_location).collect();

    let mut by_role: std::collections::HashMap<String, Vec<Party>> = std::collections::HashMap::new();
    let mut parties_sorted = parties;
    parties_sorted.sort_by_key(|p| p.position);
    for p in parties_sorted {
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

    let mut offers_sorted = offers;
    offers_sorted.sort_by_key(|o| o.position);
    let offers = offers_sorted
        .into_iter()
        .map(|o| Offer {
            name: o.name,
            price: o.price.as_ref().map(|p| p.to_string()),
            price_currency: o.price_currency,
            url: o.url,
            availability: o.availability.as_deref().and_then(parse_offer_availability),
            valid_from: o.valid_from,
            valid_through: o.valid_through,
        })
        .collect();

    let links = links
        .into_iter()
        .map(|l| EventLink {
            other_event_id: l.other_event_id,
            link_type: str_to_enum(&l.link_type, LinkType::Seealso),
        })
        .collect();

    let mut sub_events_sorted = sub_events;
    sub_events_sorted.sort_by_key(|s| s.position);
    let sub_events_ids = sub_events_sorted
        .into_iter()
        .map(|s| s.sub_event_id)
        .collect();

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
        maximum_attendee_capacity: event_row.maximum_attendee_capacity.map(|v| v as u32),
        maximum_physical_attendee_capacity: event_row
            .maximum_physical_attendee_capacity
            .map(|v| v as u32),
        maximum_virtual_attendee_capacity: event_row
            .maximum_virtual_attendee_capacity
            .map(|v| v as u32),
        remaining_attendee_capacity: event_row.remaining_attendee_capacity.map(|v| v as u32),
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

fn row_to_location(row: event_locations::Model) -> Option<Location> {
    match row.kind.as_str() {
        "place" => Some(Location::Place(Place {
            id: row.place_id,
            name: row.name.clone().unwrap_or_default(),
            address: address_from_row(&row),
            latitude: row.latitude,
            longitude: row.longitude,
            url: row.url.clone(),
        })),
        "postal_address" => address_from_row(&row).map(Location::PostalAddress),
        "virtual" => Some(Location::Virtual(VirtualLocation {
            name: row.name.clone(),
            url: row.url.clone().unwrap_or_default(),
        })),
        "text" => row.name.clone().map(|value| Location::Text { value }),
        _ => None,
    }
}

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

fn enum_to_str<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

fn str_to_enum<T: serde::de::DeserializeOwned>(s: &str, default: T) -> T {
    serde_json::from_value::<T>(serde_json::Value::String(s.to_string())).unwrap_or(default)
}

fn parse_identifier_type(s: &str) -> IdentifierType {
    str_to_enum(s, IdentifierType::Other)
}

fn parse_identifier_use(s: &str) -> Option<IdentifierUse> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
}

fn parse_offer_availability(s: &str) -> Option<OfferAvailability> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
}

// ---------------------------------------------------------------------------
// EventRepository impl
// ---------------------------------------------------------------------------

impl SeaOrmEventRepository {
    async fn load_children(
        &self,
        event_id: &Uuid,
    ) -> Result<(
        Vec<event_identifiers::Model>,
        Vec<event_locations::Model>,
        Vec<event_parties::Model>,
        Vec<event_offers::Model>,
        Vec<event_links::Model>,
        Vec<event_sub_events::Model>,
    )> {
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
        Ok((identifiers, locations, parties, offers, links, sub_events))
    }
}

#[async_trait::async_trait]
impl EventRepository for SeaOrmEventRepository {
    async fn create(&self, event: &Event) -> Result<Event> {
        let txn = self.db.begin().await?;

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

        txn.commit().await?;

        let (identifiers, locations, parties, offers, links, sub_events) =
            self.load_children(&inserted.id).await?;
        let result = from_rows(inserted, identifiers, locations, parties, offers, links, sub_events);

        self.publish_event(crate::streaming::EventEvent::Created {
            event: result.clone(),
            timestamp: Utc::now(),
        });
        if let Ok(json) = serde_json::to_value(&result) {
            self.log_audit("CREATE", result.id, None, Some(json), &AuditContext::default())
                .await;
        }
        Ok(result)
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Event>> {
        let event_row = events::Entity::find_by_id(*id)
            .filter(events::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;
        let Some(event_row) = event_row else {
            return Ok(None);
        };
        let (identifiers, locations, parties, offers, links, sub_events) =
            self.load_children(id).await?;
        Ok(Some(from_rows(
            event_row, identifiers, locations, parties, offers, links, sub_events,
        )))
    }

    async fn update(&self, event: &Event) -> Result<Event> {
        let old = self.get_by_id(&event.id).await?;
        let txn = self.db.begin().await?;

        // Update the events row (preserving created_*; updating updated_*).
        let rows = to_rows(event);
        let mut row = rows.event_row;
        row.created_at = NotSet;
        row.created_by = NotSet;
        row.updated_at = Set(Utc::now());
        row.update(&txn).await?;

        // Replace child rows wholesale.
        event_identifiers::Entity::delete_many()
            .filter(event_identifiers::Column::EventId.eq(event.id))
            .exec(&txn)
            .await?;
        event_locations::Entity::delete_many()
            .filter(event_locations::Column::EventId.eq(event.id))
            .exec(&txn)
            .await?;
        event_parties::Entity::delete_many()
            .filter(event_parties::Column::EventId.eq(event.id))
            .exec(&txn)
            .await?;
        event_offers::Entity::delete_many()
            .filter(event_offers::Column::EventId.eq(event.id))
            .exec(&txn)
            .await?;
        event_links::Entity::delete_many()
            .filter(event_links::Column::EventId.eq(event.id))
            .exec(&txn)
            .await?;
        event_sub_events::Entity::delete_many()
            .filter(event_sub_events::Column::EventId.eq(event.id))
            .exec(&txn)
            .await?;

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

        txn.commit().await?;

        let result = self
            .get_by_id(&event.id)
            .await?
            .ok_or_else(|| crate::Error::Validation("Event not found after update".into()))?;

        self.publish_event(crate::streaming::EventEvent::Updated {
            event: result.clone(),
            timestamp: Utc::now(),
        });
        if let (Some(old), Ok(new_json)) = (old, serde_json::to_value(&result)) {
            if let Ok(old_json) = serde_json::to_value(&old) {
                self.log_audit(
                    "UPDATE",
                    result.id,
                    Some(old_json),
                    Some(new_json),
                    &AuditContext::default(),
                )
                .await;
            }
        }
        Ok(result)
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        let old = self.get_by_id(id).await?;
        let row = events::ActiveModel {
            id: Set(*id),
            deleted_at: Set(Some(Utc::now())),
            deleted_by: Set(Some("system".into())),
            ..Default::default()
        };
        row.update(&self.db).await?;
        self.publish_event(crate::streaming::EventEvent::Deleted {
            event_id: *id,
            timestamp: Utc::now(),
        });
        if let Some(old) = old {
            if let Ok(old_json) = serde_json::to_value(&old) {
                self.log_audit("DELETE", *id, Some(old_json), None, &AuditContext::default())
                    .await;
            }
        }
        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<Event>> {
        let pattern = format!("%{}%", query.to_lowercase());
        let event_ids: Vec<Uuid> = events::Entity::find()
            .filter(events::Column::DeletedAt.is_null())
            .filter(Expr::cust_with_values("LOWER(name) LIKE $1", [pattern]))
            .select_only()
            .column(events::Column::Id)
            .into_tuple()
            .all(&self.db)
            .await?;
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
