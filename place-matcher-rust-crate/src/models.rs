//! Data models for geographic places.
//!
//! This module is intentionally **logic-free**: it defines the types that
//! flow through the matching engine but contains no matching code itself.
//! See [`crate::matcher`] for the engine and [`crate::normalizer`] for the
//! text transformations that the matcher applies to these fields.
//!
//! All public types here are `Serialize + Deserialize` so they round-trip
//! through JSON, MessagePack, or any other `serde` format.
//!
//! ## Building a place
//!
//! Prefer [`Place::builder`] over constructing the struct literal — the
//! builder accepts `impl Into<String>` so call-sites can pass `&str`,
//! `String`, or owned values interchangeably.
//!
//! ```
//! use place_matcher::Place;
//!
//! let p = Place::builder()
//!     .name("Eiffel Tower")
//!     .add_alternate_name("La Tour Eiffel")
//!     .latitude(48.858_222)
//!     .longitude(2.294_500)
//!     .build();
//!
//! assert_eq!(p.name.as_deref(), Some("Eiffel Tower"));
//! ```

use serde::{Deserialize, Serialize};

/// Physical address used as supporting evidence in place matcher.
///
/// All fields are `Option<String>` so partial addresses are first-class —
/// a record with only a postcode is still useful for matching.
///
/// The matcher does **not** weight every component equally; see
/// [`crate::matcher::MatchingEngine`] for the weighted comparison rules.
///
/// # Example
///
/// ```
/// use place_matcher::Address;
///
/// let mut addr = Address::new();
/// addr.line1    = Some("10 Downing Street".into());
/// addr.city     = Some("London".into());
/// addr.postcode = Some("SW1A 2AA".into());
///
/// assert_eq!(addr.postcode.as_deref(), Some("SW1A 2AA"));
/// assert!(addr.country.is_none());
/// ```
///
/// `Address` is JSON round-trippable.
///
/// ```
/// # use place_matcher::Address;
/// let mut a = Address::new();
/// a.postcode = Some("CF10 1AA".into());
///
/// let json = serde_json::to_string(&a).unwrap();
/// let back: Address = serde_json::from_str(&json).unwrap();
/// assert_eq!(a, back);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Address {
    /// First line — typically house number and street, e.g. `"10 Downing Street"`.
    pub line1: Option<String>,
    /// Second line — typically flat, locality, or care-of details.
    pub line2: Option<String>,
    /// Town or city, e.g. `"Cardiff"`.
    pub city: Option<String>,
    /// County or administrative region, e.g. `"South Glamorgan"`.
    pub county: Option<String>,
    /// Postal code, e.g. `"CF10 1AA"`. Compared after whitespace normalisation.
    pub postcode: Option<String>,
    /// Country, e.g. `"Wales"` or `"United Kingdom"`.
    pub country: Option<String>,
}

impl Address {
    /// Construct an empty address with every field set to `None`.
    ///
    /// # Example
    ///
    /// ```
    /// use place_matcher::Address;
    ///
    /// let a = Address::new();
    /// assert!(a.line1.is_none());
    /// assert!(a.postcode.is_none());
    /// ```
    pub fn new() -> Self {
        Self {
            line1: None,
            line2: None,
            city: None,
            county: None,
            postcode: None,
            country: None,
        }
    }

    /// Fluent setter for `line1`.
    ///
    /// ```
    /// use place_matcher::Address;
    /// let a = Address::new().with_line1("10 Downing Street");
    /// assert_eq!(a.line1.as_deref(), Some("10 Downing Street"));
    /// ```
    pub fn with_line1(mut self, value: impl Into<String>) -> Self {
        self.line1 = Some(value.into());
        self
    }

    /// Fluent setter for `line2`.
    pub fn with_line2(mut self, value: impl Into<String>) -> Self {
        self.line2 = Some(value.into());
        self
    }

    /// Fluent setter for `city`.
    pub fn with_city(mut self, value: impl Into<String>) -> Self {
        self.city = Some(value.into());
        self
    }

