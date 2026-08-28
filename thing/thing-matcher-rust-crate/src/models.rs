//! Data models for things, aligned with `schema.org/Thing`.
//!
//! This module is intentionally **logic-free**: it defines the types that
//! flow through the matching engine but contains no matching code itself.
//! See [`crate::matcher`] for the engine and [`crate::normalizer`] for the
//! text transformations that the matcher applies to these fields.
//!
//! All public types here are `Serialize + Deserialize` so they round-trip
//! through JSON, `MessagePack`, or any other `serde` format.
//!
//! ## Schema.org alignment
//!
//! The fields of [`Thing`] correspond to the properties of
//! `schema.org/Thing`:
//!
//! | Rust field | schema.org property |
//! |---|---|
//! | `name` | `name` |
//! | `alternate_names` | `alternateName` |
//! | `description` | `description` |
//! | `disambiguating_description` | `disambiguatingDescription` |
//! | `identifiers` | `identifier` (as `PropertyValue`) |
//! | `url` | `url` |
//! | `image` | `image` |
//! | `same_as` | `sameAs` |
//! | `main_entity_of_page` | `mainEntityOfPage` |
//! | `additional_types` | `additionalType` |
//! | `subject_of` | `subjectOf` |
//! | `owner` | `owner` |
//!
//! ## Building a thing
//!
//! Prefer [`Thing::builder`] over constructing the struct literal — the
//! builder accepts `impl Into<String>` so call-sites can pass `&str`,
//! `String`, or owned values interchangeably.
//!
//! ```
//! use thing_matcher::Thing;
//!
//! let t = Thing::builder()
//!     .name("Eiffel Tower")
//!     .add_alternate_name("La Tour Eiffel")
//!     .url("https://www.toureiffel.paris/")
//!     .build();
//!
//! assert_eq!(t.name.as_deref(), Some("Eiffel Tower"));
//! ```

use serde::{Deserialize, Serialize};

/// External identifier for a thing, modelled on `schema.org/PropertyValue`.
///
/// Two `Identifier` values are equal iff both their `property_id` and their
/// `value` are equal. Equality is structural — no per-scheme
/// canonicalisation is performed.
///
/// `property_id` is the issuer or vocabulary that names the identifier,
/// such as `"wikidata"`, `"isbn"`, `"doi"`, `"gtin"`, or a fully-qualified
/// URL. `value` is the identifier string itself.
///
/// # Example
///
/// ```
/// use thing_matcher::Identifier;
///
/// let a = Identifier::new("wikidata", "Q243").unwrap();
/// let b = Identifier::new("wikidata", " Q243 ").unwrap();
/// assert_eq!(a, b, "values are trimmed at construction");
/// assert!(Identifier::new("wikidata", "").is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Identifier {
    /// Issuer or vocabulary that scopes the identifier, e.g.
    /// `"wikidata"`, `"isbn"`, `"doi"`, `"gtin"`, or a URL.
    pub property_id: String,
    /// The identifier value, trimmed of surrounding whitespace.
    pub value: String,
}

impl Identifier {
    /// Construct an [`Identifier`], trimming the value and property id of
    /// surrounding whitespace. Returns `None` if either trimmed component
    /// is empty.
    ///
    /// No further normalisation is applied — different vocabularies have
    /// different rules and the crate makes no assumptions.
    ///
    /// # Example
    ///
    /// ```
    /// use thing_matcher::Identifier;
    ///
    /// let id = Identifier::new("wikidata", "  Q243  ").unwrap();
    /// assert_eq!(id.value, "Q243");
    /// assert_eq!(id.property_id, "wikidata");
    ///
    /// assert!(Identifier::new("wikidata", "   ").is_none());
    /// assert!(Identifier::new("   ", "Q243").is_none());
    /// ```
    pub fn new(property_id: impl Into<String>, value: impl Into<String>) -> Option<Self> {
        let property_id = property_id.into().trim().to_string();
        let value = value.into().trim().to_string();
        if property_id.is_empty() || value.is_empty() {
            None
        } else {
            Some(Self { property_id, value })
        }
    }
}

