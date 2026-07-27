//! SOUP register (IEC 62304 §5.3.3, §8.1.2) and `CycloneDX` SBOM.
//!
//! IEC 62304 calls every third-party component **SOUP** — *Software Of
//! Unknown Provenance* — and requires each one to be identified, versioned,
//! and justified.
//!
//! ## Why a person registry keeps one
//!
//! This service holds personal data and serves a FHIR `Patient`
//! representation, but it is **not** a medical device: no output drives
//! an individual's treatment (the qualification caveat in
//! `agents/share/compliance-for-healthcare.md` §2.4). It therefore
//! carries no IEC 62304 safety classification, and `GET /api/compliance`
//! says so rather than leaving it inferred from silence.
//!
//! The register is kept for reasons that stand without the device
//! framing: ISO/IEC 27001 A.8 wants supply-chain and
//! configuration-management evidence in exactly this shape, and an
//! inventory that states *why* each dependency is present is the
//! artefact that makes a vulnerability advisory actionable instead of a
//! research project.
//!
//! "Safety relevance" in the register therefore means relevance to the
//! correctness and confidentiality of personal data, not to physical
//! safety.
//!
//! ## No drift, by construction
//!
//! The component list is **not** a hand-maintained inventory that can
//! silently fall behind the code. It is derived at compile time from the
//! crate's own `Cargo.lock` via [`include_str!`], so it is exactly the
//! dependency graph the binary was built from — if the lockfile changes,
//! the SBOM changes with it and the crate rebuilds.
//!
//! What *is* hand-maintained is the **annotation**: `compliance/soup.tsv`
//! records, for each direct dependency, what it is used for and whether it
//! is safety- or security-relevant — the §8.1.2 justification a lockfile
//! cannot supply. The unit tests below pin the two together, so an added
//! dependency fails the build until it is annotated.
//!
//! ## Determinism
//!
//! The rendered SBOM contains no timestamp and no random serial number, so
//! two builds of the same source produce byte-identical output.
//! `CycloneDX` makes both fields optional precisely for this case. (The
//! reproducible-build wrapper that consumes this property lives in
//! care-pathway's `scripts/build-reproducible.sh` and has not been
//! ported to this crate yet; the SBOM is deterministic regardless.)

use serde::Serialize;

/// The crate's lockfile, embedded at compile time — the authoritative
/// dependency graph for this binary.
const CARGO_LOCK: &str = include_str!("../../Cargo.lock");

/// The crate manifest, embedded so the register can be checked against the
/// crate's *declared direct* dependencies.
const CARGO_TOML: &str = include_str!("../../Cargo.toml");

/// Hand-maintained SOUP annotations: `name<TAB>purpose<TAB>safety_relevance`.
const SOUP_REGISTER: &str = include_str!("../../compliance/soup.tsv");

/// One SBOM component: identity from the lockfile, justification from the
/// register.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Component {
    /// Package name.
    pub name: String,
    /// Resolved version.
    pub version: String,
    /// What this crate uses it for (§8.1.2), when annotated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    /// Why it is (or is not) safety- or security-relevant, when annotated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_relevance: Option<String>,
    /// `true` when the crate depends on this directly (and so must
    /// annotate it); `false` for a transitive dependency.
    pub direct: bool,
}

/// One annotation row from `compliance/soup.tsv`.
struct Annotation<'a> {
    name: &'a str,
    purpose: &'a str,
    safety_relevance: &'a str,
}

/// Parse the SOUP register, skipping blank lines and `#` comments.
fn annotations() -> Vec<Annotation<'static>> {
    SOUP_REGISTER
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .filter_map(|line| {
            let mut parts = line.split('\t');
            Some(Annotation {
                name: parts.next()?.trim(),
                purpose: parts.next()?.trim(),
                safety_relevance: parts.next()?.trim(),
            })
        })
        .collect()
}

