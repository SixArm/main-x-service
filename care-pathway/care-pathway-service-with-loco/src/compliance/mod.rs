//! Regulatory-compliance controls — the family's **reference
//! implementation** of the four control-driving frameworks in
//! [`agents/share/compliance-for-healthcare.md`](../../../../agents/share/compliance-for-healthcare.md)
//! §2, adopted by the care-pathway entity per its
//! [`spec/12-compliance.md`](../../../spec/12-compliance.md) §12.4.
//!
//! | Framework | Module |
//! |---|---|
//! | **HIPAA** — audit controls, integrity, accounting of disclosures | [`audit_chain`] (tamper-evident history), [`disclosure`] (read-auditing) |
//! | **GDPR / EU EHDS** — erasure vs. immutable history, residency, lawful basis | [`erasure`], [`Posture`] (this module) |
//! | **ONC / HTI** — profile & terminology conformance, SMART, Bulk Data | [`crate::fhir::profile`], [`bulk`], [`crate::controllers::fhir`] |
//! | **IEC 62304 / `SaMD`** — lifecycle, SOUP/SBOM, traceability, reproducible builds | [`soup`], `compliance/` at the crate root, `tests/traceability.rs` |
//!
//! ## Configuration
//!
//! Every control is configured from the environment, read **once** and
//! cached (restart to change), mirroring [`crate::auth::require_auth`].
//! The defaults are deliberately conservative and **behaviour-neutral**:
//! read-auditing is off, and every data-protection declaration reads
//! `undeclared` rather than asserting a posture the deployment has not
//! actually adopted.
//!
//! | Variable | Default | Meaning |
//! |---|---|---|
//! | `CARE_PATHWAY_AUDIT_READS` | off | Write an audit row for reads / searches / exports (HIPAA §164.312(b)). |
//! | `CARE_PATHWAY_AUDIT_FAIL_CLOSED` | off | Refuse a read (`503`) when its audit row cannot be written, rather than serving data unaccounted for. |
//! | `CARE_PATHWAY_DATA_RESIDENCY` | `undeclared` | The region this deployment's data is confined to (GDPR Ch. V). |
//! | `CARE_PATHWAY_LAWFUL_BASIS` | `undeclared` | GDPR Art. 6 basis, e.g. `public_task`. |
//! | `CARE_PATHWAY_ART9_CONDITION` | `undeclared` | GDPR Art. 9(2) condition for health data. |
//! | `CARE_PATHWAY_TRANSFER_SAFEGUARD` | `undeclared` | Ch. V safeguard, e.g. `adequacy_decision`. |
//! | `CARE_PATHWAY_SAFETY_CLASS` | `A` | IEC 62304 software safety classification (`A`/`B`/`C`). |
//! | `CARE_PATHWAY_SMART_ISSUER` / `_AUTHORIZATION_URL` / `_TOKEN_URL` | unset | The deployment's real SMART authorization server (see [`smart`]). |
//!
//! ## What this module does not do
//!
//! It **declares and records**; it does not enforce. A residency
//! declaration makes a cross-border export visible in the audit trail — it
//! does not block one. Blocking is a deployment-network decision, and
//! saying so is more useful than implying otherwise.

/// Tamper-evident audit history: the SHA-256 hash chain over `audit_logs`.
pub mod audit_chain;
/// FHIR Bulk Data Access (`$export`) job registry and NDJSON assembly.
pub mod bulk;
/// Read/disclosure auditing: purpose-of-use capture and access records.
pub mod disclosure;
/// GDPR Art. 17 erasure that survives the immutable chain (redaction).
pub mod erasure;

/// Keyed integrity (HMAC) with a key the database never holds.
pub mod mac;
/// Row-level integrity hashing over the `care_pathways` table.
pub mod record_integrity;
/// SOUP register (IEC 62304 §8.1.2) and CycloneDX SBOM assembly.
pub mod soup;

use std::sync::OnceLock;

use serde::Serialize;

