//! A Keycloak access token through the real blanket guard: authentication
//! (`request_claims`), the role map onto ABAC attributes, and the ABAC
//! decision, with `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` on. DB-free: a local OIDC
//! provider (`authentication_verifier::test_idp`) stands in for Keycloak.
//!
//! One test function because the verifiers, policy and flag live in
//! process-wide `OnceLock`s that read the environment on first use.
#![cfg(feature = "keycloak")]

use authentication_verifier::test_idp::TestIdp;
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, Method};
use project_portfolio_management_service::auth;
use serde_json::json;

fn bearer(token: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
    h
}

async fn status(method: &Method, path: &str, headers: &HeaderMap) -> u16 {
    match auth::enforce_request(true, method, path, headers, &auth::policy().current()).await {
        Ok(()) => 200,
        Err((code, _)) => code.as_u16(),
    }
}

#[tokio::test]
async fn a_keycloak_token_is_authenticated_and_authorized_through_the_real_guard() {
    let idp = TestIdp::start().await;
    unsafe {
        std::env::set_var("PROJECT_PORTFOLIO_MANAGEMENT_KEYCLOAK_URL", &idp.base_url);
        std::env::set_var("PROJECT_PORTFOLIO_MANAGEMENT_KEYCLOAK_REALM", "mxi");
        std::env::set_var("PROJECT_PORTFOLIO_MANAGEMENT_KEYCLOAK_AUDIENCES", "mxi-api");
        std::env::set_var(
            "PROJECT_PORTFOLIO_MANAGEMENT_KEYCLOAK_ROLE_MAP",
            json!({ "editor": { "access": ["write"] }, "root": { "access": ["admin"] } })
                .to_string(),
        );
    }
    auth::init().await;
    assert!(auth::keycloak_verifier().is_some(), "keycloak configured");

    let list = "/api/plans";
    let merge = "/api/plans/merge";
    let editor = bearer(&idp.token(|_| {}));
    let admin = bearer(&idp.token(|c| c["realm_access"] = json!({ "roles": ["root"] })));
    let nobody = bearer(&idp.token(|c| c["realm_access"] = json!({ "roles": ["offline_access"] })));

    // No credential, and public paths stay open.
    assert_eq!(status(&Method::GET, list, &HeaderMap::new()).await, 401);
    assert_eq!(
        status(&Method::GET, "/_health", &HeaderMap::new()).await,
        200
    );

    // `editor` maps to access=write: read and write, never delete or merge.
    assert_eq!(status(&Method::GET, list, &editor).await, 200);
    assert_eq!(status(&Method::POST, list, &editor).await, 200);
    assert_eq!(status(&Method::DELETE, list, &editor).await, 403);
    assert_eq!(status(&Method::POST, merge, &editor).await, 403);

    // `root` maps to access=admin: destructive allowed.
    assert_eq!(status(&Method::DELETE, list, &admin).await, 200);
    assert_eq!(status(&Method::POST, merge, &admin).await, 200);

    // A valid token with only unmapped roles is authenticated but grants
    // nothing: read-only under the default policy.
    assert_eq!(status(&Method::GET, list, &nobody).await, 200);
    assert_eq!(status(&Method::POST, list, &nobody).await, 403);

    // Bad credentials are 401, never 200/403.
    let expired = bearer(&idp.token(|c| c["exp"] = json!(1_000_000_000)));
    let wrong_aud = bearer(&idp.token(|c| c["aud"] = json!(["other-client"])));
    assert_eq!(status(&Method::GET, list, &expired).await, 401);
    assert_eq!(status(&Method::GET, list, &wrong_aud).await, 401);
    assert_eq!(status(&Method::GET, list, &bearer("garbage")).await, 401);
    // A PASETO-shaped token is routed to the PASETO verifier (or refused
    // outright in a Keycloak-only build), never to Keycloak.
    assert_eq!(
        status(&Method::GET, list, &bearer("v4.public.AAAA")).await,
        401
    );
}