/// Every package in `Cargo.lock`, as `(name, version)` in lockfile order.
fn locked_packages() -> Vec<(String, String)> {
    let mut packages = Vec::new();
    let mut name: Option<String> = None;
    let mut in_package = false;
    for line in CARGO_LOCK.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            in_package = true;
            name = None;
        } else if trimmed.starts_with('[') && trimmed != "[[package]]" {
            in_package = false;
        } else if in_package {
            if let Some(value) = field(trimmed, "name") {
                name = Some(value.to_string());
            } else if let Some(value) = field(trimmed, "version")
                && let Some(n) = name.take()
            {
                packages.push((n, value.to_string()));
            }
        }
    }
    packages
}

/// The crate's **declared direct** dependencies, from the `[dependencies]`
/// and `[dev-dependencies]` tables of `Cargo.toml`.
fn declared_dependencies() -> Vec<String> {
    let mut names = Vec::new();
    let mut in_deps = false;
    for line in CARGO_TOML.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = matches!(trimmed, "[dependencies]" | "[dev-dependencies]");
            continue;
        }
        if !in_deps || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = trimmed.split_once('=') {
            let key = key.trim().trim_matches('"');
            if !key.is_empty() && !names.iter().any(|n| n == key) {
                names.push(key.to_string());
            }
        }
    }
    names
}

/// Extract `key = "value"` from a lockfile line.
fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line
        .strip_prefix(key)?
        .trim_start()
        .strip_prefix('=')?
        .trim();
    rest.strip_prefix('"')?.strip_suffix('"')
}

/// The full component list: every locked package, annotated where the
/// register covers it and flagged `direct` where the manifest declares it.
#[must_use]
pub fn components() -> Vec<Component> {
    let annotations = annotations();
    let direct = declared_dependencies();
    locked_packages()
        .into_iter()
        .filter(|(name, _)| name != env!("CARGO_PKG_NAME"))
        .map(|(name, version)| {
            let annotation = annotations.iter().find(|a| a.name == name);
            Component {
                purpose: annotation.map(|a| a.purpose.to_string()),
                safety_relevance: annotation.map(|a| a.safety_relevance.to_string()),
                direct: direct.contains(&name),
                name,
                version,
            }
        })
        .collect()
}

/// Direct dependencies missing an entry in `compliance/soup.tsv`.
///
/// IEC 62304 §8.1.2 wants a justification for every SOUP item the product
/// depends on. Transitive packages are listed in the SBOM but are not
/// individually justified — stating that boundary is more honest than
/// pretending to have reviewed 400 crates.
#[must_use]
pub fn unannotated_direct_dependencies() -> Vec<String> {
    let annotations = annotations();
    declared_dependencies()
        .into_iter()
        .filter(|name| !annotations.iter().any(|a| a.name == *name))
        .collect()
}

/// Register entries naming a package the lockfile does not contain — a
/// stale annotation left behind by a removed dependency.
#[must_use]
pub fn stale_register_entries() -> Vec<String> {
    let packages = locked_packages();
    annotations()
        .into_iter()
        .map(|a| a.name.to_string())
        .filter(|name| !packages.iter().any(|(p, _)| p == name))
        .collect()
}

/// A `CycloneDX` 1.5 bill of materials.
#[derive(Debug, Clone, Serialize)]
pub struct Sbom {
    /// Always `"CycloneDX"`.
    #[serde(rename = "bomFormat")]
    pub bom_format: &'static str,
    /// `CycloneDX` specification version.
    #[serde(rename = "specVersion")]
    pub spec_version: &'static str,
    /// BOM revision.
    pub version: u32,
    /// Subject-of-the-BOM metadata.
    pub metadata: SbomMetadata,
    /// One entry per dependency.
    pub components: Vec<SbomComponent>,
}

/// `CycloneDX` `metadata` — deliberately timestamp-free (see the module docs).
#[derive(Debug, Clone, Serialize)]
pub struct SbomMetadata {
    /// The component this BOM describes.
    pub component: SbomComponent,
    /// IEC 62304 properties carried alongside the standard fields.
    pub properties: Vec<SbomProperty>,
}

/// A `CycloneDX` `component`.
#[derive(Debug, Clone, Serialize)]
pub struct SbomComponent {
    /// `CycloneDX` component type — `application` for this crate, `library`
    /// for its dependencies.
    #[serde(rename = "type")]
    pub component_type: &'static str,
    /// Package name.
    pub name: String,
    /// Resolved version.
    pub version: String,
    /// Package URL (`pkg:cargo/<name>@<version>`).
    pub purl: String,
    /// Free-text description — the SOUP purpose, where annotated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// IEC 62304 properties (SOUP classification).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<SbomProperty>,
}