/// The kind of typed relationship one [`Thing`] asserts toward another,
/// carried on a [`RelationshipRef`].
///
/// `Contains` / `ContainedIn` are containment inverses — if thing A holds
/// `Contains` pointing at thing B, the converse fact is that B is
/// `ContainedIn` A — and `SuperPart` / `SubPart` are part-of inverses
/// (schema.org `hasPart` / `isPartOf`: A `SuperPart` B means A is the
/// whole and B the sub-part). The matcher does **not** resolve, invert,
/// or transitively close these references; it only compares the raw
/// `(relation, thing_id)` pairs each side asserts (spec §5.9.1). The enum
/// is `#[non_exhaustive]` because it is expected to grow (e.g.
/// `SimilarTo`, `Replaces` / `ReplacedBy`) as consuming services add
/// relationship kinds; a new variant is purely additive.
///
/// # Example
///
/// ```
/// use thing_matcher::RelationKind;
///
/// let k = RelationKind::Contains;
/// assert_eq!(k, RelationKind::Contains);
/// assert_ne!(k, RelationKind::ContainedIn);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RelationKind {
    /// This thing contains the referenced thing.
    Contains,
    /// This thing is contained in the referenced thing.
    ContainedIn,
    /// This thing is the whole of which the referenced thing is a part
    /// (schema.org `hasPart`).
    SuperPart,
    /// This thing is a part of the referenced thing (schema.org
    /// `isPartOf`).
    SubPart,
}

/// A typed reference from one [`Thing`] to another, by opaque id in the
/// consuming registry (e.g. "this thing contains thing `X`").
///
/// `RelationshipRef` is a **supporting** matching signal, not an
/// identifying one: the matcher never resolves `thing_id` against a
/// registry (it has none) — it only compares the two things'
/// relationship **sets** via typed-set Jaccard over `(relation,
/// thing_id)` pairs (spec §5.9.1 / §6.6).
///
/// Construct via [`RelationshipRef::new`], which trims `thing_id` and
/// rejects an empty result, so two records carrying different incidental
/// whitespace around the same id compare equal.
///
/// # Example
///
/// ```
/// use thing_matcher::{RelationKind, RelationshipRef};
///
/// let r = RelationshipRef::new(RelationKind::Contains, " thing-42 ").unwrap();
/// assert_eq!(r.thing_id, "thing-42");
/// assert_eq!(r.relation, RelationKind::Contains);
///
/// assert!(RelationshipRef::new(RelationKind::ContainedIn, "   ").is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationshipRef {
    /// The kind of relationship this side asserts. See [`RelationKind`].
    pub relation: RelationKind,
    /// Opaque id of the related thing in the consuming registry.
    /// Whitespace-trimmed and non-empty — see [`RelationshipRef::new`].
    pub thing_id: String,
}

impl RelationshipRef {
    /// Construct a relationship reference, trimming `thing_id` and
    /// rejecting an empty result.
    ///
    /// Returns `None` when `thing_id` is empty after trimming.
    ///
    /// ```
    /// use thing_matcher::{RelationKind, RelationshipRef};
    /// let r = RelationshipRef::new(RelationKind::SubPart, "thing-7").unwrap();
    /// assert_eq!(r.thing_id, "thing-7");
    /// assert!(RelationshipRef::new(RelationKind::SubPart, "").is_none());
    /// ```
    #[must_use]
    pub fn new(relation: RelationKind, thing_id: impl AsRef<str>) -> Option<Self> {
        let thing_id = thing_id.as_ref().trim();
        if thing_id.is_empty() {
            return None;
        }
        Some(Self {
            relation,
            thing_id: thing_id.to_string(),
        })
    }
}