    /// Fluent setter for `county`.
    pub fn with_county(mut self, value: impl Into<String>) -> Self {
        self.county = Some(value.into());
        self
    }

    /// Fluent setter for `postcode`.
    ///
    /// ```
    /// use place_matcher::Address;
    /// let a = Address::new().with_postcode("CF10 1AA");
    /// assert_eq!(a.postcode.as_deref(), Some("CF10 1AA"));
    /// ```
    pub fn with_postcode(mut self, value: impl Into<String>) -> Self {
        self.postcode = Some(value.into());
        self
    }

    /// Fluent setter for `country`.
    pub fn with_country(mut self, value: impl Into<String>) -> Self {
        self.country = Some(value.into());
        self
    }
}

impl Default for Address {
    /// Identical to [`Address::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// Coarse-grained category for a geographic place.
///
/// Categories are matched by structural equality. The catch-all
/// [`PlaceCategory::Other`] variant lets callers express categories that the
/// crate does not enumerate; two `Other` values match iff their carried
/// string is structurally equal (`PartialEq`).
///
/// The enum is `#[non_exhaustive]` so future variants can be added without
/// breaking SemVer for downstream pattern matches.
///
/// # Example
///
/// ```
/// use place_matcher::PlaceCategory;
///
/// assert_eq!(PlaceCategory::Hotel, PlaceCategory::Hotel);
/// assert_ne!(PlaceCategory::Hotel, PlaceCategory::Restaurant);
/// assert_eq!(
///     PlaceCategory::Other("ferry-terminal".into()),
///     PlaceCategory::Other("ferry-terminal".into())
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PlaceCategory {
    /// Lodging: hotel, inn, hostel, or similar overnight accommodation.
    Hotel,
    /// Sit-down restaurant or eatery serving prepared meals.
    Restaurant,
    /// Cafe, coffee shop, or tearoom.
    Cafe,
    /// Bar, pub, tavern, or other primarily-alcohol establishment.
    Bar,
    /// Retail shop or store selling goods to the public.
    Shop,
    /// Shopping mall or multi-tenant retail complex.
    Mall,
    /// Hospital, clinic, or other medical facility.
    Hospital,
    /// Primary, secondary, or other school (pre-tertiary).
    School,
    /// University, college, or other tertiary educational institution.
    University,
    /// Public, academic, or private library.
    Library,
    /// Museum or gallery exhibiting cultural or scientific artefacts.
    Museum,
    /// Theatre for live performance.
    Theatre,
    /// Cinema or movie theatre.
    Cinema,
    /// Park or other public green space.
    Park,
    /// Beach or coastal shoreline open to the public.
    Beach,
    /// Stadium or large outdoor sporting venue.
    Stadium,
    /// Airport or airfield.
    Airport,
    /// Railway, train, or metro station.
    RailwayStation,
    /// Bus station or coach terminal.
    BusStation,
    /// Bank branch or banking facility.
    Bank,
    /// Post office or postal branch.
    PostOffice,
    /// Government building or civic facility.
    Government,
    /// Monument, memorial, or commemorative structure.
    Monument,
    /// Place of worship: church, mosque, temple, synagogue, or similar.
    ReligiousBuilding,
    /// Cemetery, graveyard, or burial ground.
    Cemetery,
    /// Mountain, peak, or other terrain high point.
    Mountain,
    /// Lake, pond, or other enclosed body of water.
    Lake,
    /// River, stream, or other flowing watercourse.
    River,
    /// City — large populated settlement.
    City,
    /// Town — medium-sized populated settlement.
    Town,
    /// Village — small populated settlement.
    Village,
    /// Neighbourhood or district within a larger settlement.
    Neighborhood,
    /// Office building or commercial workplace.
    OfficeBuilding,
    /// Residence: house, flat, apartment, or other dwelling.
    Residence,
    /// Warehouse or storage facility.
    Warehouse,
    /// Catch-all for categories not enumerated above. The carried string
    /// participates in equality, so `Other("foo")` does not match
    /// `Other("bar")`.
    Other(String),
}

/// Issuer / scheme of a [`PlaceId`].
///
/// Identifiers from different schemes never match each other, even when
/// their string values happen to coincide — see [`PlaceId`].
///
/// The enum is `#[non_exhaustive]` so future variants can be added without
/// breaking SemVer for downstream pattern matches.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PlaceIdScheme {
    /// Google Place ID — issued by Google Maps / Places API.
    Google,
    /// OpenStreetMap node — a single point feature.
    OsmNode,
    /// OpenStreetMap way — a line or area feature.
    OsmWay,
    /// OpenStreetMap relation — a multi-feature aggregate.
    OsmRelation,
    /// GeoNames numeric identifier (`geonameid`).
    GeoNames,
    /// Wikidata QID (e.g. `Q243`).
    Wikidata,
    /// Foursquare venue identifier.
    Foursquare,
    /// HERE Technologies place identifier.
    Here,
    /// Mapbox place / feature identifier.
    Mapbox,
    /// Catch-all for schemes not enumerated above. The carried string
    /// participates in equality.
    Other(String),
}