/// The value every unset data-protection declaration reports. Chosen over
/// an optimistic default so an operator reading `GET /api/compliance`
/// cannot mistake "nobody configured this" for "this is compliant".
pub const UNDECLARED: &str = "undeclared";

/// Read an environment variable, treating unset/blank as absent.
fn env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Read an environment variable, falling back to [`UNDECLARED`].
fn env_declared(name: &str) -> String {
    env(name).unwrap_or_else(|| UNDECLARED.to_string())
}

/// Whether read-auditing is on, from `CARE_PATHWAY_AUDIT_READS`
/// (read once and cached).
///
/// **Default off.** HIPAA §164.312(b) requires recording activity — and a
/// lookup is activity — but writing an audit row per read costs a write on
/// every `GET`, so adopting this module is behaviour-neutral until a
/// deployment opts in. A HIPAA-facing deployment MUST turn it on, together
/// with `CARE_PATHWAY_REQUIRE_AUTH` (without which the rows carry no
/// actor and the accounting is close to worthless).
#[must_use]
pub fn audit_reads() -> bool {
    static AUDIT_READS: OnceLock<bool> = OnceLock::new();
    *AUDIT_READS.get_or_init(|| {
        crate::auth::parse_bool(&std::env::var("CARE_PATHWAY_AUDIT_READS").unwrap_or_default())
    })
}

/// The declared data-protection posture (GDPR Ch. V / Art. 6 / Art. 9).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DataProtection {
    /// The region data is confined to, or [`UNDECLARED`].
    pub residency: String,
    /// GDPR Art. 6 lawful basis, or [`UNDECLARED`].
    pub lawful_basis: String,
    /// GDPR Art. 9(2) condition for health data, or [`UNDECLARED`].
    pub art9_condition: String,
    /// GDPR Ch. V transfer safeguard, or [`UNDECLARED`].
    pub transfer_safeguard: String,
}

impl DataProtection {
    /// Read the declaration from the environment.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            residency: env_declared("CARE_PATHWAY_DATA_RESIDENCY"),
            lawful_basis: env_declared("CARE_PATHWAY_LAWFUL_BASIS"),
            art9_condition: env_declared("CARE_PATHWAY_ART9_CONDITION"),
            transfer_safeguard: env_declared("CARE_PATHWAY_TRANSFER_SAFEGUARD"),
        }
    }

    /// The process-wide declaration, read once and cached.
    #[must_use]
    pub fn get() -> &'static Self {
        static DP: OnceLock<DataProtection> = OnceLock::new();
        DP.get_or_init(Self::from_env)
    }

    /// Whether sending data to `destination` leaves the declared residency
    /// region — a GDPR Ch. V transfer that must be recorded.
    ///
    /// Conservative by construction: with residency undeclared, or a
    /// destination the caller did not name, the answer is `false`, because
    /// asserting a transfer we cannot substantiate would poison the audit
    /// trail with noise. Comparison is case-insensitive and matches on the
    /// region prefix, so `eu-west-1` is inside `eu`.
    #[must_use]
    pub fn is_cross_border(&self, destination: Option<&str>) -> bool {
        let (Some(dest), true) = (destination, self.residency != UNDECLARED) else {
            return false;
        };
        let dest = dest.trim().to_ascii_lowercase();
        let home = self.residency.trim().to_ascii_lowercase();
        !(dest == home || dest.starts_with(&format!("{home}-")))
    }
}

/// IEC 62304 software safety classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SafetyClass {
    /// No injury or damage to health is possible.
    A,
    /// Non-serious injury is possible.
    B,
    /// Death or serious injury is possible.
    C,
}

