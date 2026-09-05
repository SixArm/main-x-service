#![warn(clippy::pedantic)]

//! T-37: `ListPersons` over gRPC applies the same SEC-G3 per-record
//! read-visibility filtering/masking REST's `list_persons` /
//! `search_persons` do — a record the policy denies is omitted so its
//! existence is never revealed, and a `mask`-obligated one is redacted —
//! both driven by the exact same
//! [`person_service::api::rest::handlers::search_result_disposition`],
//! not a second copy of the rule.
//!
//! Its **own test binary**, like `tests/enforcement.rs` — see that
//! file's own doc comment for why: `PERSON_REQUIRE_AUTH`,
//! `PERSON_PASETO_KEYS`, and `PERSON_ABAC_POLICY` are process-wide
//! `OnceLock`s read once on first use, and this test turns enforcement
//! on with a resource-keyed policy. Running it in the same process as
//! `tests/grpc_integration_test.rs`'s enforcement-off suite would let
//! whichever test runs first decide the flag for both.
//!
//! `#[ignore]`d — requires `PostgreSQL`. Run with
//! `cargo test --test grpc_sec_g3 -- --ignored`.

mod common;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use person_service::api::grpc::{proto, service::PersonGrpcService};
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use sea_orm::ConnectionTrait as _;
use serde_json::json;
use serial_test::serial;
use sha2::{Digest, Sha256};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::{Endpoint, Server};

/// Throwaway Ed25519 seed: mints test tokens and the matching published
/// key set in-process. Not a secret, never used in production. Distinct
/// from `tests/enforcement.rs`'s seed only so the two test binaries'
/// fixtures cannot be confused for one another when read side by side.
const SEED: [u8; 32] = [73; 32];
const ISSUER: &str = "authentication-service";
const AUDIENCE: &str = "main-x-service";

/// The published key set (as the auth service would serve it) plus the
/// `kid` that selects the throwaway key.
fn keys_and_kid() -> (serde_json::Value, String) {
    let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
    let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
    let keys = json!({
        "keys": [{ "kty": "OKP", "crv": "Ed25519", "use": "sig",
                   "kid": kid, "x": URL_SAFE_NO_PAD.encode(public) }]
    });
    (keys, kid)
}

/// Mint a signed `v4.public` bearer carrying the given ABAC attributes.
/// The fixture records under test are created with a writer token
/// (`access: write`, needed since the policy under test declares no
/// `write` rule and the default decision denies unmatched mutations);
/// the `ListPersons` call itself uses a plain reader token with no
/// attributes, since the policy under test discriminates on the
/// *resource*, not the caller.
fn mint(kid: &str, attrs: &[(&str, &[&str])]) -> String {
    let attrs_map: serde_json::Map<String, serde_json::Value> = attrs
        .iter()
        .map(|(k, vs)| {
            (
                (*k).to_string(),
                serde_json::Value::Array(
                    vs.iter()
                        .map(|v| serde_json::Value::String((*v).to_string()))
                        .collect(),
                ),
            )
        })
        .collect();
    let iat: i64 = 1_700_000_000;
    let payload = json!({
        "sub": "33333333-3333-3333-3333-333333333333",
        "email": "reader@example.test", "name": "Reader",
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

// Note: `proto::Person.active`, though present on the wire, is not
// wired into `person_from_proto`/`CreatePerson` at all — every gRPC
// create lands `active = true` regardless of what the client sent
// (REST's `create_person`, which deserializes the full `Person`
// JSON, does not have this gap). Out of this task's scope to fix, so
// this fixture sets `active` via ground-truth SQL below instead of
// relying on it, the same way it must for `deceased`.
fn create_request(family_name: String, tax_id: Option<String>) -> proto::CreatePersonRequest {
    proto::CreatePersonRequest {
        person: Some(proto::Person {
            id: String::new(),
            active: true,
            family_name,
            given_names: vec!["SecG3".to_string()],
            gender: proto::Gender::Unknown as i32,
            birth_date: None,
            tax_id,
            created_at: String::new(),
            updated_at: String::new(),
        }),
    }
}

/// Wrap a request body with an `authorization: Bearer <token>` metadata
/// entry, so a call site reads as "this request, as this caller"
/// rather than three lines of `metadata_mut()` boilerplate each time.
fn bearer_request<T>(body: T, token: &str) -> tonic::Request<T> {
    let mut request = tonic::Request::new(body);
    request
        .metadata_mut()
        .insert("authorization", format!("Bearer {token}").parse().unwrap());
    request
}

/// Bind `PersonGrpcService` on an ephemeral port and return a connected
/// client, over the given (already policy-configured) [`AppState`].
async fn start_test_server(
    state: person_service::api::rest::AppState,
) -> proto::person_service_client::PersonServiceClient<tonic::transport::Channel> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        Server::builder()
            .add_service(proto::person_service_server::PersonServiceServer::new(
                PersonGrpcService::new(state),
            ))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .ok();
    });
    let channel = Endpoint::from_shared(format!("http://{addr}"))
        .expect("valid endpoint")
        .connect()
        .await
        .expect("connect to the test gRPC server");
    proto::person_service_client::PersonServiceClient::new(channel)
}