/// External identifier for a place, scoped by its issuing scheme.
///
/// Two `PlaceId` values are equal iff both their [`PlaceIdScheme`] and
/// their `value` are equal. Equality is structural — no per-scheme
/// canonicalisation is performed.
///
/// # Example
///
/// ```
/// use place_matcher::{PlaceId, PlaceIdScheme};
///
/// let a = PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap();
/// let b = PlaceId::new(PlaceIdScheme::Wikidata, " Q243 ").unwrap();
/// assert_eq!(a, b, "values are trimmed at construction");
/// assert!(PlaceId::new(PlaceIdScheme::Wikidata, "").is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlaceId {
    /// The issuing scheme.
    pub scheme: PlaceIdScheme,
    /// The identifier value, trimmed of surrounding whitespace.
    pub value: String,
}

impl PlaceId {
    /// Construct a [`PlaceId`], trimming the value of surrounding
    /// whitespace. Returns `None` if the trimmed value is empty.
    ///
    /// No further normalisation is applied — different schemes have
    /// different rules and the crate makes no assumptions.
    ///
    /// # Example
    ///
    /// ```
    /// use place_matcher::{PlaceId, PlaceIdScheme};
    ///
    /// let id = PlaceId::new(PlaceIdScheme::Google, "  ChIJ_xyz  ").unwrap();
    /// assert_eq!(id.value, "ChIJ_xyz");
    ///
    /// assert!(PlaceId::new(PlaceIdScheme::Google, "   ").is_none());
    /// ```
    pub fn new(scheme: PlaceIdScheme, value: impl Into<String>) -> Option<Self> {
        let trimmed = value.into().trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(Self {
                scheme,
                value: trimmed,
            })
        }
    }
}

