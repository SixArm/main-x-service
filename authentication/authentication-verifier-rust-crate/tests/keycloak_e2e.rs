//! End-to-end test of the `keycloak` feature: a real RS256-signed JWT is
//! verified against a real OIDC discovery document and JWKS served by the
//! crate's local `test_idp`, exercising the engine (`axum-keycloak-auth`)
//! and our issuer / verified-email / role-map layer together.
#![cfg(feature = "test-idp")]

use authentication_verifier::VerifyError;
use authentication_verifier::keycloak::{KeycloakSettings, KeycloakVerifier};
use authentication_verifier::test_idp::TestIdp;
use serde_json::json;

fn settings(base: &str) -> KeycloakSettings {
    serde_json::from_value(json!({
        "url": base,
        "realm": "mxi",
        "audiences": ["mxi-api"],
        "role_map": { "editor": { "access": ["write"] }, "root": { "access": ["admin"] } },
    }))
    .unwrap()
}

async fn verifier() -> (KeycloakVerifier, TestIdp) {
    let idp = TestIdp::start().await;
    (KeycloakVerifier::new(settings(&idp.base_url)).unwrap(), idp)
}

fn now() -> i64 {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap()
}

#[tokio::test]
async fn a_valid_token_yields_claims_with_mapped_attrs() {
    let (v, idp) = verifier().await;
    let claims = v
        .verify(&idp.token(|_| {}))
        .await
        .expect("valid token verifies");
    assert_eq!(claims.sub, "0c4f1e2a-0000-4000-8000-000000000001");
    assert_eq!(claims.email, "ada@example.com");
    assert_eq!(claims.aud, "mxi-api");
    assert_eq!(claims.sid, "sess-1");
    assert_eq!(claims.attrs["access"], vec!["write".to_string()]);
    assert!(!claims.attrs.contains_key("svc"));
}

#[tokio::test]
async fn an_admin_role_maps_to_the_admin_attribute() {
    let (v, idp) = verifier().await;
    let t = idp.token(|c| c["realm_access"] = json!({ "roles": ["root"] }));
    let claims = v.verify(&t).await.unwrap();
    assert_eq!(claims.attrs["access"], vec!["admin".to_string()]);
}

#[tokio::test]
async fn an_expired_token_is_rejected() {
    let (v, idp) = verifier().await;
    let t = idp.token(|c| c["exp"] = json!(now() - 3600));
    assert!(matches!(v.verify(&t).await, Err(VerifyError::Keycloak(_))));
}

#[tokio::test]
async fn a_token_for_another_audience_is_rejected() {
    let (v, idp) = verifier().await;
    let t = idp.token(|c| c["aud"] = json!(["some-other-client"]));
    assert!(matches!(v.verify(&t).await, Err(VerifyError::Keycloak(_))));
}

#[tokio::test]
async fn a_wrong_issuer_is_rejected_even_with_a_valid_signature() {
    let (v, idp) = verifier().await;
    let t = idp.token(|c| c["iss"] = json!("https://evil.example.com/realms/mxi"));
    assert!(matches!(v.verify(&t).await, Err(VerifyError::Claim(_))));
}

#[tokio::test]
async fn an_unverified_email_is_rejected() {
    let (v, idp) = verifier().await;
    let t = idp.token(|c| c["email_verified"] = json!(false));
    assert!(matches!(v.verify(&t).await, Err(VerifyError::Claim(_))));
}

#[tokio::test]
async fn a_tampered_signature_is_rejected() {
    let (v, idp) = verifier().await;
    let mut t = idp.token(|_| {});
    // Flip a character in the middle of the signature segment.
    let dot = t.rfind('.').unwrap();
    let mid = dot + 1 + (t.len() - dot - 1) / 2;
    let flipped = if &t[mid..=mid] == "A" { "B" } else { "A" };
    t.replace_range(mid..=mid, flipped);
    assert!(v.verify(&t).await.is_err());
}

#[tokio::test]
async fn garbage_and_paseto_shaped_tokens_are_rejected_not_panicked() {
    let (v, _) = verifier().await;
    for bad in ["", "x", "a.b.c", "v4.public.AAAA", "Bearer abc"] {
        assert!(v.verify(bad).await.is_err(), "{bad:?}");
    }
}

#[tokio::test]
async fn an_unreachable_realm_rejects_everything_and_does_not_panic() {
    // Nothing listens on this port: discovery fails, so every token is
    // refused (fail-closed) rather than accepted or crashing the service.
    let idp = TestIdp::start().await;
    let v = KeycloakVerifier::new(settings("http://127.0.0.1:9")).unwrap();
    assert!(v.verify(&idp.token(|_| {})).await.is_err());
}
