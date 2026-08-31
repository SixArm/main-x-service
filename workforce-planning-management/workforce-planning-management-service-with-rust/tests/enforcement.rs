//! Auth-activation persona matrix over the live routes (WPM-R15).
//!
//! Its own test binary (not part of `tests/mod.rs`) because
//! `WPM_REQUIRE_AUTH` and the key set are cached in process-wide
//! `OnceLock`s — the flag must be set **before** the app boots, once
//! per process. A throwaway Ed25519 key mints PASETO tokens + the
//! matching key set in-process (no auth service needed).
//!
//! `#[ignore]`d: boots the app (needs PostgreSQL via
//! `config/test.yaml` / `DATABASE_URL`). Run with
//! `cargo test --test enforcement -- --ignored`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use loco_rs::testing::prelude::*;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};
use workforce_planning_management_service::app::App;

const ISSUER: &str = "authentication-service";
const AUDIENCE: &str = "main-x-service";
/// Throwaway Ed25519 seed — mints test tokens only, never a secret.
const SEED: [u8; 32] = [9; 32];

/// The published-key-set JSON + `kid` for the test key.
fn keys_and_kid() -> (Value, String) {
    let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
    let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
    let keys = json!({
        "keys": [{ "kty": "OKP", "crv": "Ed25519", "use": "sig",
                   "kid": kid, "x": URL_SAFE_NO_PAD.encode(public) }]
    });
    (keys, kid)
}

/// Mint a PASETO `v4.public` with a given `sub` and ABAC attributes.
fn sign_as(kid: &str, sub: &str, attrs: &[(&str, &[&str])]) -> String {
    let attrs_map: serde_json::Map<String, Value> = attrs
        .iter()
        .map(|(key, values)| ((*key).to_string(), json!(values)))
        .collect();
    let iat: i64 = 1_700_000_000;
    let payload = json!({
        "sub": sub,
        "email": "person@example.com", "name": "Test Person",
        "iss": ISSUER, "aud": AUDIENCE,
        "exp": iat + 10_000_000_000_i64, "iat": iat,
        "sid": "test-sid", "attrs": attrs_map,
    })
    .to_string();
    let keypair = SigningKey::from_bytes(&SEED).to_keypair_bytes();
    let key = Key::<64>::from(keypair);
    let private = PasetoAsymmetricPrivateKey::<V4, Public>::from(&key);
    let footer = format!(r#"{{"kid":"{kid}"}}"#);
    let mut builder = Paseto::<V4, Public>::builder();
    builder.set_payload(Payload::from(payload.as_str()));
    builder.set_footer(Footer::from(footer.as_str()));
    builder.try_sign(&private).expect("sign")
}

/// The matrix runs against **the shipped reference policy file**
/// (`config/abac-policy.reference.json`, WPM-G1) — so what the
/// runbook tells a deployment to mount is exactly what is verified:
/// svc/admin everything; `payroll=true` unmasked read; `hr=true`
/// write + masked read; `$sub` self-read unmasked; masked-read
/// fallback for every other authenticated caller.
const REFERENCE_POLICY_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/config/abac-policy.reference.json"
);