/// Core place data structure.
///
/// Every field is optional (or defaults to empty). The matcher tolerates
/// missing data field-by-field — a `None` value never penalises a place.
/// See [`crate::matcher::MatchingEngine::match_places`] for how missing
/// fields affect the weighted score.
///
/// Construct via [`Place::builder`] rather than struct literal syntax so
/// the call-site stays compact and forward-compatible if fields are added.
///
/// # Example
///
/// ```
/// use place_matcher::Place;
///
/// let p = Place::builder()
///     .name("Big Ben")
///     .add_alternate_name("Elizabeth Tower")
///     .latitude(51.500_7)
///     .longitude(-0.124_7)
///     .build();
///
/// assert_eq!(p.name.as_deref(), Some("Big Ben"));
/// assert_eq!(p.alternate_names, vec!["Elizabeth Tower".to_string()]);
/// ```
///
/// `Place` round-trips through `serde`.
///
/// ```
/// # use place_matcher::Place;
/// let p = Place::builder().name("Eiffel Tower").build();
/// let json = serde_json::to_string(&p).unwrap();
/// let back: Place = serde_json::from_str(&json).unwrap();
/// assert_eq!(p, back);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Place {
    /// Primary canonical name of the place, e.g. `"Eiffel Tower"`.
    pub name: Option<String>,

    /// Aliases, endonyms, or translations of the primary name, e.g.
    /// `vec!["La Tour Eiffel".into(), "Tour Eiffel".into()]`. Default empty.
    pub alternate_names: Vec<String>,

    /// Latitude in decimal degrees. Conventionally `[-90.0, 90.0]`. Values
    /// outside that range or non-finite values are stored as supplied for
    /// round-trip honesty, but the coordinates scorer treats them as
    /// missing.
    pub latitude: Option<f64>,

    /// Longitude in decimal degrees. Conventionally `[-180.0, 180.0]`.
    /// Values outside that range or non-finite values are stored as
    /// supplied for round-trip honesty, but the coordinates scorer treats
    /// them as missing.
    pub longitude: Option<f64>,

    /// Coarse-grained category of the place (see [`PlaceCategory`]).
    pub category: Option<PlaceCategory>,

    /// External scheme-scoped identifiers for the place. Sharing any one
    /// `(scheme, value)` pair across two places is a deterministic match.
    pub place_ids: Vec<PlaceId>,

    /// Postal address of the place, when applicable.
    pub address: Option<Address>,

    /// Primary contact phone number for the place.
    pub phone: Option<String>,

    /// Primary contact email address for the place.
    pub email: Option<String>,

    /// Local identifier issued by the originating system. Not normalised —
    /// different organisations may issue colliding values.
    pub local_id: Option<String>,

    /// Altitude in metres relative to mean sea level. May be negative
    /// (e.g. the Dead Sea shoreline is approximately -430 m).
    /// Conventionally used for objects positioned above the surface (a
    /// building's rooftop, an aircraft's cruise level); for the height of
    /// a fixed point on the ground, prefer [`Self::elevation_as_metre`].
    pub altitude_as_metre: Option<f64>,

    /// Elevation in metres relative to mean sea level of the place's
    /// ground-surface reference point. May be negative. Conventionally
    /// used for fixed terrestrial features (a mountain summit, a town
    /// centre).
    pub elevation_as_metre: Option<f64>,

    /// Area in square metres (m^2). For example, a 1 ha park is
    /// `Some(10_000.0)`; the area of Wales is roughly `Some(2.08e10)`.
    pub area_as_metre_2: Option<f64>,

    /// Country code in ISO 3166-1 alpha-2 form, e.g. `"GB"`, `"US"`,
    /// `"FR"`. Stored as supplied — no canonicalisation is performed
    /// at construction time. The matcher compares case-insensitively
    /// after trimming whitespace.
    pub country_code_as_iso_3166_1_alpha_2: Option<String>,

    /// Maximum number of people the place can hold (e.g. legal
    /// occupancy of a venue, seating capacity of a stadium). `0` is a
    /// meaningful value (a place that holds nobody); absence is
    /// `None`.
    pub maximum_capacity_count: Option<u32>,
}

impl Place {
    /// Begin constructing a [`Place`] with the [`PlaceBuilder`].
    ///
    /// All fields default to `None` / empty until a setter is called.
    ///
    /// # Example
    ///
    /// ```
    /// use place_matcher::Place;
    ///
    /// let p = Place::builder()
    ///     .name("Big Ben")
    ///     .build();
    ///
    /// assert_eq!(p.name.as_deref(), Some("Big Ben"));
    /// ```
    pub fn builder() -> PlaceBuilder {
        PlaceBuilder::default()
    }