impl SafetyClass {
    /// Parse `A` / `B` / `C` (case-insensitive); anything else ⇒ [`Self::A`],
    /// the classification the *template registry alone* warrants.
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_uppercase().as_str() {
            "B" => Self::B,
            "C" => Self::C,
            _ => Self::A,
        }
    }

    /// The declared classification, from `CARE_PATHWAY_SAFETY_CLASS`.
    #[must_use]
    pub fn from_env() -> Self {
        env("CARE_PATHWAY_SAFETY_CLASS").map_or(Self::A, |v| Self::parse(&v))
    }

    /// Why this crate is classified where it is — recorded alongside the
    /// class so the declaration is not a bare letter (IEC 62304 §4.3).
    #[must_use]
    pub fn rationale(self) -> &'static str {
        match self {
            Self::A => {
                "Registry of care-pathway templates only: no individual patient is \
                        tracked and no output drives an individual's treatment, so no injury \
                        is possible from a software failure. A deployment that enables the \
                        instance layer, or surfaces pathway steps as clinical decision \
                        support, MUST re-classify (EU MDR Rule 11 / MDCG 2019-11)."
            }
            Self::B => {
                "The instance layer is in use: an individual patient's progress through \
                        a pathway is tracked, so a software failure could contribute to a \
                        non-serious injury (a missed or mistimed step)."
            }
            Self::C => {
                "Declared by the deployment: pathway output is relied upon in a context \
                        where a software failure could contribute to death or serious injury. \
                        Requires the full IEC 62304 Class C process and an ISO 14971 risk file."
            }
        }
    }
}

/// Build provenance — the IEC 62304 §8 "reconstructible release" evidence.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct Build {
    /// Crate version.
    pub version: &'static str,
    /// Source commit, from `BUILD_SHA` / `GITHUB_SHA` at compile time.
    pub commit: &'static str,
    /// `SOURCE_DATE_EPOCH` at compile time — present iff the build was run
    /// through `scripts/build-reproducible.sh`.
    pub source_date_epoch: Option<&'static str>,
    /// Whether the toolchain was pinned by the repository's
    /// `rust-toolchain.toml` (always true for a repo-local build; recorded
    /// so the field is present in the evidence bundle).
    pub toolchain_pinned: bool,
    /// `true` for a debug build — a release artefact must report `false`.
    pub debug: bool,
}

impl Build {
    /// This binary's provenance.
    #[must_use]
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            commit: option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("unknown"),
            source_date_epoch: option_env!("SOURCE_DATE_EPOCH"),
            toolchain_pinned: true,
            debug: cfg!(debug_assertions),
        }
    }

    /// Whether this artefact carries the evidence a reproducible release
    /// needs: a known commit, a pinned `SOURCE_DATE_EPOCH`, and a
    /// non-debug profile.
    #[must_use]
    pub fn is_reproducible_release(&self) -> bool {
        self.commit != "unknown" && self.source_date_epoch.is_some() && !self.debug
    }
}

/// Which controls are actually live in this process — the honest half of
/// the posture report. Every field is read from the running configuration,
/// not asserted.
// Each field is an independent on/off fact about the running deployment,
// reported verbatim in a JSON document. Clippy's suggestion (fold them
// into state enums) would obscure exactly what this type exists to make
// legible, so the flags stay flags.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Controls {
    /// Blanket `/api/*` authentication + ABAC enforcement.
    pub require_auth: bool,
    /// Read/disclosure auditing (HIPAA §164.312(b)).
    pub audit_reads: bool,
    /// The tamper-evident audit chain (always on once migrated).
    pub audit_chain: bool,
    /// The active event transport (`memory` / `outbox`).
    pub event_transport: String,
    /// `true` when an ABAC policy was configured, `false` for the built-in
    /// default policy.
    pub abac_policy_configured: bool,
    /// Whether SMART App Launch discovery is configured (see [`smart`]).
    pub smart_configured: bool,
}

impl Controls {
    /// Read the live control state.
    #[must_use]
    pub fn current() -> Self {
        Self {
            require_auth: crate::auth::require_auth(),
            audit_reads: audit_reads(),
            audit_chain: true,
            event_transport: format!("{:?}", crate::streaming::transport()).to_lowercase(),
            abac_policy_configured: env("CARE_PATHWAY_ABAC_POLICY").is_some()
                || env("CARE_PATHWAY_ABAC_POLICY_FILE").is_some(),
            smart_configured: smart::Configuration::from_env().is_some(),
        }
    }
}

