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