/// Core data structure for a thing, aligned with `schema.org/Thing`.
///
/// Every field is optional (or defaults to empty). The matcher tolerates
/// missing data field-by-field — a `None` value never penalises a thing.
/// See [`crate::matcher::MatchingEngine::match_things`] for how missing
/// fields affect the weighted score.
///
/// Construct via [`Thing::builder`] rather than struct literal syntax so
/// the call-site stays compact and forward-compatible if fields are added.
///
/// # Example
///
/// ```
/// use thing_matcher::Thing;
///
/// let t = Thing::builder()
///     .name("Eiffel Tower")
///     .add_alternate_name("La Tour Eiffel")
///     .description("Wrought-iron lattice tower on the Champ de Mars in Paris.")
///     .url("https://www.toureiffel.paris/")
///     .build();
///
/// assert_eq!(t.name.as_deref(), Some("Eiffel Tower"));
/// assert_eq!(t.alternate_names, vec!["La Tour Eiffel".to_string()]);
/// ```
///
/// `Thing` round-trips through `serde`.
///
/// ```
/// # use thing_matcher::Thing;
/// let t = Thing::builder().name("Eiffel Tower").build();
/// let json = serde_json::to_string(&t).unwrap();
/// let back: Thing = serde_json::from_str(&json).unwrap();
/// assert_eq!(t, back);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Thing {
    /// Primary canonical name. Corresponds to `schema.org/name`.
    pub name: Option<String>,

    /// Aliases, endonyms, or translations. Corresponds to
    /// `schema.org/alternateName`. The matcher takes the best score
    /// across the cartesian product of primary + alternates.
    pub alternate_names: Vec<String>,

    /// Free-form description. Corresponds to `schema.org/description`.
    pub description: Option<String>,

    /// Short disambiguating description. Corresponds to
    /// `schema.org/disambiguatingDescription`.
    pub disambiguating_description: Option<String>,

    /// Scheme-scoped external identifiers. Corresponds to
    /// `schema.org/identifier` modelled as `PropertyValue`. Sharing any
    /// one `(property_id, value)` pair across two things is a
    /// deterministic match.
    pub identifiers: Vec<Identifier>,

    /// Canonical URL of the item. Corresponds to `schema.org/url`.
    pub url: Option<String>,

    /// URL of a representative image. Corresponds to `schema.org/image`.
    pub image: Option<String>,

    /// Reference URLs that unambiguously indicate the same item, e.g. a
    /// Wikipedia article or an authority record. Corresponds to
    /// `schema.org/sameAs`.
    pub same_as: Vec<String>,

    /// Page (URL) for which this thing is the main entity. Corresponds
    /// to `schema.org/mainEntityOfPage`.
    pub main_entity_of_page: Option<String>,

    /// Additional types from external vocabularies, typically schema.org
    /// subtypes or other ontology URIs. Corresponds to
    /// `schema.org/additionalType`.
    pub additional_types: Vec<String>,

    /// Works or events about this thing (URLs). Corresponds to
    /// `schema.org/subjectOf`.
    pub subject_of: Vec<String>,

    /// Person or organisation that owns this thing. Corresponds to
    /// `schema.org/owner`. Stored as a string (a name or URL) — the
    /// crate does not model `Person` / `Organization` separately.
    pub owner: Option<String>,

    /// Local identifier issued by the originating system. Not
    /// normalised, not scored — different organisations may issue
    /// colliding values. Kept for round-trip honesty.
    pub local_id: Option<String>,

    /// Typed references to other things (by opaque id) in the consuming
    /// registry — e.g. "this thing contains thing X". A **supporting**
    /// signal only: never identifying on its own, and the matcher never
    /// resolves the reference (it has no registry) — it only compares
    /// the two things' relationship **sets** via typed-set Jaccard.
    /// Default empty. See [`RelationshipRef`] / [`RelationKind`]; spec
    /// §3.3.1 / §5.9.1 / §6.6.
    #[serde(default)]
    pub relationships: Vec<RelationshipRef>,

    /// Operator-applied free-text labels (e.g. `"landmark"`,
    /// `"unesco"`). Stored verbatim; compared case-insensitively at
    /// match time, consistent with the crate's normalise-at-match-time
    /// convention. A **supporting** signal only: never identifying on
    /// its own. Default empty. See spec §3.1 / §5.9.2 / §6.8.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Thing {
    /// Begin constructing a [`Thing`] with the [`ThingBuilder`].
    ///
    /// All fields default to `None` / empty until a setter is called.
    ///
    /// # Example
    ///
    /// ```
    /// use thing_matcher::Thing;
    ///
    /// let t = Thing::builder()
    ///     .name("Big Ben")
    ///     .build();
    ///
    /// assert_eq!(t.name.as_deref(), Some("Big Ben"));
    /// ```
    #[must_use]
    pub fn builder() -> ThingBuilder {
        ThingBuilder::default()
    }

    /// Validate that the thing carries a primary name.
    ///
    /// Returns `Ok(())` if `name` is set. Otherwise returns
    /// [`crate::MatchingError::MissingField`].
    ///
    /// This is **not** invoked automatically by the matcher — call it at
    /// the system boundary when you ingest data, not on every
    /// comparison.
    ///
    /// # Errors
    ///
    /// Returns [`crate::MatchingError::MissingField`] when `name` is absent.
    ///
    /// # Example
    ///
    /// ```
    /// use thing_matcher::Thing;
    ///
    /// assert!(Thing::builder().name("Eiffel Tower").build().validate().is_ok());
    /// assert!(Thing::builder().build().validate().is_err());
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