/// A `CycloneDX` `property` — a namespaced name/value pair.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SbomProperty {
    /// Property name, namespaced under `mxi:`.
    pub name: String,
    /// Property value.
    pub value: String,
}

impl SbomProperty {
    /// Build a namespaced property.
    fn new(name: &str, value: impl Into<String>) -> Self {
        Self {
            name: format!("mxi:{name}"),
            value: value.into(),
        }
    }
}

/// Render the `CycloneDX` SBOM for this binary.
#[must_use]
pub fn sbom() -> Sbom {
    let build = super::Build::current();
    let components = components()
        .into_iter()
        .map(|c| SbomComponent {
            component_type: "library",
            purl: format!("pkg:cargo/{}@{}", c.name, c.version),
            description: c.purpose.clone(),
            properties: vec![
                SbomProperty::new("soup", if c.direct { "direct" } else { "transitive" }),
                SbomProperty::new(
                    "safety-relevance",
                    c.safety_relevance
                        .clone()
                        .unwrap_or_else(|| "not individually assessed (transitive)".to_string()),
                ),
            ],
            name: c.name,
            version: c.version,
        })
        .collect();
    Sbom {
        bom_format: "CycloneDX",
        spec_version: "1.5",
        version: 1,
        metadata: SbomMetadata {
            component: SbomComponent {
                component_type: "application",
                name: env!("CARGO_PKG_NAME").to_string(),
                version: build.version.to_string(),
                purl: format!("pkg:cargo/{}@{}", env!("CARGO_PKG_NAME"), build.version),
                description: Some("Person identity registry service".to_string()),
                properties: Vec::new(),
            },
            properties: vec![
                // No `iec62304-safety-class` property: serving FHIR
                // Patient does not make this a medical device, and
                // claiming a classification would overstate the case.
                SbomProperty::new("build-commit", build.commit),
                SbomProperty::new(
                    "source-date-epoch",
                    build.source_date_epoch.unwrap_or("unset"),
                ),
                SbomProperty::new(
                    "reproducible-release",
                    build.is_reproducible_release().to_string(),
                ),
            ],
        },
        components,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The lockfile parser finds a realistic number of packages, each with
    /// a non-empty name and version. A parser that silently returned an
    /// empty list would make the SBOM vacuously "complete".
    #[test]
    fn lockfile_parses_into_packages() {
        let packages = locked_packages();
        assert!(
            packages.len() > 50,
            "expected a realistic dependency graph, got {}",
            packages.len()
        );
        for (name, version) in &packages {
            assert!(!name.is_empty(), "package with an empty name");
            assert!(!version.is_empty(), "{name} has no version");
            assert!(!name.contains('"'), "{name} kept its quotes");
        }
    }

    /// Known direct dependencies really are in the parsed lockfile — a
    /// spot check that the parser is reading the right file.
    #[test]
    fn lockfile_contains_known_dependencies() {
        let packages = locked_packages();
        for expected in [
            "loco-rs",
            "sea-orm",
            "axum",
            "sha2",
            "sha3",
            "hmac",
            "person-matcher",
            "tantivy",
        ] {
            assert!(
                packages.iter().any(|(n, _)| n == expected),
                "{expected} missing from the parsed lockfile"
            );
        }
    }

    /// The manifest parser finds the crate's declared direct dependencies
    /// and does not leak table headers or the `[workspace]` block.
    #[test]
    fn manifest_parses_direct_dependencies() {
        let direct = declared_dependencies();
        for expected in [
            "loco-rs",
            "serde",
            "sha2",
            "sha3",
            "hmac",
            "person-matcher",
            "entity-ref",
            "tantivy",
        ] {
            assert!(direct.contains(&expected.to_string()), "{expected} missing");
        }
        assert!(!direct.iter().any(|d| d.starts_with('[')), "{direct:?}");
        assert!(
            !direct.contains(&"name".to_string()),
            "leaked [package] key"
        );
    }

    /// **IEC 62304 §8.1.2 gate.** Every direct dependency must carry a
    /// SOUP annotation. Adding a dependency without annotating it fails
    /// here — which is the point: the register cannot drift.
    #[test]
    fn every_direct_dependency_is_annotated() {
        let missing = unannotated_direct_dependencies();
        assert!(
            missing.is_empty(),
            "add these to compliance/soup.tsv (name<TAB>purpose<TAB>safety relevance): {missing:?}"
        );
    }

    /// The register must not name packages that are no longer depended on.
    #[test]
    fn register_has_no_stale_entries() {
        let stale = stale_register_entries();
        assert!(
            stale.is_empty(),
            "remove these from compliance/soup.tsv: {stale:?}"
        );
    }

    /// Every annotation is substantive — a register full of empty cells
    /// satisfies the letter of §8.1.2 and none of its purpose.
    #[test]
    fn annotations_are_substantive() {
        let rows = annotations();
        assert!(!rows.is_empty(), "the SOUP register is empty");
        for row in rows {
            assert!(!row.purpose.is_empty(), "{}: no purpose", row.name);
            assert!(
                row.safety_relevance.len() > 3,
                "{}: safety relevance must say something",
                row.name
            );
        }
    }

    /// The SBOM is well-formed `CycloneDX` and covers the whole graph.
    #[test]
    fn sbom_is_well_formed_cyclonedx() {
        let bom = sbom();
        assert_eq!(bom.bom_format, "CycloneDX");
        assert_eq!(bom.spec_version, "1.5");
        assert_eq!(bom.components.len(), components().len());
        assert!(bom.metadata.component.purl.starts_with("pkg:cargo/"));
        for component in &bom.components {
            assert!(
                component.purl.starts_with("pkg:cargo/"),
                "{} has a malformed purl",
                component.name
            );
            assert!(component.properties.iter().any(|p| p.name == "mxi:soup"));
        }
    }

    /// The SBOM carries build provenance — the fields that make it
    /// evidence rather than an inventory.
    #[test]
    fn sbom_metadata_carries_lifecycle_evidence() {
        let bom = sbom();
        let names: Vec<&str> = bom
            .metadata
            .properties
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        for expected in [
            "mxi:build-commit",
            "mxi:source-date-epoch",
            "mxi:reproducible-release",
        ] {
            assert!(names.contains(&expected), "{expected} missing");
        }
        // Deliberately absent: a person registry does not drive anyone's
        // treatment, so a medical-device safety class would be an
        // unsupportable claim even though the crate serves FHIR Patient.
        assert!(
            !names.contains(&"mxi:iec62304-safety-class"),
            "a person registry must not declare a device safety class"
        );
    }

    /// Rendering is deterministic: no timestamp, no random serial number,
    /// so a reproducible build produces a byte-identical SBOM.
    #[test]
    fn sbom_rendering_is_deterministic() {
        let first = serde_json::to_string(&sbom()).expect("serialize");
        let second = serde_json::to_string(&sbom()).expect("serialize");
        assert_eq!(first, second);
        // Match the JSON *keys*, not the word anywhere in the document:
        // `timestamp` and `serialNumber` are the two optional CycloneDX
        // fields that would vary between builds. A bare substring test
        // also fires on any dependency annotation that happens to use the
        // word — which it did, the first time a register described a
        // crate as providing "timestamps".
        assert!(
            !first.contains("\"timestamp\":"),
            "a CycloneDX timestamp field would break reproducibility"
        );
        assert!(
            !first.contains("\"serialNumber\":"),
            "a random serial number would break reproducibility"
        );
    }

    /// The `key = "value"` lockfile-field parser is exact.
    #[test]
    fn field_parser_is_exact() {
        assert_eq!(field(r#"name = "serde""#, "name"), Some("serde"));
        assert_eq!(field(r#"version = "1.0.1""#, "version"), Some("1.0.1"));
        assert_eq!(field(r#"name = "serde""#, "version"), None);
        assert_eq!(field(r#"checksum = "abc""#, "name"), None);
        assert_eq!(field(" \"itoa\",", "name"), None);
    }
}