/// The full persona matrix in one test (one boot ⇒ one set of cached
/// `OnceLock`s): public paths stay open, missing tokens are 401, the
/// ABAC actions gate mutations, the `$sub` self-rule reads the own
/// record with salary, and the masked fallback redacts salary and
/// payslip amounts.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test enforcement -- --ignored`"]
async fn enforcement_personas_gate_and_mask() {
    let (keys, kid) = keys_and_kid();
    // The auth OnceLocks read these on first use — set before boot.
    // `set_var` is `unsafe` in edition 2024; single-threaded setup step.
    unsafe {
        std::env::set_var("WPM_REQUIRE_AUTH", "1");
        std::env::set_var("WPM_PASETO_KEYS", keys.to_string());
        std::env::set_var("WPM_ABAC_POLICY_FILE", REFERENCE_POLICY_FILE);
    }
    let my_person = uuid::Uuid::new_v4();
    let me = sign_as(&kid, &my_person.to_string(), &[]);
    let other = sign_as(&kid, &uuid::Uuid::new_v4().to_string(), &[]);
    let writer = sign_as(&kid, "hr-user", &[("hr", &["true"])]);
    let payroll = sign_as(&kid, "payroll-user", &[("payroll", &["true"])]);
    let machine = sign_as(&kid, "svc-user", &[("svc", &["true"])]);

    request::<App, _, _>(|request, _ctx| async move {
        let bearer = |token: &str| format!("Bearer {token}");

        // Public allow-list stays open without a token.
        assert_eq!(request.get("/_health").await.status_code(), 200);
        assert_eq!(request.get("/metrics.prom").await.status_code(), 200);

        // Protected: no token ⇒ 401; junk token ⇒ 401.
        assert_eq!(request.get("/api/employees").await.status_code(), 401);
        assert_eq!(
            request
                .get("/api/employees")
                .add_header("authorization", "Bearer v4.public.junk")
                .await
                .status_code(),
            401
        );

        // Plain authenticated caller: reads allowed, mutations 403.
        assert_eq!(
            request
                .get("/api/employees")
                .add_header("authorization", bearer(&other))
                .await
                .status_code(),
            200
        );
        assert_eq!(
            request
                .post("/api/employees")
                .add_header("authorization", bearer(&other))
                .json(&json!({ "person_ref": "person:x" }))
                .await
                .status_code(),
            403
        );

        // `access=write` creates my employee record (salary present).
        let org = format!("organization:{}", uuid::Uuid::new_v4());
        let created = request
            .post("/api/employees")
            .add_header("authorization", bearer(&writer))
            .json(&json!({
                "person_ref": format!("person:{my_person}"),
                "organization_ref": org,
                "employee_number": "E-8001", "display_name": "Mask Test Person",
                "employment_type": "permanent", "department": "engineering",
                "job_title": "Engineer",
                "salary_minor": 3_600_000, "salary_currency": "GBP",
                "hired_on": "2026-01-05",
            }))
            .await;
        assert_eq!(created.status_code(), 200);
        let employee_pid = created.json::<Value>()["pid"].as_str().unwrap().to_string();

        // Self-read (`$sub` = my person uuid): salary visible.
        let mine: Value = {
            let response = request
                .get(&format!("/api/employees/{employee_pid}"))
                .add_header("authorization", bearer(&me))
                .await;
            assert_eq!(response.status_code(), 200);
            response.json()
        };
        assert_eq!(mine["salary_minor"], 3_600_000, "self-read sees the salary");

        // Another employee's read falls through to the mask rule:
        // employment facts visible, salary redacted.
        let theirs: Value = {
            let response = request
                .get(&format!("/api/employees/{employee_pid}"))
                .add_header("authorization", bearer(&other))
                .await;
            assert_eq!(response.status_code(), 200);
            response.json()
        };
        assert_eq!(theirs["department"], "engineering");
        assert!(
            theirs["salary_minor"].is_null(),
            "masked read hides the salary"
        );
        assert!(theirs["salary_currency"].is_null());

        // Payslips inherit the employee masking: build a run as the
        // machine peer, then read as the masked caller.
        let run: Value = {
            let response = request
                .post("/api/payroll-runs")
                .add_header("authorization", bearer(&machine))
                .json(&json!({
                    "organization_ref": org,
                    "period_start": "2026-07-01", "period_end": "2026-07-31",
                }))
                .await;
            assert_eq!(response.status_code(), 200);
            response.json()
        };
        let run_pid = run["pid"].as_str().unwrap();
        // Activate the employee so the run picks them up, then calculate.
        assert_eq!(
            request
                .post(&format!("/api/employees/{employee_pid}/status"))
                .add_header("authorization", bearer(&machine))
                .json(&json!({ "to": "active" }))
                .await
                .status_code(),
            200
        );
        assert_eq!(
            request
                .post(&format!("/api/payroll-runs/{run_pid}/calculate"))
                .add_header("authorization", bearer(&machine))
                .await
                .status_code(),
            200
        );
        let masked_slips: Value = {
            let response = request
                .get(&format!("/api/payroll-runs/{run_pid}/payslips"))
                .add_header("authorization", bearer(&other))
                .await;
            assert_eq!(response.status_code(), 200);
            response.json()
        };
        let slip = &masked_slips.as_array().unwrap()[0];
        assert_eq!(slip["gross_minor"], 0, "masked payslip zeroes the money");
        assert_eq!(slip["net_minor"], 0);
        assert_eq!(slip["deductions"].as_array().map(Vec::len), Some(0));
        // The self-reader sees their real payslip.
        let my_slips: Value = {
            let response = request
                .get(&format!("/api/employees/{employee_pid}/payslips"))
                .add_header("authorization", bearer(&me))
                .await;
            assert_eq!(response.status_code(), 200);
            response.json()
        };
        assert!(
            my_slips.as_array().unwrap()[0]["gross_minor"]
                .as_i64()
                .unwrap()
                > 0
        );

        // The payroll persona reads the employee unmasked.
        let payroll_view: Value = {
            let response = request
                .get(&format!("/api/employees/{employee_pid}"))
                .add_header("authorization", bearer(&payroll))
                .await;
            assert_eq!(response.status_code(), 200);
            response.json()
        };
        assert_eq!(
            payroll_view["salary_minor"], 3_600_000,
            "payroll sees salary"
        );
        // HR reads masked (salary stays payroll + self).
        let hr_view: Value = {
            let response = request
                .get(&format!("/api/employees/{employee_pid}"))
                .add_header("authorization", bearer(&writer))
                .await;
            assert_eq!(response.status_code(), 200);
            response.json()
        };
        assert!(hr_view["salary_minor"].is_null(), "hr read is masked");

        // Subject access (WPM-R30): the subject exports unmasked; a
        // masked caller is refused outright — a full export cannot be
        // "masked".
        let my_export = request
            .get(&format!("/api/employees/{employee_pid}/subject-access"))
            .add_header("authorization", bearer(&me))
            .await;
        assert_eq!(my_export.status_code(), 200);
        assert_eq!(
            my_export.json::<Value>()["employee"]["salary_minor"],
            3_600_000
        );
        assert_eq!(
            request
                .get(&format!("/api/employees/{employee_pid}/subject-access"))
                .add_header("authorization", bearer(&other))
                .await
                .status_code(),
            403,
            "masked callers cannot receive the export"
        );

        // Erasure and the sweep are destructive: hr (write) is 403;
        // even the machine peer cannot erase an ACTIVE employment
        // (the lawful basis holds regardless of privilege).
        assert_eq!(
            request
                .post(&format!("/api/employees/{employee_pid}/erase"))
                .add_header("authorization", bearer(&writer))
                .await
                .status_code(),
            403,
            "write is not destructive"
        );
        assert_eq!(
            request
                .post(&format!("/api/employees/{employee_pid}/erase"))
                .add_header("authorization", bearer(&machine))
                .await
                .status_code(),
            422,
            "active employment refuses erasure even for svc"
        );
        assert_eq!(
            request
                .post("/api/retention/sweep")
                .add_header("authorization", bearer(&writer))
                .await
                .status_code(),
            403
        );
        assert_eq!(
            request
                .post("/api/retention/sweep")
                .add_header("authorization", bearer(&machine))
                .await
                .status_code(),
            200
        );

        // 360 report under mask (WPM-R29): comments are review-content
        // tier — a masked caller keeps the numbers, loses the words.
        // A macro, not a closure: a bare `let f = |builder, token| ...;`
        // needs `builder`'s type known at the closure expression itself
        // (Rust does not infer a closure parameter's type from a later
        // call site in the same block), and that type is loco's
        // internally-pinned `axum_test::TestRequest` (17.x) — not this
        // crate's own, newer `axum-test` dev-dependency (see
        // tests/requests/mod.rs for the same rationale on seed helpers).
        // Expanding inline sidesteps naming the type at all.
        macro_rules! with_auth {
            ($builder:expr, $token:expr $(,)?) => {
                $builder.add_header("authorization", format!("Bearer {}", $token))
            };
        }
        let mut rater_pids = Vec::new();
        for n in 0..3 {
            let rater = with_auth!(request.post("/api/employees"), &machine)
                .json(&json!({
                    "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
                    "organization_ref": org,
                    "employee_number": format!("E-90{n}"),
                    "display_name": format!("Rater {n}"),
                    "employment_type": "permanent", "department": "engineering",
                    "job_title": "Engineer", "hired_on": "2026-01-05",
                }))
                .await
                .json::<Value>();
            rater_pids.push(rater["pid"].as_str().unwrap().to_string());
        }
        let appraisal = with_auth!(
            request.post(&format!("/api/employees/{employee_pid}/appraisals")),
            &machine,
        )
        .json(&json!({ "competencies": ["communication"] }))
        .await
        .json::<Value>();
        let a_pid = appraisal["pid"].as_str().unwrap().to_string();
        for (n, rater) in rater_pids.iter().enumerate() {
            let group = if n == 0 { "manager" } else { "peer" };
            with_auth!(
                request.post(&format!("/api/appraisals/{a_pid}/nominations")),
                &machine,
            )
            .json(&json!({ "rater_pid": rater, "group": group }))
            .await
            .assert_status_ok();
        }
        with_auth!(
            request.post(&format!("/api/appraisals/{a_pid}/status")),
            &machine,
        )
        .json(&json!({ "to": "collecting" }))
        .await
        .assert_status_ok();
        with_auth!(
            request.post(&format!("/api/appraisals/{a_pid}/responses")),
            &machine,
        )
        .json(&json!({ "rater_pid": rater_pids[0],
                           "scores": { "communication": 4 },
                           "comment": "Delegate more." }))
        .await
        .assert_status_ok();
        with_auth!(
            request.post(&format!("/api/appraisals/{a_pid}/status")),
            &machine,
        )
        .json(&json!({ "to": "shared" }))
        .await
        .assert_status_ok();
        let full_report: Value = with_auth!(
            request.get(&format!("/api/appraisals/{a_pid}/report")),
            &payroll,
        )
        .await
        .json();
        let manager_group = full_report["groups"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["group"] == "manager")
            .unwrap()
            .clone();
        assert_eq!(
            manager_group["comments"][0], "Delegate more.",
            "unmasked read sees words"
        );
        let masked_report: Value = with_auth!(
            request.get(&format!("/api/appraisals/{a_pid}/report")),
            &other,
        )
        .await
        .json();
        let masked_manager = masked_report["groups"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["group"] == "manager")
            .unwrap()
            .clone();
        assert_eq!(masked_manager["comments_withheld"], true);
        assert!(
            masked_manager["comments"].is_null(),
            "masked read loses the words"
        );
        assert_eq!(
            masked_manager["competencies"]["communication"]["mean"], 4.0,
            "masked read keeps the numbers"
        );

        // Writer may not delete; the machine peer may.
        assert_eq!(
            request
                .delete(&format!("/api/employees/{employee_pid}"))
                .add_header("authorization", bearer(&writer))
                .await
                .status_code(),
            403,
            "write is not delete"
        );
        assert_eq!(
            request
                .delete(&format!("/api/employees/{employee_pid}"))
                .add_header("authorization", bearer(&machine))
                .await
                .status_code(),
            200,
            "svc=true may delete"
        );
    })
    .await;
}