/// Fluent builder for [`Thing`].
///
/// All string setters accept `impl Into<String>` so call-sites may pass
/// `&str`, `String`, or `&String` interchangeably without explicit
/// conversion.
///
/// # Example
///
/// ```
/// use thing_matcher::{Thing, ThingBuilder};
///
/// let t: Thing = ThingBuilder::default()
///     .name(String::from("Eiffel Tower"))
///     .add_alternate_name("La Tour Eiffel")
///     .url("https://www.toureiffel.paris/")
///     .add_same_as("https://www.wikidata.org/wiki/Q243")
///     .build();
///
/// assert_eq!(t.name.as_deref(), Some("Eiffel Tower"));
/// assert_eq!(t.same_as.len(), 1);
/// ```
#[derive(Default)]
pub struct ThingBuilder {
    // Each field mirrors the matching [`Thing`] field it will populate in
    // [`ThingBuilder::build`]. All default to `None` / empty (via
    // `#[derive(Default)]`) so an unset field never appears as data on the
    // built thing.
    /// Pending primary canonical name; populates [`Thing::name`].
    name: Option<String>,
    /// Pending alternate names; populates [`Thing::alternate_names`].
    alternate_names: Vec<String>,
    /// Pending free-form description; populates [`Thing::description`].
    description: Option<String>,
    /// Pending disambiguating description; populates
    /// [`Thing::disambiguating_description`].
    disambiguating_description: Option<String>,
    /// Pending external identifiers; populates [`Thing::identifiers`].
    identifiers: Vec<Identifier>,
    /// Pending canonical URL; populates [`Thing::url`].
    url: Option<String>,
    /// Pending representative image URL; populates [`Thing::image`].
    image: Option<String>,
    /// Pending `sameAs` reference URLs; populates [`Thing::same_as`].
    same_as: Vec<String>,
    /// Pending `mainEntityOfPage` URL; populates
    /// [`Thing::main_entity_of_page`].
    main_entity_of_page: Option<String>,
    /// Pending `additionalType` URIs; populates [`Thing::additional_types`].
    additional_types: Vec<String>,
    /// Pending `subjectOf` URLs; populates [`Thing::subject_of`].
    subject_of: Vec<String>,
    /// Pending owner string; populates [`Thing::owner`].
    owner: Option<String>,
    /// Pending originating-system local id; populates [`Thing::local_id`].
    local_id: Option<String>,
    /// Pending relationship references; populates [`Thing::relationships`].
    relationships: Vec<RelationshipRef>,
    /// Pending operator tags; populates [`Thing::tags`].
    tags: Vec<String>,
}

impl ThingBuilder {
    /// Set the primary canonical name.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder().name("Eiffel Tower").build();
    /// assert_eq!(t.name.as_deref(), Some("Eiffel Tower"));
    /// ```
    #[must_use]
    pub fn name<S: Into<String>>(mut self, value: S) -> Self {
        self.name = Some(value.into());
        self
    }

