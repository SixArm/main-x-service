//! Client for the Main Place Service.
//!
//! The Main Place Service is an external Rust service that owns the
//! registry of physical places — buildings, rooms, and cabinets — that
//! house paper case-note folders. We talk to it over HTTP/JSON (REST).
//! This crate keeps no place tables of its own; folders carry the
//! opaque cabinet UUID issued by the service plus a denormalised
//! `cabinet_path_snapshot` so the UI can render lists without a network
//! round-trip per row.
//!
//! Places are arranged hierarchically via [`Place::contained_in_place`].
//! In this app's vocabulary:
//!
//! - A **building** is a `Place` with `place_type = Hospital` and no parent.
//! - A **room**  is a `Place` with `place_type = Other("RecordsRoom")` and a building parent.
//! - A **cabinet** is a `Place` with `place_type = Other("FileCabinet")` and a room parent.
//!
//! These conventions are encoded by [`PlaceType::HOSPITAL`],
//! [`PlaceType::RECORDS_ROOM`], and [`PlaceType::FILE_CABINET`] below.
//!
//! The [`Client`] trait keeps controllers and tests decoupled from any
//! particular transport. The HTTP implementation lives in
//! [`http`](self::http); tests use [`stub::StubClient`].

pub mod http;
pub mod stub;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Place type variants. The Main Place Service uses a `PlaceType` enum
/// with a tagged `Other(String)` escape hatch; the tracker only cares
/// about three values, encoded as string constants below.
pub struct PlaceType;
impl PlaceType {
    pub const HOSPITAL: &'static str = "Hospital";
    pub const RECORDS_ROOM: &'static str = "RecordsRoom";
    pub const FILE_CABINET: &'static str = "FileCabinet";
}

/// Minimal place projection used by Case Tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    pub id: Uuid,
    pub name: String,
    /// Free-string place type. `Hospital` for buildings,
    /// `RecordsRoom` for rooms, `FileCabinet` for cabinets, anything else
    /// for unrelated places.
    pub place_type: Option<String>,
    pub description: Option<String>,
    /// Parent place UUID — the room a cabinet sits in, the building a
    /// room sits in.
    pub contained_in_place: Option<Uuid>,
    /// Approximate folder capacity. Not modelled by the upstream Place
    /// schema directly; the tracker reads it from a custom keyword like
    /// `capacity=120` when present.
    #[serde(default)]
    pub capacity: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct CreatePlace {
    pub name: String,
    pub place_type: String,
    pub description: Option<String>,
    pub contained_in_place: Option<Uuid>,
    pub capacity: Option<i32>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Main Place Service request failed: {0}")]
    Transport(String),
    #[error("Main Place Service returned an unexpected response: {0}")]
    BadResponse(String),
    #[error("Main Place Service did not find a place")]
    NotFound,
}

impl From<Error> for loco_rs::Error {
    fn from(value: Error) -> Self {
        loco_rs::Error::Message(value.to_string())
    }
}

#[async_trait]
pub trait Client: Send + Sync {
    /// Free-text name search, optionally filtered by `place_type`.
    async fn search(&self, query: &str, place_type: Option<&str>) -> Result<Vec<Place>, Error>;

    /// Look up a single place by UUID.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Place>, Error>;

    /// Register a new place. Used by the seed task to populate the demo
    /// hierarchy.
    async fn create(&self, input: CreatePlace) -> Result<Place, Error>;
}

/// Walk the containment chain to build a human-readable path like
/// `"Main Hospital — Ward A Records Room — Cabinet A1"`. Stops at the
/// first place with no parent or after 4 hops (depth guard).
pub async fn label_path(client: &dyn Client, cabinet_id: Uuid) -> Result<String, Error> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = Some(cabinet_id);
    let mut depth = 0;
    while let Some(id) = current {
        if depth > 4 {
            break;
        }
        match client.find_by_id(id).await? {
            Some(place) => {
                parts.push(place.name.clone());
                current = place.contained_in_place;
            }
            None => break,
        }
        depth += 1;
    }
    parts.reverse();
    Ok(if parts.is_empty() {
        String::new()
    } else {
        parts.join(" — ")
    })
}