    /// Validate that the place carries a primary name.
    ///
    /// Returns `Ok(())` if `name` is set. Otherwise returns
    /// [`crate::MatchingError::MissingField`].
    ///
    /// This is **not** invoked automatically by the matcher — call it at the
    /// system boundary when you ingest data, not on every comparison.
    ///
    /// # Example
    ///
    /// ```
    /// use place_matcher::Place;
    ///
    /// assert!(Place::builder().name("Eiffel Tower").build().validate().is_ok());
    /// assert!(Place::builder().build().validate().is_err());
    /// ```
    pub fn validate(&self) -> crate::Result<()> {
        if self.name.is_none() {
            return Err(crate::MatchingError::MissingField(
                "name is required".to_string(),
            ));
        }
        Ok(())
    }
}

/// Fluent builder for [`Place`].
///
/// All string setters accept `impl Into<String>` so call-sites may pass
/// `&str`, `String`, or `&String` interchangeably without explicit
/// conversion.
///
/// # Example
///
/// ```
/// use place_matcher::{Place, PlaceBuilder, PlaceCategory};
///
/// let p: Place = PlaceBuilder::default()
///     .name(String::from("Eiffel Tower"))
///     .add_alternate_name("La Tour Eiffel")
///     .latitude(48.858_222)
///     .longitude(2.294_500)
///     .category(PlaceCategory::Monument)
///     .build();
///
/// assert_eq!(p.name.as_deref(), Some("Eiffel Tower"));
/// assert_eq!(p.category, Some(PlaceCategory::Monument));
/// ```
#[derive(Default)]
pub struct PlaceBuilder {
    name: Option<String>,
    alternate_names: Vec<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    category: Option<PlaceCategory>,
    place_ids: Vec<PlaceId>,
    address: Option<Address>,
    phone: Option<String>,
    email: Option<String>,
    local_id: Option<String>,
    altitude_as_metre: Option<f64>,
    elevation_as_metre: Option<f64>,
    area_as_metre_2: Option<f64>,
    country_code_as_iso_3166_1_alpha_2: Option<String>,
    maximum_capacity_count: Option<u32>,
}

impl PlaceBuilder {
    /// Set the primary canonical name.
    ///
    /// ```
    /// # use place_matcher::Place;
    /// let p = Place::builder().name("Eiffel Tower").build();
    /// assert_eq!(p.name.as_deref(), Some("Eiffel Tower"));
    /// ```
    pub fn name<S: Into<String>>(mut self, value: S) -> Self {
        self.name = Some(value.into());
        self
    }

    /// Replace the entire list of alternate names.
    ///
    /// ```
    /// # use place_matcher::Place;
    /// let p = Place::builder()
    ///     .alternate_names(vec!["La Tour Eiffel".into(), "Tour Eiffel".into()])
    ///     .build();
    /// assert_eq!(p.alternate_names.len(), 2);
    /// ```
    pub fn alternate_names(mut self, value: Vec<String>) -> Self {
        self.alternate_names = value;
        self
    }

    /// Append a single alternate name.
    ///
    /// ```
    /// # use place_matcher::Place;
    /// let p = Place::builder()
    ///     .add_alternate_name("La Tour Eiffel")
    ///     .add_alternate_name("Tour Eiffel")
    ///     .build();
    /// assert_eq!(p.alternate_names.len(), 2);
    /// ```
    pub fn add_alternate_name<S: Into<String>>(mut self, value: S) -> Self {
        self.alternate_names.push(value.into());
        self
    }

    /// Set the latitude in decimal degrees.
    ///
    /// ```
    /// # use place_matcher::Place;
    /// let p = Place::builder().latitude(48.858_222).build();
    /// assert_eq!(p.latitude, Some(48.858_222));
    /// ```
    pub fn latitude(mut self, value: f64) -> Self {
        self.latitude = Some(value);
        self
    }

    /// Set the longitude in decimal degrees.
    ///
    /// ```
    /// # use place_matcher::Place;
    /// let p = Place::builder().longitude(2.294_500).build();
    /// assert_eq!(p.longitude, Some(2.294_500));
    /// ```
    pub fn longitude(mut self, value: f64) -> Self {
        self.longitude = Some(value);
        self
    }