    /// Replace the entire list of alternate names.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder()
    ///     .alternate_names(vec!["La Tour Eiffel".into(), "Tour Eiffel".into()])
    ///     .build();
    /// assert_eq!(t.alternate_names.len(), 2);
    /// ```
    #[must_use]
    pub fn alternate_names(mut self, value: Vec<String>) -> Self {
        self.alternate_names = value;
        self
    }

    /// Append a single alternate name.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder()
    ///     .add_alternate_name("La Tour Eiffel")
    ///     .add_alternate_name("Tour Eiffel")
    ///     .build();
    /// assert_eq!(t.alternate_names.len(), 2);
    /// ```
    #[must_use]
    pub fn add_alternate_name<S: Into<String>>(mut self, value: S) -> Self {
        self.alternate_names.push(value.into());
        self
    }

    /// Set the free-form description.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder().description("A landmark in Paris.").build();
    /// assert_eq!(t.description.as_deref(), Some("A landmark in Paris."));
    /// ```
    #[must_use]
    pub fn description<S: Into<String>>(mut self, value: S) -> Self {
        self.description = Some(value.into());
        self
    }

    /// Set the disambiguating description.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder().disambiguating_description("The Paris one.").build();
    /// assert_eq!(t.disambiguating_description.as_deref(), Some("The Paris one."));
    /// ```
    #[must_use]
    pub fn disambiguating_description<S: Into<String>>(mut self, value: S) -> Self {
        self.disambiguating_description = Some(value.into());
        self
    }

    /// Replace the entire list of identifiers.
    ///
    /// ```
    /// # use thing_matcher::{Thing, Identifier};
    /// let id = Identifier::new("wikidata", "Q243").unwrap();
    /// let t = Thing::builder().identifiers(vec![id.clone()]).build();
    /// assert_eq!(t.identifiers, vec![id]);
    /// ```
    #[must_use]
    pub fn identifiers(mut self, value: Vec<Identifier>) -> Self {
        self.identifiers = value;
        self
    }

    /// Append a single identifier.
    ///
    /// ```
    /// # use thing_matcher::{Thing, Identifier};
    /// let t = Thing::builder()
    ///     .add_identifier(Identifier::new("wikidata", "Q243").unwrap())
    ///     .build();
    /// assert_eq!(t.identifiers.len(), 1);
    /// ```
    #[must_use]
    pub fn add_identifier(mut self, value: Identifier) -> Self {
        self.identifiers.push(value);
        self
    }

    /// Set the canonical URL.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder().url("https://www.toureiffel.paris/").build();
    /// assert_eq!(t.url.as_deref(), Some("https://www.toureiffel.paris/"));
    /// ```
    #[must_use]
    pub fn url<S: Into<String>>(mut self, value: S) -> Self {
        self.url = Some(value.into());
        self
    }

    /// Set the representative image URL.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder().image("https://example.org/eiffel.jpg").build();
    /// assert_eq!(t.image.as_deref(), Some("https://example.org/eiffel.jpg"));
    /// ```
    #[must_use]
    pub fn image<S: Into<String>>(mut self, value: S) -> Self {
        self.image = Some(value.into());
        self
    }

    /// Replace the entire `sameAs` list.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder()
    ///     .same_as(vec![
    ///         "https://www.wikidata.org/wiki/Q243".into(),
    ///         "https://en.wikipedia.org/wiki/Eiffel_Tower".into(),
    ///     ])
    ///     .build();
    /// assert_eq!(t.same_as.len(), 2);
    /// ```
    #[must_use]
    pub fn same_as(mut self, value: Vec<String>) -> Self {
        self.same_as = value;
        self
    }

    /// Append a single `sameAs` URL.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder()
    ///     .add_same_as("https://www.wikidata.org/wiki/Q243")
    ///     .build();
    /// assert_eq!(t.same_as.len(), 1);
    /// ```
    #[must_use]
    pub fn add_same_as<S: Into<String>>(mut self, value: S) -> Self {
        self.same_as.push(value.into());
        self
    }