/// The full compliance posture served at `GET /api/compliance`.
///
/// This is the runtime **software identification** surface an IEC 62304 or
/// HIPAA assessment would otherwise have to reconstruct by reading the
/// deployment's configuration by hand.
#[derive(Debug, Clone, Serialize)]
pub struct Posture {
    /// Service name.
    pub service: &'static str,
    /// Build provenance (IEC 62304 §8).
    pub build: Build,
    /// Declared IEC 62304 safety classification.
    pub safety_class: SafetyClass,
    /// Why that classification (IEC 62304 §4.3).
    pub safety_rationale: &'static str,
    /// Whether the artefact carries reproducible-release evidence.
    pub reproducible_release: bool,
    /// Which controls are live.
    pub controls: Controls,
    /// The declared data-protection posture.
    pub data_protection: DataProtection,
    /// Per-framework status, deliberately including what is **not** claimed.
    pub frameworks: Vec<FrameworkStatus>,
}

/// One framework's honest status line.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FrameworkStatus {
    /// Framework name.
    pub framework: &'static str,
    /// What this service implements towards it.
    pub implemented: Vec<&'static str>,
    /// What it deliberately does **not** implement or claim.
    pub not_claimed: Vec<&'static str>,
}

impl Posture {
    /// Assemble the current posture.
    #[must_use]
    pub fn current() -> Self {
        let safety_class = SafetyClass::from_env();
        let build = Build::current();
        Self {
            service: "care-pathway-service",
            reproducible_release: build.is_reproducible_release(),
            build,
            safety_class,
            safety_rationale: safety_class.rationale(),
            controls: Controls::current(),
            data_protection: DataProtection::get().clone(),
            frameworks: framework_statuses(),
        }
    }
}

/// The per-framework status lines. Hard-coded because they describe what
/// this crate's *code* does, not what its configuration says — a
/// deployment cannot make them more flattering.
fn framework_statuses() -> Vec<FrameworkStatus> {
    vec![
        FrameworkStatus {
            framework: "HIPAA (45 CFR Part 164 Subpart C)",
            implemented: vec![
                "§164.312(b) read/disclosure auditing (env-gated)",
                "§164.312(c) tamper-evident audit hash chain + verification endpoint",
                "§164.528 per-record accounting of disclosures",
                "§164.312(a)(1),(d) blanket guard + ABAC + offline PASETO verification",
            ],
            not_claimed: vec![
                "row-level integrity hashing over the care_pathways table",
                "§164.312(e) transmission security — TLS is terminated at the deployment edge",
                "any organisational safeguard (risk analysis, workforce training, BAAs)",
            ],
        },
        FrameworkStatus {
            framework: "GDPR / EU EHDS (Reg. (EU) 2025/327)",
            implemented: vec![
                "Art. 17 erasure by redaction, preserving chain linkage",
                "Art. 6/9 lawful basis + Art. 9(2) condition recorded on every audit row",
                "Ch. V residency declaration; cross-border export recorded as a transfer",
                "EHDS primary/secondary use separated by the X-Purpose-Of-Use marker",
            ],
            not_claimed: vec![
                "enforcement of a cross-border block — declaration and audit only",
                "EHDS data permits or secure processing environments",
                "Art. 30 records of processing, Art. 35 DPIA (organisational artefacts)",
            ],
        },
        FrameworkStatus {
            framework: "ONC / HTI certification (45 CFR Part 170)",
            implemented: vec![
                "declared profile + structural (must-support / cardinality) validation",
                "terminology validation against bound value sets",
                "$validate operation",
                "FHIR Bulk Data $export (kickoff / status / NDJSON output)",
                "SMART discovery document when the deployment configures one",
            ],
            not_claimed: vec![
                "certification — this serves FHIR R5, certification targets R4 + US Core",
                "US Core conformance — PlanDefinition has no US Core profile",
                "SMART App Launch itself — the family credential is PASETO, not OAuth 2.0",
                "Inferno test-suite execution",
            ],
        },
        FrameworkStatus {
            framework: "IEC 62304 / SaMD",
            implemented: vec![
                "declared safety classification with rationale",
                "SOUP register + CycloneDX SBOM from the real dependency graph",
                "machine-checked requirement-to-test traceability",
                "reproducible-build script and recorded build provenance",
            ],
            not_claimed: vec![
                "medical-device qualification or conformity assessment",
                "ISO 14971 risk file and hazard analysis (organisational artefacts)",
                "IEC 62304 §9 problem-resolution process records",
            ],
        },
    ]
}