    /// Set the place's category.
    ///
    /// ```
    /// # use place_matcher::{Place, PlaceCategory};
    /// let p = Place::builder().category(PlaceCategory::Museum).build();
    /// assert_eq!(p.category, Some(PlaceCategory::Museum));
    /// ```
    pub fn category(mut self, value: PlaceCategory) -> Self {
        self.category = Some(value);
        self
    }

    /// Replace the entire list of external place IDs.
    ///
    /// ```
    /// # use place_matcher::{Place, PlaceId, PlaceIdScheme};
    /// let id = PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap();
    /// let p = Place::builder().place_ids(vec![id.clone()]).build();
    /// assert_eq!(p.place_ids, vec![id]);
    /// ```
    pub fn place_ids(mut self, value: Vec<PlaceId>) -> Self {
        self.place_ids = value;
        self
    }

    /// Append a single external place ID.
    ///
    /// ```
    /// # use place_matcher::{Place, PlaceId, PlaceIdScheme};
    /// let p = Place::builder()
    ///     .add_place_id(PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap())
    ///     .build();
    /// assert_eq!(p.place_ids.len(), 1);
    /// ```
    pub fn add_place_id(mut self, value: PlaceId) -> Self {
        self.place_ids.push(value);
        self
    }

    /// Set the postal address.
    ///
    /// ```
    /// # use place_matcher::{Address, Place};
    /// let mut a = Address::new();
    /// a.postcode = Some("CF10 1AA".into());
    /// let p = Place::builder().address(a).build();
    /// assert_eq!(p.address.unwrap().postcode.as_deref(), Some("CF10 1AA"));
    /// ```
    pub fn address(mut self, value: Address) -> Self {
        self.address = Some(value);
        self
    }

    /// Set the primary phone number.
    ///
    /// ```
    /// # use place_matcher::Place;
    /// let p = Place::builder().phone("029 2034 5678").build();
    /// assert_eq!(p.phone.as_deref(), Some("029 2034 5678"));
    /// ```
    pub fn phone<S: Into<String>>(mut self, value: S) -> Self {
        self.phone = Some(value.into());
        self
    }

    /// Set the email address.
    ///
    /// ```
    /// # use place_matcher::Place;
    /// let p = Place::builder().email("info@example.org").build();
    /// assert_eq!(p.email.as_deref(), Some("info@example.org"));
    /// ```
    pub fn email<S: Into<String>>(mut self, value: S) -> Self {
        self.email = Some(value.into());
        self
    }

    /// Set the local identifier.
    ///
    /// ```
    /// # use place_matcher::Place;
    /// let p = Place::builder().local_id("REF-12345").build();
    /// assert_eq!(p.local_id.as_deref(), Some("REF-12345"));
    /// ```
    pub fn local_id<S: Into<String>>(mut self, value: S) -> Self {
        self.local_id = Some(value.into());
        self
    }

    /// Set the altitude in metres above mean sea level.
    ///
    /// ```
    /// # use place_matcher::Place;
    /// let p = Place::builder().altitude_as_metre(120.5).build();
    /// assert_eq!(p.altitude_as_metre, Some(120.5));
    /// ```
    pub fn altitude_as_metre(mut self, value: f64) -> Self {
        self.altitude_as_metre = Some(value);
        self
    }

    /// Set the elevation in metres above mean sea level.
    ///
    /// ```
    /// # use place_matcher::Place;
    /// let p = Place::builder().elevation_as_metre(-430.0).build();
    /// assert_eq!(p.elevation_as_metre, Some(-430.0));
    /// ```
    pub fn elevation_as_metre(mut self, value: f64) -> Self {
        self.elevation_as_metre = Some(value);
        self
    }