    /// Set the `mainEntityOfPage` URL.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder()
    ///     .main_entity_of_page("https://en.wikipedia.org/wiki/Eiffel_Tower")
    ///     .build();
    /// assert_eq!(
    ///     t.main_entity_of_page.as_deref(),
    ///     Some("https://en.wikipedia.org/wiki/Eiffel_Tower"),
    /// );
    /// ```
    #[must_use]
    pub fn main_entity_of_page<S: Into<String>>(mut self, value: S) -> Self {
        self.main_entity_of_page = Some(value.into());
        self
    }

    /// Replace the entire list of `additionalType` URIs.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder()
    ///     .additional_types(vec!["https://schema.org/Landmark".into()])
    ///     .build();
    /// assert_eq!(t.additional_types.len(), 1);
    /// ```
    #[must_use]
    pub fn additional_types(mut self, value: Vec<String>) -> Self {
        self.additional_types = value;
        self
    }

    /// Append a single `additionalType` URI.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder()
    ///     .add_additional_type("https://schema.org/Landmark")
    ///     .build();
    /// assert_eq!(t.additional_types.len(), 1);
    /// ```
    #[must_use]
    pub fn add_additional_type<S: Into<String>>(mut self, value: S) -> Self {
        self.additional_types.push(value.into());
        self
    }

    /// Replace the entire `subjectOf` list.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder()
    ///     .subject_of(vec!["https://en.wikipedia.org/wiki/Eiffel_Tower".into()])
    ///     .build();
    /// assert_eq!(t.subject_of.len(), 1);
    /// ```
    #[must_use]
    pub fn subject_of(mut self, value: Vec<String>) -> Self {
        self.subject_of = value;
        self
    }

    /// Append a single `subjectOf` URL.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder()
    ///     .add_subject_of("https://en.wikipedia.org/wiki/Eiffel_Tower")
    ///     .build();
    /// assert_eq!(t.subject_of.len(), 1);
    /// ```
    #[must_use]
    pub fn add_subject_of<S: Into<String>>(mut self, value: S) -> Self {
        self.subject_of.push(value.into());
        self
    }

    /// Set the owner (person or organisation, as a string).
    ///
    /// The crate does not model `Person` / `Organization` separately; the
    /// owner is stored as an opaque name or URL string.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder().owner("City of Paris").build();
    /// assert_eq!(t.owner.as_deref(), Some("City of Paris"));
    /// ```
    #[must_use]
    pub fn owner<S: Into<String>>(mut self, value: S) -> Self {
        self.owner = Some(value.into());
        self
    }

    /// Set the local identifier.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder().local_id("REF-12345").build();
    /// assert_eq!(t.local_id.as_deref(), Some("REF-12345"));
    /// ```
    #[must_use]
    pub fn local_id<S: Into<String>>(mut self, value: S) -> Self {
        self.local_id = Some(value.into());
        self
    }

    /// Append a single relationship reference. Chainable; call multiple
    /// times to record several relationships.
    ///
    /// ```
    /// # use thing_matcher::{RelationKind, RelationshipRef, Thing};
    /// let t = Thing::builder()
    ///     .add_relationship(RelationshipRef::new(RelationKind::Contains, "thing-42").unwrap())
    ///     .build();
    /// assert_eq!(t.relationships.len(), 1);
    /// ```
    #[must_use]
    pub fn add_relationship(mut self, relationship: RelationshipRef) -> Self {
        self.relationships.push(relationship);
        self
    }

    /// Replace the entire relationship list.
    ///
    /// ```
    /// # use thing_matcher::{RelationKind, RelationshipRef, Thing};
    /// let refs = vec![RelationshipRef::new(RelationKind::SubPart, "thing-1").unwrap()];
    /// let t = Thing::builder().relationships(refs).build();
    /// assert_eq!(t.relationships.len(), 1);
    /// ```
    #[must_use]
    pub fn relationships(mut self, value: Vec<RelationshipRef>) -> Self {
        self.relationships = value;
        self
    }

    /// Append a single tag. Chainable; call multiple times to record
    /// several tags.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder().add_tag("landmark").add_tag("unesco").build();
    /// assert_eq!(t.tags, vec!["landmark".to_string(), "unesco".to_string()]);
    /// ```
    #[must_use]
    pub fn add_tag<S: Into<String>>(mut self, value: S) -> Self {
        self.tags.push(value.into());
        self
    }

