//! Domain model — a slim, library-friendly subset of
//! `schema.org/Organization`. The matcher only models the properties
//! that carry identity signal.

use serde::{Deserialize, Serialize};

/// Pairwise input to the matcher.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Organization {
    /// schema.org/name — required common name.
    pub name: String,
    /// schema.org/legalName — registered legal name (e.g. "Acme, Inc.").
    /// Also tried when scoring `name`.
    #[serde(default)]
    pub legal_name: Option<String>,
    /// schema.org/alternateName — aliases / trading names.
    #[serde(default)]
    pub alternate_names: Vec<String>,
    /// External identifiers — LEI, DUNS, VAT, tax ID, Wikidata, etc.
    #[serde(default)]
    pub identifiers: Vec<OrgIdentifier>,
    /// schema.org/url — primary website.
    #[serde(default)]
    pub url: Option<String>,
    /// schema.org/sameAs — cross-system identity URLs (Wikidata, ROR
    /// page, official register entry). Used by the deterministic
    /// short-circuit.
    #[serde(default)]
    pub same_as: Vec<String>,
    /// schema.org/address — postal address.
    #[serde(default)]
    pub address: Option<PostalAddress>,
    /// ISO 3166 country (or region) scoping jurisdiction-bound
    /// identifiers like `TaxId`. "US", "GB", "DE", …
    #[serde(default)]
    pub jurisdiction: Option<String>,
    /// schema.org/foundingDate — ISO-8601 date string (`YYYY` or
    /// `YYYY-MM-DD`). Only the year is compared.
    #[serde(default)]
    pub founding_date: Option<String>,
    /// schema.org/telephone.
    #[serde(default)]
    pub telephone: Option<String>,
    /// schema.org/email.
    #[serde(default)]
    pub email: Option<String>,
    /// schema.org/keywords — descriptive tags.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Typed references to other organizations (by opaque id) in the
    /// consuming registry — e.g. "this organization's parent is
    /// organization X". A **supporting** signal only: never identifying
    /// on its own, and the matcher never resolves the reference (it has
    /// no registry) — it only compares the two organizations'
    /// relationship **sets**. Default empty. See [`RelationshipRef`] /
    /// [`RelationKind`]; spec §14a / §23.
    #[serde(default)]
    pub relationships: Vec<RelationshipRef>,
    /// Operator-applied free-text labels (e.g. `"vendor"`, `"tier-1"`),
    /// distinct from the descriptive `keywords`. Stored verbatim;
    /// compared case-insensitively at match time. A **supporting**
    /// signal only: never identifying on its own. Default empty. See
    /// spec §14b / §23.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Organization {
    /// Construct an `Organization` with just the required name; every
    /// other field defaults to empty / `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use organization_matcher::Organization;
    ///
    /// let org = Organization::new("Acme Corporation");
    /// assert_eq!(org.name, "Acme Corporation");
    /// assert!(org.identifiers.is_empty());
    /// ```
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

/// An external identifier: a scheme plus its value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgIdentifier {
    /// The scheme under which `value` is published.
    pub scheme: IdentifierScheme,
    /// The identifier value within `scheme`.
    pub value: String,
}

/// The scheme under which an identifier's `value` is published.
///
/// Schemes marked **deterministic** are globally unique by construction
/// — a match on these pins the final score to `1.0` via the R-0
/// short-circuit. **Jurisdiction-scoped** schemes (`TaxId`) only make
/// sense within a country/register and short-circuit via R-1.
/// **Classification** schemes (`Naics` / `IsicV4` / `Sic`) describe the
/// *sector*, not the entity, and never short-circuit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentifierScheme {
    /// Legal Entity Identifier (ISO 17442). **Deterministic.**
    /// Example: `5493001KJTIIGC8Y1R12`.
    Lei,
    /// Dun & Bradstreet DUNS number. **Deterministic.**
    /// Example: `150483782`.
    Duns,
    /// ISO 6523 organization identifier. **Deterministic.**
    Iso6523,
    /// GS1 Global Location Number. **Deterministic.**
    Gln,
    /// Wikidata entity id. **Deterministic.** Example: `Q312`.
    Wikidata,
    /// Research Organization Registry id. **Deterministic.**
    /// Example: `02nr0ka47`.
    Ror,
    /// ISNI (ISO 27729). **Deterministic.**
    Isni,
    /// VAT identification number *with national prefix* (e.g.
    /// `DE811569869`). The prefix makes it globally unique →
    /// **Deterministic.**
    Vat,
    /// National tax / fiscal id. **Jurisdiction-scoped** — only
    /// short-circuits when both records share `jurisdiction`.
    TaxId,
    /// NAICS industry code. **Classification** — never identity.
    Naics,
    /// ISIC Rev.4 industry code. **Classification** — never identity.
    IsicV4,
    /// SIC industry code. **Classification** — never identity.
    Sic,
    /// Free-form custom scheme with a caller-supplied label.
    /// Non-deterministic.
    Custom(String),
}

