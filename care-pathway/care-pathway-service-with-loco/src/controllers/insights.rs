//! Registry insight views — read-only derivations over the stored
//! `CarePathway` templates (the matcher DTO), for the provider /
//! setting / coverage lenses:
//!
//! - **directory** — pathways faceted by care setting + the
//!   `specialty:<x>` keyword convention.
//! - **coverage** — per condition code, which settings have a pathway,
//!   with disclosed gap rules (no primary-care / no emergency pathway).
//! - **variants** — the same condition offered by different providers /
//!   jurisdictions (a comparison directory, **never** a match signal).
//! - **providers** — pathways per issuing provider, by setting.
//! - **languages** — pathways per BCP-47 language, flagging conditions
//!   covered in a single language only.
//!
//! Facets are derived from existing DTO fields plus two disclosed
//! keyword conventions (`specialty:<x>`, `jurisdiction:<x>`); no new
//! storage, and nothing here feeds the matcher.

use care_pathway_matcher::{CarePathway, CareSetting};
use loco_rs::prelude::*;

use crate::models::care_pathways::Model as PathwayModel;

/// Load all active pathways as `(pid, name, parsed DTO)` (cap 1000).
async fn load(ctx: &AppContext) -> Result<Vec<(String, String, CarePathway)>> {
    let rows = PathwayModel::list(&ctx.db, 1000).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        out.push((row.pid.to_string(), row.name.clone(), row.to_pathway()?));
    }
    Ok(out)
}

/// The care setting as a stable string (`Custom(label)` → the label).
fn setting_key(setting: Option<&CareSetting>) -> String {
    match setting {
        None => "unspecified".to_string(),
        Some(CareSetting::Custom(label)) => label.clone(),
        Some(other) => format!("{other:?}"),
    }
}

/// One condition code as a stable `SYSTEM:code` key.
fn condition_keys(pathway: &CarePathway) -> Vec<String> {
    pathway
        .condition_codes
        .iter()
        .map(|c| format!("{:?}:{}", c.system, c.code))
        .collect()
}

/// Parse a `prefix:<value>` keyword convention (lowercased value).
fn keyword_value(pathway: &CarePathway, prefix: &str) -> Option<String> {
    pathway.keywords.iter().find_map(|keyword| {
        keyword
            .trim()
            .strip_prefix(prefix)
            .map(|value| value.trim().to_lowercase())
            .filter(|value| !value.is_empty())
    })
}

/// The issuing provider as a stable key (`provider_id`, else
/// `provider_name`, else `unattributed`).
fn provider_key(pathway: &CarePathway) -> String {
    pathway
        .provider_id
        .clone()
        .or_else(|| pathway.provider_name.clone())
        .unwrap_or_else(|| "unattributed".to_string())
}

/// `{pid, name}` reference for a pathway.
fn pathway_ref(pid: &str, name: &str) -> serde_json::Value {
    serde_json::json!({ "pid": pid, "name": name })
}

/// `GET /api/care-pathways/insights/directory` — pathways faceted by
/// care setting, each with its `specialty:<x>` facet counts.
#[debug_handler]
async fn directory(State(ctx): State<AppContext>) -> Result<Response> {
    let pathways = load(&ctx).await?;
    let mut by_setting: std::collections::BTreeMap<String, Vec<serde_json::Value>> =
        std::collections::BTreeMap::new();
    let mut by_specialty: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for (pid, name, pathway) in &pathways {
        let mut entry = pathway_ref(pid, name);
        if let Some(map) = entry.as_object_mut() {
            map.insert(
                "specialty".to_string(),
                keyword_value(pathway, "specialty:").map_or(serde_json::Value::Null, serde_json::Value::from),
            );
        }
        by_setting.entry(setting_key(pathway.care_setting.as_ref())).or_default().push(entry);
        if let Some(specialty) = keyword_value(pathway, "specialty:") {
            *by_specialty.entry(specialty).or_default() += 1;
        }
    }
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "note": "settings from the DTO's care_setting; specialty from the \
                 `specialty:<x>` keyword convention",
        "total": pathways.len(),
        "by_setting": by_setting,
        "by_specialty": by_specialty,
    }))
}

/// `GET /api/care-pathways/insights/coverage` — per condition code, the
/// settings that have a pathway for it, plus disclosed gap findings:
/// a condition with a pathway somewhere but none in primary care, and
/// one with none in the emergency department.
#[debug_handler]
async fn coverage(State(ctx): State<AppContext>) -> Result<Response> {
    let pathways = load(&ctx).await?;
    // condition → set of setting keys
    let mut per_condition: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
        std::collections::BTreeMap::new();
    for (_pid, _name, pathway) in &pathways {
        let setting = setting_key(pathway.care_setting.as_ref());
        for condition in condition_keys(pathway) {
            per_condition.entry(condition).or_default().insert(setting.clone());
        }
    }
    let primary = format!("{:?}", CareSetting::PrimaryCare);
    let emergency = format!("{:?}", CareSetting::EmergencyDepartment);
    let mut gaps: Vec<serde_json::Value> = Vec::new();
    let coverage: Vec<serde_json::Value> = per_condition
        .iter()
        .map(|(condition, settings)| {
            if !settings.contains(&primary) {
                gaps.push(serde_json::json!({
                    "rule": "no_primary_care_pathway",
                    "detail": "a condition has pathways but none in primary care",
                    "condition": condition,
                }));
            }
            if !settings.contains(&emergency) {
                gaps.push(serde_json::json!({
                    "rule": "no_emergency_pathway",
                    "detail": "a condition has pathways but none in the emergency department",
                    "condition": condition,
                }));
            }
            serde_json::json!({ "condition": condition, "settings": settings })
        })
        .collect();
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "note": "coverage over condition_codes × care_setting; gaps are disclosed \
                 rules, not a readiness score",
        "conditions": coverage,
        "gaps": gaps,
    }))
}

