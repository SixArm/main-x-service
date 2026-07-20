//! The registry-insight round-trip: the directory / coverage /
//! variants / providers / languages derived views over a seeded set
//! of pathways.

use care_pathway_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded registry, five views
async fn insight_views_round_trip() {
    request::<App, _, _>(|request, _ctx| async move {
        let create = async |body: Value| {
            let response = request.post("/api/care-pathways").json(&body).await;
            assert_eq!(response.status_code(), 200, "create should succeed");
        };
        // Diabetes: two providers, two settings, two jurisdictions,
        // one only in English.
        create(json!({
            "name": "Diabetes primary care (NICE)",
            "provider_id": "nice-uk",
            "care_setting": "PrimaryCare",
            "condition_codes": [{"system": "Icd10", "code": "E11"}],
            "keywords": ["specialty:endocrinology", "jurisdiction:gb"],
            "in_language": ["en"],
        })).await;
        create(json!({
            "name": "Diabetes community care (Ontario)",
            "provider_id": "ontario-ca",
            "care_setting": "Community",
            "condition_codes": [{"system": "Icd10", "code": "E11"}],
            "keywords": ["specialty:endocrinology", "jurisdiction:ca"],
            "in_language": ["en", "fr"],
        })).await;
        // Glaucoma: outpatient ophthalmology, single provider, no
        // primary-care or emergency pathway → two coverage gaps.
        create(json!({
            "name": "Glaucoma outpatient (Moorfields)",
            "provider_id": "moorfields-uk",
            "care_setting": "Outpatient",
            "condition_codes": [{"system": "Icd10", "code": "H40"}],
            "keywords": ["specialty:ophthalmology", "jurisdiction:gb"],
            "in_language": ["en"],
        })).await;

        // ── Directory: settings faceted, specialty counts.
        let directory: Value = request.get("/api/care-pathways/insights/directory").await.json();
        assert_eq!(directory["total"], 3);
        assert_eq!(directory["by_setting"]["PrimaryCare"].as_array().unwrap().len(), 1);
        assert_eq!(directory["by_specialty"]["endocrinology"], 2);
        assert_eq!(directory["by_specialty"]["ophthalmology"], 1);

        // ── Coverage: E11 covered in primary care; H40 gapped there
        // and in emergency; neither has an emergency pathway.
        let coverage: Value = request.get("/api/care-pathways/insights/coverage").await.json();
        let gap_rules: Vec<&str> = coverage["gaps"].as_array().unwrap().iter()
            .filter_map(|g| g["rule"].as_str()).collect();
        // E11 has no emergency; H40 has no primary and no emergency.
        assert_eq!(gap_rules.iter().filter(|r| **r == "no_emergency_pathway").count(), 2);
        assert_eq!(gap_rules.iter().filter(|r| **r == "no_primary_care_pathway").count(), 1);

        // ── Variants: E11 offered by two providers.
        let variants: Value = request.get("/api/care-pathways/insights/variants").await.json();
        let e11 = variants["variants"].as_array().unwrap().iter()
            .find(|v| v["condition"] == "Icd10:E11").expect("E11 variant").clone();
        assert_eq!(e11["providers"], 2);
        assert!(e11["by_provider"].get("nice-uk").is_some());
        assert!(e11["by_provider"].get("ontario-ca").is_some());
        // H40 has a single provider ⇒ not a variant.
        assert!(variants["variants"].as_array().unwrap().iter()
            .all(|v| v["condition"] != "Icd10:H40"));

        // ── Providers: three providers, each one pathway.
        let providers: Value = request.get("/api/care-pathways/insights/providers").await.json();
        assert_eq!(providers["providers"].as_array().unwrap().len(), 3);
        let nice = providers["providers"].as_array().unwrap().iter()
            .find(|p| p["provider"] == "nice-uk").expect("nice").clone();
        assert_eq!(nice["by_setting"]["PrimaryCare"], 1);

        // ── Languages: en on all three; fr on one; E11 covered in en+fr
        // (not single-language), H40 single-language en.
        let languages: Value = request.get("/api/care-pathways/insights/languages").await.json();
        assert_eq!(languages["by_language"]["en"], 3);
        assert_eq!(languages["by_language"]["fr"], 1);
        let single: Vec<&str> = languages["single_language_conditions"].as_array().unwrap()
            .iter().filter_map(|c| c["condition"].as_str()).collect();
        assert!(single.contains(&"Icd10:H40"));
        assert!(!single.contains(&"Icd10:E11"));
    })
    .await;
}