/// A policy that denies reading a record under `denied_org` outright
/// (never revealed to exist) and masks a deceased one (readable,
/// redacted) — everything else falls to the default decision (read
/// allow, no obligation), exactly REST's SEC-G3 path exercises via
/// `search_result_disposition`.
///
/// The denial keys on `managing_org`, not `active`: `active = false`
/// would also drop the record from `PersonRepository::list_active`'s
/// own `WHERE active = true` filter, which would make the record
/// disappear from `ListPersons` for a reason unrelated to the policy
/// under test — indistinguishable from a false positive here.
fn sec_g3_policy(denied_org: uuid::Uuid) -> serde_json::Value {
    json!({
        "rules": [
            // Supplying a custom policy replaces the built-in default
            // wholesale (authorization-attributes.md §5), so the
            // `access: write` fixtures below need an explicit grant —
            // the family's usual `access=write` ⇒ write convention,
            // spelled out rather than inherited.
            { "effect": "allow", "actions": ["write"],
              "when": { "access": ["write"] } },
            { "effect": "deny", "actions": ["read"],
              "when": { "resource.managing_org": [denied_org.to_string()] } },
            { "effect": "allow", "actions": ["read"],
              "when": { "resource.deceased": ["true"] },
              "obligations": ["mask"] }
        ]
    })
}

/// Drives three records — denied, masked, and full — through a real
/// `ListPersons` call and asserts each gets the disposition the policy
/// implies: the denied record never appears, the masked one appears
/// with its `tax_id` redacted, and the full one appears unchanged.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test grpc_sec_g3 -- --ignored`"]
async fn list_persons_over_grpc_applies_sec_g3_disposition() {
    let (keys, kid) = keys_and_kid();
    let denied_org = uuid::Uuid::new_v4();
    // Set BEFORE the app state is built — the flag, the key set, and
    // the policy are read into process-wide `OnceLock`s on first use.
    // `set_var` is `unsafe` in edition 2024; single-threaded setup.
    unsafe {
        std::env::set_var("PERSON_REQUIRE_AUTH", "1");
        std::env::set_var("PERSON_PASETO_KEYS", keys.to_string());
        std::env::set_var("PERSON_ABAC_POLICY", sec_g3_policy(denied_org).to_string());
    }

    let state = common::create_test_app_state().await;
    let db = common::db().await;
    let mut client = start_test_server(state).await;
    // The policy under test declares no `write` rule, so the default
    // decision (§5 of authorization-attributes.md) denies an unmatched
    // mutation — the fixtures need a real `access: write` token to be
    // created at all, distinct from the plain reader token the
    // `ListPersons` call under test uses.
    let writer = mint(&kid, &[("access", &["write"])]);
    let reader = mint(&kid, &[]);

    let unique = uuid::Uuid::new_v4().simple().to_string();
    let denied = client
        .create_person(bearer_request(
            create_request(format!("SecG3Denied{unique}"), None),
            &writer,
        ))
        .await
        .expect("CreatePerson (denied)")
        .into_inner();
    let masked = client
        .create_person(bearer_request(
            create_request(
                format!("SecG3Masked{unique}"),
                Some("987654321".to_string()),
            ),
            &writer,
        ))
        .await
        .expect("CreatePerson (masked)")
        .into_inner();
    let full = client
        .create_person(bearer_request(
            create_request(format!("SecG3Full{unique}"), Some("123456789".to_string())),
            &writer,
        ))
        .await
        .expect("CreatePerson (full)")
        .into_inner();

    // Ground truth: neither the denying org nor the deceased flag is
    // reachable through a create-time field on this surface (see
    // `create_request`'s note), the same reason the erasure tests
    // reach for raw SQL.
    let denied_id = uuid::Uuid::parse_str(&denied.id).expect("valid uuid");
    let masked_id = uuid::Uuid::parse_str(&masked.id).expect("valid uuid");
    // `managing_organization_id` is FK-constrained, so the denying org
    // must actually exist first.
    db.execute_raw(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO organizations (id, name) VALUES ($1, 'SEC-G3 test org')",
        [denied_org.into()],
    ))
    .await
    .expect("create the denying organization");
    db.execute_raw(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE persons SET managing_organization_id = $1 WHERE id = $2",
        [denied_org.into(), denied_id.into()],
    ))
    .await
    .expect("set managing_organization_id");
    db.execute_raw(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE persons SET deceased = true WHERE id = $1",
        [masked_id.into()],
    ))
    .await
    .expect("mark deceased");

    let request = bearer_request(
        proto::ListPersonsRequest {
            limit: 500,
            offset: 0,
        },
        &reader,
    );
    let listed = client
        .list_persons(request)
        .await
        .expect("ListPersons")
        .into_inner()
        .persons;

    assert!(
        !listed.iter().any(|p| p.id == denied.id),
        "a record the policy denies must be omitted (concealment), not merely 403'd"
    );

    let masked_hit = listed
        .iter()
        .find(|p| p.id == masked.id)
        .expect("a masked record must still appear, just redacted");
    assert_ne!(
        masked_hit.tax_id.as_deref(),
        Some("987654321"),
        "the mask obligation must redact tax_id, not pass it through verbatim"
    );
    assert!(
        masked_hit.tax_id.as_deref().unwrap().ends_with("4321"),
        "masking shows only the last 4 characters: {:?}",
        masked_hit.tax_id
    );

    let full_hit = listed
        .iter()
        .find(|p| p.id == full.id)
        .expect("a record with no denying/masking rule must appear in full");
    assert_eq!(
        full_hit.tax_id.as_deref(),
        Some("123456789"),
        "no obligation applies to the full record, so tax_id must be unredacted"
    );

    unsafe {
        std::env::remove_var("PERSON_REQUIRE_AUTH");
        std::env::remove_var("PERSON_PASETO_KEYS");
        std::env::remove_var("PERSON_ABAC_POLICY");
    }
}