    /// Set the area in square metres.
    ///
    /// ```
    /// # use place_matcher::Place;
    /// let p = Place::builder().area_as_metre_2(10_000.0).build();
    /// assert_eq!(p.area_as_metre_2, Some(10_000.0));
    /// ```
    pub fn area_as_metre_2(mut self, value: f64) -> Self {
        self.area_as_metre_2 = Some(value);
        self
    }

    /// Set the country code in ISO 3166-1 alpha-2 form.
    ///
    /// ```
    /// # use place_matcher::Place;
    /// let p = Place::builder().country_code_as_iso_3166_1_alpha_2("GB").build();
    /// assert_eq!(p.country_code_as_iso_3166_1_alpha_2.as_deref(), Some("GB"));
    /// ```
    pub fn country_code_as_iso_3166_1_alpha_2<S: Into<String>>(mut self, value: S) -> Self {
        self.country_code_as_iso_3166_1_alpha_2 = Some(value.into());
        self
    }

    /// Set the maximum number of people the place can hold.
    ///
    /// ```
    /// # use place_matcher::Place;
    /// let p = Place::builder().maximum_capacity_count(82_500).build();
    /// assert_eq!(p.maximum_capacity_count, Some(82_500));
    /// ```
    pub fn maximum_capacity_count(mut self, value: u32) -> Self {
        self.maximum_capacity_count = Some(value);
        self
    }