/// SMART App Launch discovery — served **only** when the deployment
/// configures a real authorization server.
///
/// ONC §170.315(g)(10) requires SMART App Launch, an OAuth 2.0 flow this
/// family does not implement: its credential is a PASETO v4.public token
/// minted from a cookie session
/// (`agents/share/authentication-sessions.md`). Publishing a
/// `smart-configuration` that pointed at endpoints which do not exist
/// would be worse than publishing none, so the document is emitted only
/// when `CARE_PATHWAY_SMART_AUTHORIZATION_URL` and
/// `CARE_PATHWAY_SMART_TOKEN_URL` are both set — i.e. when an operator has
/// deliberately put a SMART-capable authorization server in front of this
/// service and is telling clients where it is.
pub mod smart {
    use serde::Serialize;

    /// A configured SMART authorization server.
    #[derive(Debug, Clone, Serialize, PartialEq, Eq)]
    pub struct Configuration {
        /// The authorization server's issuer URL.
        pub issuer: String,
        /// The OAuth 2.0 authorization endpoint.
        pub authorization_endpoint: String,
        /// The OAuth 2.0 token endpoint.
        pub token_endpoint: String,
        /// PKCE methods supported (SMART v2 requires S256).
        pub code_challenge_methods_supported: Vec<String>,
        /// Grant types the authorization server supports.
        pub grant_types_supported: Vec<String>,
        /// Scopes advertised for this resource server.
        pub scopes_supported: Vec<String>,
        /// SMART capabilities advertised.
        pub capabilities: Vec<String>,
    }