    /// Replace the entire tag list.
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder().tags(vec!["landmark".to_string()]).build();
    /// assert_eq!(t.tags.len(), 1);
    /// ```
    #[must_use]
    pub fn tags(mut self, value: Vec<String>) -> Self {
        self.tags = value;
        self
    }

    /// Consume the builder and produce the [`Thing`].
    ///
    /// ```
    /// # use thing_matcher::Thing;
    /// let t = Thing::builder().name("Big Ben").build();
    /// assert!(t.url.is_none());
    /// ```
    #[must_use]
    pub fn build(self) -> Thing {
        Thing {
            name: self.name,
            alternate_names: self.alternate_names,
            description: self.description,
            disambiguating_description: self.disambiguating_description,
            identifiers: self.identifiers,
            url: self.url,
            image: self.image,
            same_as: self.same_as,
            main_entity_of_page: self.main_entity_of_page,
            additional_types: self.additional_types,
            subject_of: self.subject_of,
            owner: self.owner,
            local_id: self.local_id,
            relationships: self.relationships,
            tags: self.tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins that a freshly built thing carries no data: every optional
    /// field is `None` and every collection field is empty.
    #[test]
    fn thing_builder_starts_empty() {
        let t = Thing::builder().build();
        assert!(t.name.is_none());
        assert!(t.alternate_names.is_empty());
        assert!(t.description.is_none());
        assert!(t.disambiguating_description.is_none());
        assert!(t.identifiers.is_empty());
        assert!(t.url.is_none());
        assert!(t.image.is_none());
        assert!(t.same_as.is_empty());
        assert!(t.main_entity_of_page.is_none());
        assert!(t.additional_types.is_empty());
        assert!(t.subject_of.is_empty());
        assert!(t.owner.is_none());
        assert!(t.local_id.is_none());
        assert!(t.relationships.is_empty());
        assert!(t.tags.is_empty());
    }

    /// Pins the `impl Into<String>` ergonomics: setters accept both `&str`
    /// and owned `String` without explicit conversion at the call site.
    #[test]
    fn thing_builder_accepts_str_and_string() {
        let t = Thing::builder()
            .name("Eiffel Tower")
            .add_alternate_name(String::from("La Tour Eiffel"))
            .build();
        assert_eq!(t.name.as_deref(), Some("Eiffel Tower"));
        assert_eq!(t.alternate_names, vec!["La Tour Eiffel".to_string()]);
    }

    /// Pins the single validation rule: `validate()` succeeds when a name
    /// is present and returns `MissingField` when it is absent.
    #[test]
    fn thing_validate_requires_a_name() {
        assert!(
            Thing::builder()
                .name("Eiffel Tower")
                .build()
                .validate()
                .is_ok()
        );
        let err = Thing::builder()
            .build()
            .validate()
            .expect_err("should be missing");
        assert!(matches!(err, crate::MatchingError::MissingField(_)));
    }

    /// Pins that a fully-populated `Thing` survives a serde JSON round-trip
    /// unchanged — the data model is a faithful serialisation contract.
    #[test]
    fn thing_round_trips_through_serde() {
        let t = Thing::builder()
            .name("Eiffel Tower")
            .add_alternate_name("La Tour Eiffel")
            .description("Iron tower in Paris.")
            .url("https://www.toureiffel.paris/")
            .add_identifier(Identifier::new("wikidata", "Q243").unwrap())
            .add_same_as("https://www.wikidata.org/wiki/Q243")
            .add_additional_type("https://schema.org/Landmark")
            .build();
        let json = serde_json::to_string(&t).expect("serialise");
        let back: Thing = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(t, back);
    }

    /// Pins that the plural `alternate_names` setter *replaces* the whole
    /// vec (as opposed to `add_alternate_name`, which appends).
    #[test]
    fn alternate_names_setter_replaces_vec() {
        let t = Thing::builder()
            .alternate_names(vec!["X".into(), "Y".into()])
            .build();
        assert_eq!(t.alternate_names, vec!["X".to_string(), "Y".to_string()]);
    }

    /// Pins the only normalisation `Identifier::new` performs: surrounding
    /// whitespace is trimmed off both the property id and the value.
    #[test]
    fn identifier_trims_value_and_property_id() {
        let id = Identifier::new("  wikidata  ", "   Q243 ").unwrap();
        assert_eq!(id.property_id, "wikidata");
        assert_eq!(id.value, "Q243");
    }

    /// Pins the rejection rule: a component that is empty or
    /// whitespace-only (after trimming) makes `Identifier::new` return
    /// `None` — there is no such thing as an empty identifier.
    #[test]
    fn identifier_rejects_empty_components() {
        assert!(Identifier::new("wikidata", "").is_none());
        assert!(Identifier::new("wikidata", "    ").is_none());
        assert!(Identifier::new("", "Q243").is_none());
        assert!(Identifier::new("   ", "Q243").is_none());
    }

    /// Pins that identifier equality is scheme-scoped: the same value under
    /// different `property_id`s is NOT equal, preventing cross-scheme
    /// collisions (e.g. an `isbn` value matching a `wikidata` value).
    #[test]
    fn identifier_equality_is_property_scoped() {
        let g = Identifier::new("google", "X").unwrap();
        let w = Identifier::new("wikidata", "X").unwrap();
        assert_ne!(g, w);
    }

    /// Pins that an `Identifier` survives a serde JSON round-trip unchanged.
    #[test]
    fn identifier_round_trips_through_serde() {
        let id = Identifier::new("custom", "abc-123").unwrap();
        let json = serde_json::to_string(&id).expect("serialise");
        let back: Identifier = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(id, back);
    }

    // ---------- relationships & tags (T-PRO-H7) ----------

    /// Pins the trim + empty-rejection behaviour of `RelationshipRef::new`.
    #[test]
    fn relationship_ref_trims_and_rejects_empty_thing_id() {
        let r = RelationshipRef::new(RelationKind::Contains, "  thing-42  ").unwrap();
        assert_eq!(r.thing_id, "thing-42");
        assert_eq!(r.relation, RelationKind::Contains);
        assert!(RelationshipRef::new(RelationKind::ContainedIn, "   ").is_none());
        assert!(RelationshipRef::new(RelationKind::ContainedIn, "").is_none());
    }

    /// Pins that `RelationKind` variants are distinct.
    #[test]
    fn relation_kind_variants_are_distinct() {
        assert_ne!(RelationKind::Contains, RelationKind::ContainedIn);
        assert_ne!(RelationKind::SuperPart, RelationKind::SubPart);
        assert_eq!(RelationKind::Contains, RelationKind::Contains);
    }

    /// Pins that a freshly built thing carries no relationships or tags.
    #[test]
    fn thing_builder_relationships_and_tags_start_empty() {
        let t = Thing::builder().build();
        assert!(t.relationships.is_empty());
        assert!(t.tags.is_empty());
    }

    /// Pins the `add_relationship` / `add_tag` chainable-append setters.
    #[test]
    fn thing_builder_carries_relationships_and_tags() {
        let t = Thing::builder()
            .add_relationship(RelationshipRef::new(RelationKind::Contains, "thing-1").unwrap())
            .add_relationship(RelationshipRef::new(RelationKind::SubPart, "thing-2").unwrap())
            .add_tag("landmark")
            .add_tag("unesco")
            .build();
        assert_eq!(t.relationships.len(), 2);
        assert_eq!(t.tags, vec!["landmark".to_string(), "unesco".to_string()]);
    }

    /// Pins that the plural `relationships` / `tags` setters *replace* the
    /// whole vec (as opposed to `add_relationship` / `add_tag`, which
    /// append).
    #[test]
    fn relationships_and_tags_setters_replace_vec() {
        let refs = vec![RelationshipRef::new(RelationKind::Contains, "thing-9").unwrap()];
        let tags = vec!["days".to_string()];
        let t = Thing::builder()
            .relationships(refs.clone())
            .tags(tags.clone())
            .build();
        assert_eq!(t.relationships, refs);
        assert_eq!(t.tags, tags);
    }
}