    /// Consume the builder and produce the [`Place`].
    ///
    /// ```
    /// # use place_matcher::Place;
    /// let p = Place::builder().name("Big Ben").build();
    /// assert!(p.latitude.is_none());
    /// ```
    pub fn build(self) -> Place {
        Place {
            name: self.name,
            alternate_names: self.alternate_names,
            latitude: self.latitude,
            longitude: self.longitude,
            category: self.category,
            place_ids: self.place_ids,
            address: self.address,
            phone: self.phone,
            email: self.email,
            local_id: self.local_id,
            altitude_as_metre: self.altitude_as_metre,
            elevation_as_metre: self.elevation_as_metre,
            area_as_metre_2: self.area_as_metre_2,
            country_code_as_iso_3166_1_alpha_2: self.country_code_as_iso_3166_1_alpha_2,
            maximum_capacity_count: self.maximum_capacity_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_new_is_all_none() {
        let a = Address::new();
        assert!(a.line1.is_none());
        assert!(a.line2.is_none());
        assert!(a.city.is_none());
        assert!(a.county.is_none());
        assert!(a.postcode.is_none());
        assert!(a.country.is_none());
    }

    #[test]
    fn address_default_matches_new() {
        assert_eq!(Address::default(), Address::new());
    }

    #[test]
    fn address_fluent_builders_chain() {
        let a = Address::new()
            .with_line1("10 Downing Street")
            .with_city("London")
            .with_postcode("SW1A 2AA")
            .with_country("United Kingdom");
        assert_eq!(a.line1.as_deref(), Some("10 Downing Street"));
        assert_eq!(a.city.as_deref(), Some("London"));
        assert_eq!(a.postcode.as_deref(), Some("SW1A 2AA"));
        assert_eq!(a.country.as_deref(), Some("United Kingdom"));
        assert!(a.line2.is_none());
        assert!(a.county.is_none());
    }

    #[test]
    fn address_round_trips_through_serde() {
        let mut a = Address::new();
        a.line1 = Some("123 High Street".into());
        a.postcode = Some("CF10 1AA".into());
        let json = serde_json::to_string(&a).expect("serialise");
        let back: Address = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(a, back);
    }

    #[test]
    fn place_builder_starts_empty() {
        let p = Place::builder().build();
        assert!(p.name.is_none());
        assert!(p.alternate_names.is_empty());
        assert!(p.latitude.is_none());
        assert!(p.longitude.is_none());
        assert!(p.category.is_none());
        assert!(p.place_ids.is_empty());
        assert!(p.address.is_none());
        assert!(p.phone.is_none());
        assert!(p.email.is_none());
        assert!(p.local_id.is_none());
        assert!(p.altitude_as_metre.is_none());
        assert!(p.elevation_as_metre.is_none());
        assert!(p.area_as_metre_2.is_none());
        assert!(p.country_code_as_iso_3166_1_alpha_2.is_none());
        assert!(p.maximum_capacity_count.is_none());
    }

    #[test]
    fn place_builder_accepts_geographic_fields() {
        let p = Place::builder()
            .name("Wembley Stadium")
            .altitude_as_metre(48.0)
            .elevation_as_metre(45.5)
            .area_as_metre_2(40_000.0)
            .country_code_as_iso_3166_1_alpha_2("GB")
            .maximum_capacity_count(90_000)
            .build();
        assert_eq!(p.altitude_as_metre, Some(48.0));
        assert_eq!(p.elevation_as_metre, Some(45.5));
        assert_eq!(p.area_as_metre_2, Some(40_000.0));
        assert_eq!(
            p.country_code_as_iso_3166_1_alpha_2.as_deref(),
            Some("GB"),
        );
        assert_eq!(p.maximum_capacity_count, Some(90_000));
    }

    #[test]
    fn place_builder_accepts_str_and_string() {
        let p = Place::builder()
            .name("Eiffel Tower")
            .add_alternate_name(String::from("La Tour Eiffel"))
            .build();
        assert_eq!(p.name.as_deref(), Some("Eiffel Tower"));
        assert_eq!(p.alternate_names, vec!["La Tour Eiffel".to_string()]);
    }

    #[test]
    fn place_validate_requires_a_name() {
        assert!(Place::builder().name("Eiffel Tower").build().validate().is_ok());
        let err = Place::builder()
            .build()
            .validate()
            .expect_err("should be missing");
        assert!(matches!(err, crate::MatchingError::MissingField(_)));
    }

    #[test]
    fn place_round_trips_through_serde() {
        let p = Place::builder()
            .name("Eiffel Tower")
            .add_alternate_name("La Tour Eiffel")
            .latitude(48.858_222)
            .longitude(2.294_500)
            .category(PlaceCategory::Monument)
            .add_place_id(PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap())
            .country_code_as_iso_3166_1_alpha_2("FR")
            .build();
        let json = serde_json::to_string(&p).expect("serialise");
        let back: Place = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(p, back);
    }

    #[test]
    fn alternate_names_setter_replaces_vec() {
        let p = Place::builder()
            .alternate_names(vec!["X".into(), "Y".into()])
            .build();
        assert_eq!(p.alternate_names, vec!["X".to_string(), "Y".to_string()]);
    }

    #[test]
    fn place_id_trims_value() {
        let id = PlaceId::new(PlaceIdScheme::Google, "   abc ").unwrap();
        assert_eq!(id.value, "abc");
    }

    #[test]
    fn place_id_rejects_empty() {
        assert!(PlaceId::new(PlaceIdScheme::Google, "").is_none());
        assert!(PlaceId::new(PlaceIdScheme::Google, "    ").is_none());
    }

    #[test]
    fn place_id_equality_is_scheme_scoped() {
        let g = PlaceId::new(PlaceIdScheme::Google, "X").unwrap();
        let o = PlaceId::new(PlaceIdScheme::OsmNode, "X").unwrap();
        assert_ne!(g, o);
    }

    #[test]
    fn place_id_scheme_other_compares_by_string() {
        let a = PlaceId::new(PlaceIdScheme::Other("foo".into()), "1").unwrap();
        let b = PlaceId::new(PlaceIdScheme::Other("bar".into()), "1").unwrap();
        let c = PlaceId::new(PlaceIdScheme::Other("foo".into()), "1").unwrap();
        assert_ne!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn category_other_compares_by_string() {
        assert_ne!(
            PlaceCategory::Other("foo".into()),
            PlaceCategory::Other("bar".into())
        );
        assert_eq!(
            PlaceCategory::Other("foo".into()),
            PlaceCategory::Other("foo".into())
        );
    }
}