/// `GET /api/care-pathways/insights/variants` — conditions offered by
/// **more than one provider**, each listing the provider pathways with
/// their jurisdiction facet (a cross-provider comparison directory).
#[debug_handler]
async fn variants(State(ctx): State<AppContext>) -> Result<Response> {
    let pathways = load(&ctx).await?;
    // condition → provider → [pathway with jurisdiction]
    let mut per_condition: std::collections::BTreeMap<
        String,
        std::collections::BTreeMap<String, Vec<serde_json::Value>>,
    > = std::collections::BTreeMap::new();
    for (pid, name, pathway) in &pathways {
        let provider = provider_key(pathway);
        let mut entry = pathway_ref(pid, name);
        if let Some(map) = entry.as_object_mut() {
            map.insert(
                "jurisdiction".to_string(),
                keyword_value(pathway, "jurisdiction:")
                    .map_or(serde_json::Value::Null, serde_json::Value::from),
            );
        }
        for condition in condition_keys(pathway) {
            per_condition
                .entry(condition)
                .or_default()
                .entry(provider.clone())
                .or_default()
                .push(entry.clone());
        }
    }
    let variants: Vec<serde_json::Value> = per_condition
        .iter()
        .filter(|(_, providers)| providers.len() > 1)
        .map(|(condition, providers)| {
            serde_json::json!({
                "condition": condition,
                "providers": providers.len(),
                "by_provider": providers,
            })
        })
        .collect();
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "note": "conditions with pathways from ≥2 providers; jurisdiction from the \
                 `jurisdiction:<x>` keyword convention. A comparison directory — \
                 never a match signal (cross-provider variants are not deduplicated here)",
        "variants": variants,
    }))
}

/// `GET /api/care-pathways/insights/providers` — pathways per issuing
/// provider, counted by setting.
#[debug_handler]
async fn providers(State(ctx): State<AppContext>) -> Result<Response> {
    let pathways = load(&ctx).await?;
    let mut per_provider: std::collections::BTreeMap<
        String,
        (usize, std::collections::BTreeMap<String, usize>),
    > = std::collections::BTreeMap::new();
    for (_pid, _name, pathway) in &pathways {
        let entry = per_provider.entry(provider_key(pathway)).or_default();
        entry.0 += 1;
        *entry.1.entry(setting_key(pathway.care_setting.as_ref())).or_default() += 1;
    }
    let rows: Vec<serde_json::Value> = per_provider
        .iter()
        .map(|(provider, (total, by_setting))| {
            serde_json::json!({ "provider": provider, "pathways": total, "by_setting": by_setting })
        })
        .collect();
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "note": "provider = provider_id, else provider_name, else unattributed",
        "providers": rows,
    }))
}

/// `GET /api/care-pathways/insights/languages` — pathways per BCP-47
/// language, plus the equity lens: conditions whose pathways are all in
/// a single language.
#[debug_handler]
async fn languages(State(ctx): State<AppContext>) -> Result<Response> {
    let pathways = load(&ctx).await?;
    let mut per_language: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    // condition → set of languages across its pathways
    let mut condition_langs: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
        std::collections::BTreeMap::new();
    for (_pid, _name, pathway) in &pathways {
        for language in &pathway.in_language {
            *per_language.entry(language.clone()).or_default() += 1;
        }
        for condition in condition_keys(pathway) {
            let langs = condition_langs.entry(condition).or_default();
            for language in &pathway.in_language {
                langs.insert(language.clone());
            }
        }
    }
    let single_language: Vec<serde_json::Value> = condition_langs
        .iter()
        .filter(|(_, langs)| langs.len() == 1)
        .map(|(condition, langs)| {
            serde_json::json!({ "condition": condition, "language": langs.iter().next() })
        })
        .collect();
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "note": "per-language counts over in_language; single_language_conditions is an \
                 equity lens (conditions covered in one language only)",
        "by_language": per_language,
        "single_language_conditions": single_language,
    }))
}

/// The registry-insight routes (all read-only GETs).
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/care-pathways/insights")
        .add("/directory", get(directory))
        .add("/coverage", get(coverage))
        .add("/variants", get(variants))
        .add("/providers", get(providers))
        .add("/languages", get(languages))
}