impl IdentifierScheme {
    /// Schemes whose values are globally unique by construction. A
    /// match on these pins the final score to `1.0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use organization_matcher::IdentifierScheme;
    ///
    /// assert!(IdentifierScheme::Lei.is_deterministic());
    /// assert!(!IdentifierScheme::TaxId.is_deterministic());
    /// assert!(!IdentifierScheme::Naics.is_deterministic());
    /// ```
    #[must_use]
    pub fn is_deterministic(&self) -> bool {
        matches!(
            self,
            IdentifierScheme::Lei
                | IdentifierScheme::Duns
                | IdentifierScheme::Iso6523
                | IdentifierScheme::Gln
                | IdentifierScheme::Wikidata
                | IdentifierScheme::Ror
                | IdentifierScheme::Isni
                | IdentifierScheme::Vat
        )
    }
}

/// A typed reference from one [`Organization`] to another, by opaque id
/// in the consuming registry (e.g. "this organization's parent is
/// organization `X`").
///
/// `RelationshipRef` is a **supporting** matching signal, not an
/// identifying one: the matcher never resolves `organization_id`
/// against a registry (it has none) — it only compares the two
/// organizations' relationship **sets** via typed-set Jaccard over
/// `(relation, organization_id)` pairs (spec §14a).
///
/// Construct via [`RelationshipRef::new`], which trims `organization_id`
/// and rejects an empty result, so two records carrying different
/// incidental whitespace around the same id compare equal.
///
/// # Examples
///
/// ```
/// use organization_matcher::{RelationKind, RelationshipRef};
///
/// let r = RelationshipRef::new(RelationKind::SubOrganizationOf, " org-42 ").unwrap();
/// assert_eq!(r.organization_id, "org-42");
/// assert_eq!(r.relation, RelationKind::SubOrganizationOf);
///
/// assert!(RelationshipRef::new(RelationKind::ParentOrganizationOf, "   ").is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationshipRef {
    /// The kind of relationship this side asserts. See [`RelationKind`].
    pub relation: RelationKind,
    /// Opaque id of the related organization in the consuming registry.
    /// Whitespace-trimmed and non-empty — see [`RelationshipRef::new`].
    pub organization_id: String,
}

impl RelationshipRef {
    /// Construct a relationship reference, trimming `organization_id`
    /// and rejecting an empty result.
    ///
    /// Returns `None` when `organization_id` is empty after trimming.
    ///
    /// ```
    /// use organization_matcher::{RelationKind, RelationshipRef};
    /// let r = RelationshipRef::new(RelationKind::SuccessorOf, "org-7").unwrap();
    /// assert_eq!(r.organization_id, "org-7");
    /// assert!(RelationshipRef::new(RelationKind::SuccessorOf, "").is_none());
    /// ```
    #[must_use]
    pub fn new(relation: RelationKind, organization_id: impl AsRef<str>) -> Option<Self> {
        let organization_id = organization_id.as_ref().trim();
        if organization_id.is_empty() {
            return None;
        }
        Some(Self {
            relation,
            organization_id: organization_id.to_string(),
        })
    }
}

/// The kind of typed relationship one [`Organization`] asserts toward
/// another, carried on a [`RelationshipRef`].
///
/// `SubOrganizationOf` / `ParentOrganizationOf` mirror schema.org's
/// `subOrganization` / `parentOrganization` inverse pair (containment);
/// `SuccessorOf` / `PredecessorOf` cover mergers, renames, and
/// reorganisations — also inverses of each other. The matcher does
/// **not** resolve or cross-check the inverse relationship; it only
/// compares the raw `(relation, organization_id)` pairs each side
/// asserts (spec §14a). `#[non_exhaustive]` so a new variant is purely
/// additive.
///
/// # Examples
///
/// ```
/// use organization_matcher::RelationKind;
///
/// let k = RelationKind::SubOrganizationOf;
/// assert_eq!(k, RelationKind::SubOrganizationOf);
/// assert_ne!(k, RelationKind::ParentOrganizationOf);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RelationKind {
    /// This organization is a sub-organization of the referenced
    /// organization (schema.org `subOrganization`).
    SubOrganizationOf,
    /// This organization is the parent organization of the referenced
    /// organization (schema.org `parentOrganization`).
    ParentOrganizationOf,
    /// This organization is the legal successor of the referenced
    /// organization (merger, rename, reorganisation).
    SuccessorOf,
    /// This organization is the legal predecessor of the referenced
    /// organization (merger, rename, reorganisation).
    PredecessorOf,
}

/// schema.org/PostalAddress — only fields present in *both* records
/// contribute to the address component.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostalAddress {
    /// streetAddress.
    #[serde(default)]
    pub street_address: Option<String>,
    /// addressLocality (city / town).
    #[serde(default)]
    pub locality: Option<String>,
    /// addressRegion (state / province).
    #[serde(default)]
    pub region: Option<String>,
    /// postalCode.
    #[serde(default)]
    pub postal_code: Option<String>,
    /// addressCountry.
    #[serde(default)]
    pub country: Option<String>,
}
