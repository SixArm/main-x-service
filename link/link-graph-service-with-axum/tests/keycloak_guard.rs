//! A Keycloak access token through the real blanket guard: authentication
//! (`request_claims`), the role map onto ABAC attributes, and the ABAC
//! decision, with `LINK_GRAPH_REQUIRE_AUTH` on. DB-free: a local OIDC
//! provider (`authentication_verifier::test_idp`) stands in for Keycloak.
//!
//! This service is read-only to the world, so the blanket guard only ever
//! decides `read`; the destructive decision is the forced-reconcile
//! control-plane action (`authorize_reconcile`), which is exercised too.
//!
//! One test function because the verifiers, policy and flag live in
//! process-wide `OnceLock`s that read the environment on first use.
#![cfg(feature = "keycloak")]

use authentication_verifier::test_idp::TestIdp;
use axum::http::HeaderMap;
use axum::http::header::AUTHORIZATION;
use link_graph_service::auth;
use serde_json::json;

fn bearer(token: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
    h
}

async fn read_status(path: &str, headers: &HeaderMap) -> u16 {
    match auth::enforce_request(true, path, headers, &auth::policy().current()).await {
        Ok(()) => 200,
        Err((code, _)) => code.as_u16(),
    }
}

async fn reconcile_status(headers: &HeaderMap) -> u16 {
    match auth::authorize_reconcile(headers).await {
        Ok(()) => 200,
        Err((code, _)) => code.as_u16(),
    }
}

#[tokio::test]
async fn a_keycloak_token_is_authenticated_and_authorized_through_the_real_guard() {
    let idp = TestIdp::start().await;
    unsafe {
        std::env::set_var("LINK_GRAPH_REQUIRE_AUTH", "true");
        std::env::set_var("LINK_GRAPH_KEYCLOAK_URL", &idp.base_url);
        std::env::set_var("LINK_GRAPH_KEYCLOAK_REALM", "mxi");
        std::env::set_var("LINK_GRAPH_KEYCLOAK_AUDIENCES", "mxi-api");
        std::env::set_var(
            "LINK_GRAPH_KEYCLOAK_ROLE_MAP",
            json!({ "editor": { "access": ["write"] }, "root": { "access": ["admin"] } })
                .to_string(),
        );
    }
    assert!(auth::require_auth(), "enforcement on");
    auth::init().await;
    assert!(auth::keycloak_verifier().is_some(), "keycloak configured");

    let edges = "/api/edges";
    let editor = bearer(&idp.token(|_| {}));
    let admin = bearer(&idp.token(|c| c["realm_access"] = json!({ "roles": ["root"] })));
    let nobody = bearer(&idp.token(|c| c["realm_access"] = json!({ "roles": ["offline_access"] })));
    let none = HeaderMap::new();

    // No credential, and public paths stay open.
    assert_eq!(read_status(edges, &none).await, 401);
    assert_eq!(read_status("/_health", &none).await, 200);

    // Any authenticated caller may read under the default policy,
    // including one whose roles all go unmapped.
    assert_eq!(read_status(edges, &editor).await, 200);
    assert_eq!(read_status(edges, &admin).await, 200);
    assert_eq!(read_status(edges, &nobody).await, 200);

    // The forced reconcile is destructive: admin only.
    assert_eq!(reconcile_status(&none).await, 401);
    assert_eq!(reconcile_status(&editor).await, 403);
    assert_eq!(reconcile_status(&nobody).await, 403);
    assert_eq!(reconcile_status(&admin).await, 200);

    // Bad credentials are 401, never 200/403.
    let expired = bearer(&idp.token(|c| c["exp"] = json!(1_000_000_000)));
    let wrong_aud = bearer(&idp.token(|c| c["aud"] = json!(["other-client"])));
    assert_eq!(read_status(edges, &expired).await, 401);
    assert_eq!(read_status(edges, &wrong_aud).await, 401);
    assert_eq!(read_status(edges, &bearer("garbage")).await, 401);
    // A PASETO-shaped token is routed to the PASETO verifier (or refused
    // outright in a Keycloak-only build), never to Keycloak.
    assert_eq!(read_status(edges, &bearer("v4.public.AAAA")).await, 401);
}