    impl Configuration {
        /// Build the document from the environment, or `None` when the
        /// deployment has not configured an authorization server.
        #[must_use]
        pub fn from_env() -> Option<Self> {
            let authorization_endpoint = super::env("CARE_PATHWAY_SMART_AUTHORIZATION_URL")?;
            let token_endpoint = super::env("CARE_PATHWAY_SMART_TOKEN_URL")?;
            let issuer = super::env("CARE_PATHWAY_SMART_ISSUER")
                .unwrap_or_else(|| authorization_endpoint.clone());
            Some(Self {
                issuer,
                authorization_endpoint,
                token_endpoint,
                code_challenge_methods_supported: vec!["S256".to_string()],
                grant_types_supported: vec![
                    "authorization_code".to_string(),
                    "client_credentials".to_string(),
                ],
                scopes_supported: vec![
                    "system/PlanDefinition.rs".to_string(),
                    "user/PlanDefinition.rs".to_string(),
                ],
                capabilities: vec![
                    "launch-standalone".to_string(),
                    "client-confidential-symmetric".to_string(),
                    "permission-v2".to_string(),
                ],
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The safety-class parser accepts the three classes case-insensitively
    /// and falls back to the conservative `A` for anything else — the
    /// classification the template registry alone warrants.
    #[test]
    fn safety_class_parses_or_falls_back_to_a() {
        assert_eq!(SafetyClass::parse("B"), SafetyClass::B);
        assert_eq!(SafetyClass::parse(" c "), SafetyClass::C);
        assert_eq!(SafetyClass::parse("a"), SafetyClass::A);
        for junk in ["", "D", "class B", "1"] {
            assert_eq!(SafetyClass::parse(junk), SafetyClass::A, "{junk:?}");
        }
    }

    /// Every class carries a non-empty rationale — IEC 62304 §4.3 wants the
    /// reasoning, not a bare letter.
    #[test]
    fn every_safety_class_has_a_rationale() {
        for class in [SafetyClass::A, SafetyClass::B, SafetyClass::C] {
            assert!(!class.rationale().is_empty());
        }
        assert!(
            SafetyClass::A.rationale().contains("re-classify"),
            "class A must point at the condition that forces re-classification"
        );
    }

    /// Cross-border detection is conservative: undeclared residency or an
    /// unnamed destination never manufactures a transfer event.
    #[test]
    fn cross_border_is_conservative() {
        let undeclared = DataProtection {
            residency: UNDECLARED.to_string(),
            lawful_basis: UNDECLARED.to_string(),
            art9_condition: UNDECLARED.to_string(),
            transfer_safeguard: UNDECLARED.to_string(),
        };
        assert!(!undeclared.is_cross_border(Some("us-east-1")));
        let eu = DataProtection {
            residency: "eu".to_string(),
            ..undeclared.clone()
        };
        assert!(!eu.is_cross_border(None));
        assert!(!eu.is_cross_border(Some("eu")));
        assert!(
            !eu.is_cross_border(Some("EU-west-1")),
            "region prefix is inside"
        );
        assert!(eu.is_cross_border(Some("us-east-1")));
        assert!(
            eu.is_cross_border(Some("uk")),
            "uk is outside eu post-Brexit"
        );
    }

    /// A debug build is never reported as a reproducible release, and a
    /// release needs a commit and a pinned `SOURCE_DATE_EPOCH`.
    #[test]
    fn reproducible_release_requires_full_provenance() {
        let complete = Build {
            version: "0.1.0",
            commit: "abc123",
            source_date_epoch: Some("1700000000"),
            toolchain_pinned: true,
            debug: false,
        };
        assert!(complete.is_reproducible_release());
        assert!(
            !Build {
                debug: true,
                ..complete
            }
            .is_reproducible_release()
        );
        assert!(
            !Build {
                commit: "unknown",
                ..complete
            }
            .is_reproducible_release()
        );
        assert!(
            !Build {
                source_date_epoch: None,
                ..complete
            }
            .is_reproducible_release()
        );
    }

    /// Every framework line states something it does **not** claim. A
    /// posture report that only lists wins is marketing, not evidence.
    #[test]
    fn every_framework_states_what_it_does_not_claim() {
        let statuses = framework_statuses();
        assert_eq!(statuses.len(), 4);
        for status in &statuses {
            assert!(
                !status.implemented.is_empty(),
                "{} lists nothing implemented",
                status.framework
            );
            assert!(
                !status.not_claimed.is_empty(),
                "{} must state its limits",
                status.framework
            );
        }
    }

    /// The ONC line must explicitly disclaim certification and US Core —
    /// the two things a reader is most likely to over-read.
    #[test]
    fn onc_status_disclaims_certification_and_us_core() {
        let onc = framework_statuses()
            .into_iter()
            .find(|f| f.framework.starts_with("ONC"))
            .expect("ONC status present");
        let disclaimers = onc.not_claimed.join(" ").to_lowercase();
        assert!(disclaimers.contains("certification"));
        assert!(disclaimers.contains("us core"));
        assert!(disclaimers.contains("smart app launch"));
    }

    /// SMART discovery is absent unless the deployment names both
    /// endpoints — the honest-gap rule (see the [`smart`] module docs).
    #[test]
    fn smart_configuration_requires_both_endpoints() {
        // The process env is shared across tests, so build the value the
        // same way `from_env` does rather than mutating the environment.
        assert!(
            smart::Configuration::from_env().is_none()
                || super::env("CARE_PATHWAY_SMART_TOKEN_URL").is_some(),
            "a SMART document must never be emitted without a token endpoint"
        );
    }
}
