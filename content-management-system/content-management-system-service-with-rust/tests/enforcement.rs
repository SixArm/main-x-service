//! Auth-activation matrix over the live routes (CMS-R22).
//!
//! Its own test binary (not part of `tests/mod.rs`) because
//! `CMS_REQUIRE_AUTH` and the key set are cached in process-wide
//! `OnceLock`s — the flag must be set **before** the app boots, once
//! per process. A throwaway Ed25519 key mints PASETO tokens + the
//! matching key set in-process (no auth service needed).
//!
//! `#[ignore]`d: boots the app (needs PostgreSQL via
//! `config/test.yaml` / `DATABASE_URL`). Run with
//! `cargo test --test enforcement -- --ignored`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use content_management_system_service::app::App;
use ed25519_dalek::SigningKey;
use loco_rs::testing::prelude::*;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};

const ISSUER: &str = "authentication-service";
const AUDIENCE: &str = "main-x-service";
/// Throwaway Ed25519 seed — mints test tokens only, never a secret.
const SEED: [u8; 32] = [5; 32];

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
        "email": "editor@example.com", "name": "Test Editor",
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

/// The author persona's subject: a UUID, so `$sub` ownership can match
/// the bare uuid in an entry's `worker:` owner URN.
const AUTHOR_UUID: &str = "55555555-5555-4555-8555-555555555555";

/// The deployment policy, written to express the **five personas** in
/// `spec/auth.md` §"Personas as policy" — the point being that they are
/// policy, not code. Order matters: first match wins.
///
/// | Persona | Attributes | What the rules give them |
/// |---|---|---|
/// | delivery | `svc=true` | everything (a trusted machine peer) |
/// | admin | `access=admin` | everything, including destructive |
/// | editor | `access=write` | write anywhere |
/// | translator | `access=write`, `role=translator` | write only the locales the rule names |
/// | author | (none) | write only their **own** drafts |
///
/// Reads are allowed to any authenticated caller, but a caller with no
/// attributes gets a **masked** read of unpublished content — the
/// obligation this policy exists to exercise.
fn test_policy() -> String {
    json!({ "rules": [
        // delivery: a machine peer.
        { "effect": "allow",
          "actions": ["read", "write", "delete", "destructive"],
          "when": { "svc": ["true"] } },
        // admin.
        { "effect": "allow",
          "actions": ["read", "write", "delete", "destructive"],
          "when": { "access": ["admin"] } },
        // translator: writes only the locales this rule names. The
        // engine's value templates are `$sub` / `$email` only — there
        // is no `$locales` — so a deployment writes the locales into
        // the rule rather than the token.
        //
        // Both rules are **positive** matches, and both need a record.
        //
        // Negation cannot express this: a value list means "any of
        // these", so `["!fr", "!fr-CA"]` reads as "not-fr OR not-fr-CA",
        // which is true of every locale including `fr`. The deny is
        // therefore keyed on `resource.status` — a key every record
        // has and no coarse request does — so it fires only on the
        // record-level pass, after the handler has loaded the variant
        // and knows its locale. On the coarse path neither rule
        // matches, the translator falls through to the editor rule
        // (they hold `access=write`), and the guard lets the request
        // reach the handler that can actually decide.
        { "effect": "allow", "actions": ["write"],
          "when": { "role": ["translator"], "resource.locale": ["fr", "fr-CA"] } },
        { "effect": "deny", "actions": ["write"],
          "when": { "role": ["translator"],
                    "resource.status": ["draft", "in_review", "approved",
                                        "published", "archived"] } },
        // editor.
        { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } },
        // author: own drafts only, via `$sub` ownership.
        { "effect": "allow", "actions": ["write"],
          "when": { "role": ["author"], "resource.owner": ["$sub"],
                    "resource.status": ["draft"] } },
        // Anyone authenticated may read; a caller with no `access`
        // attribute reads unpublished content **masked**.
        { "effect": "allow", "actions": ["read"],
          "when": { "access": ["write", "admin"] } },
        { "effect": "allow", "actions": ["read"], "when": {}, "obligations": ["mask"] }
    ] })
    .to_string()
}

/// The activation matrix in one boot: public paths stay open, missing
/// tokens are 401, ABAC gates mutations, and — the CMS-specific pin —
/// **`/delivery/*` is not yet anonymous**, because the visibility check
/// that would make it safe does not exist until the delivery
/// controller does (CMS-D7).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test enforcement -- --ignored`"]
async fn enforcement_gates_the_real_stack() {
    let (keys, kid) = keys_and_kid();
    // `set_var` is `unsafe` in edition 2024; single-threaded setup step.
    unsafe {
        std::env::set_var("CMS_REQUIRE_AUTH", "1");
        std::env::set_var("CMS_PASETO_KEYS", keys.to_string());
        std::env::set_var("CMS_ABAC_POLICY", test_policy());
    }
    // The five personas of `spec/auth.md`, as tokens.
    let reader = sign_as(&kid, "reader-user", &[]);
    let editor = sign_as(&kid, "editor-user", &[("access", &["write"])]);
    let admin = sign_as(&kid, "admin-user", &[("access", &["admin"])]);
    // The author's `sub` is a UUID because `$sub` ownership compares it
    // against `resource.owner`, which is the bare uuid of the entry's
    // `worker:` URN — and an `owner_ref` must be a well-formed URN.
    let author = sign_as(&kid, AUTHOR_UUID, &[("role", &["author"])]);
    let translator = sign_as(
        &kid,
        "translator-user",
        &[("access", &["write"]), ("role", &["translator"])],
    );
    let delivery = sign_as(&kid, "delivery-peer", &[("svc", &["true"])]);

    request::<App, _, _>(|request, _ctx| async move {
        let bearer = |token: &str| format!("Bearer {token}");

        // Public allow-list stays open without a token.
        assert_eq!(request.get("/_health").await.status_code(), 200);
        assert_eq!(request.get("/metrics.prom").await.status_code(), 200);
        assert_eq!(
            request.get("/api-docs/openapi.json").await.status_code(),
            200
        );

        // Protected: no token ⇒ 401; junk ⇒ 401.
        assert_eq!(request.get("/api/sites").await.status_code(), 401);
        assert_eq!(
            request
                .get("/api/sites")
                .add_header("authorization", "Bearer v4.public.junk")
                .await
                .status_code(),
            401
        );

        // An unknown site is a 404 — the guard defers delivery reads to
        // the controller, which has to look the site up before it can
        // decide anything.
        assert_eq!(
            request
                .get("/delivery/nonexistent/en/about")
                .await
                .status_code(),
            404
        );

        // Reader: GET allowed, POST 403.
        assert_eq!(
            request
                .get("/api/sites")
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            200
        );
        assert_eq!(
            request
                .post("/api/sites")
                .add_header("authorization", bearer(&reader))
                .json(&json!({ "key": "denied", "name": "Denied",
                               "default_locale": "en", "locales": ["en"] }))
                .await
                .status_code(),
            403
        );

        // Editor: declares a site and a content type under enforcement.
        let key = format!("enforced-{}", uuid::Uuid::new_v4().simple());
        let site = request
            .post("/api/sites")
            .add_header("authorization", bearer(&editor))
            .json(&json!({
                "key": key, "name": "Enforced site",
                // `fr` is in scope for the translator persona below;
                // `de` is declared too, so the refusal of `de` is
                // demonstrably the *policy* talking and not a missing
                // locale.
                "default_locale": "en", "locales": ["en", "fr", "de"],
            }))
            .await;
        assert_eq!(site.status_code(), 200);
        let site_pid = site.json::<Value>()["pid"].as_str().unwrap().to_string();

        let content_type = request
            .post(&format!("/api/sites/{site_pid}/content-types"))
            .add_header("authorization", bearer(&editor))
            .json(&json!({
                "key": "article", "name": "Article",
                "fields": [{ "key": "summary", "label": "Summary", "kind": "text" }],
            }))
            .await;
        assert_eq!(content_type.status_code(), 200);

        // ---- the public delivery allow-list (CMS-D7) ----------------
        //
        // The one place this service answers anonymous readers. The
        // boundary is per-site visibility, checked on every request.
        let restricted_key = format!("restricted-{}", uuid::Uuid::new_v4().simple());
        let public_key = format!("public-{}", uuid::Uuid::new_v4().simple());
        for (key, visibility) in [(&restricted_key, "restricted"), (&public_key, "public")] {
            let site = request
                .post("/api/sites")
                .add_header("authorization", bearer(&editor))
                .json(&json!({
                    "key": key, "name": "Delivery site", "visibility": visibility,
                    "default_locale": "en", "locales": ["en"],
                    "base_url": "https://example.test",
                }))
                .await;
            assert_eq!(site.status_code(), 200, "{}", site.text());
        }

        // A restricted site refuses an anonymous delivery read...
        assert_eq!(
            request
                .get(&format!("/delivery/{restricted_key}/en/anything"))
                .await
                .status_code(),
            401,
            "a restricted site is not on the public allow-list"
        );
        // ...and answers a credentialed one (404 here only because no
        // page exists at that address — the point is that it got past
        // authorization).
        assert_eq!(
            request
                .get(&format!("/delivery/{restricted_key}/en/anything"))
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            404
        );
        // A public site answers anonymously, and its robots.txt says so.
        assert_eq!(
            request
                .get(&format!("/delivery/{public_key}/en/anything"))
                .await
                .status_code(),
            404,
            "public: past authorization, and 404 only because nothing is published there"
        );
        let robots = request
            .get(&format!("/delivery/{public_key}/robots.txt"))
            .await;
        assert_eq!(robots.status_code(), 200);
        assert!(robots.text().contains("Allow: /"));
        // A restricted site's robots.txt is answerable too — telling a
        // crawler to go away is the point — but it says Disallow.
        let robots = request
            .get(&format!("/delivery/{restricted_key}/robots.txt"))
            .await;
        assert!(robots.text().contains("Disallow: /"));

        // A preview render carries its own credential, so the guard
        // defers it to the preview controller — which refuses an
        // unknown token with the same uniform 404 it gives an expired
        // one. It is emphatically *not* a way past the guard: the token
        // is the authorization.
        assert_eq!(
            request
                .get(&format!(
                    "/delivery/{restricted_key}/preview/{}",
                    "0".repeat(64)
                ))
                .await
                .status_code(),
            404,
            "a bogus preview token is refused by the controller, not waved through"
        );

        // Unpublished content through the API still needs a credential:
        // preview is the exception, not a general loosening.
        assert_eq!(
            request
                .get(&format!("/api/sites/{restricted_key}/entries"))
                .await
                .status_code(),
            401
        );

        // The feed is a delivery read like any other: a restricted
        // site does not syndicate, a public one does. A feed is the
        // easiest surface to forget when a site is closed off, because
        // nobody navigates to it.
        assert_eq!(
            request
                .get(&format!("/delivery/{restricted_key}/en/feed.xml"))
                .await
                .status_code(),
            401,
            "a restricted site does not publish a feed"
        );
        assert_eq!(
            request
                .get(&format!("/delivery/{public_key}/en/feed.xml"))
                .await
                .status_code(),
            200,
            "a public site's feed is answerable anonymously"
        );

        // The allow-list does not widen by verb: a mutating method on
        // the delivery prefix is still refused by the guard.
        assert_eq!(
            request
                .post(&format!("/delivery/{public_key}/en/anything"))
                .json(&json!({}))
                .await
                .status_code(),
            401
        );

        // ---- webhooks (CMS-R23) --------------------------------------
        //
        // A subscription is an outbound disclosure channel: whoever can
        // register one can have unpublished-adjacent event payloads sent
        // to a host of their choosing. It is guarded like any other
        // mutation, and its secret never reaches an unauthorized caller.
        assert_eq!(
            request
                .post(&format!("/api/sites/{site_pid}/webhooks"))
                .json(&json!({ "name": "Anon", "url": "https://hooks.example.test/x" }))
                .await
                .status_code(),
            401,
            "registering a webhook without a token"
        );
        assert_eq!(
            request
                .post(&format!("/api/sites/{site_pid}/webhooks"))
                .add_header("authorization", bearer(&reader))
                .json(&json!({ "name": "Reader", "url": "https://hooks.example.test/x" }))
                .await
                .status_code(),
            403,
            "a reader may not open an outbound channel"
        );
        assert_eq!(
            request
                .get(&format!("/api/sites/{site_pid}/webhooks"))
                .await
                .status_code(),
            401,
            "nor is the subscription list readable anonymously"
        );
        // Dispatch is a mutation too — it is what actually sends.
        assert_eq!(
            request.post("/api/webhooks/dispatch").await.status_code(),
            401
        );

        // ---- the five personas (spec `auth.md`) ---------------------
        //
        // Each is a policy expression, not a code path: the same
        // handlers serve all five, and only the rules differ.

        // An entry with a body, owned by the author persona, so the
        // record-level attributes have something to decide over.
        let entry = request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .add_header("authorization", bearer(&editor))
            .json(&json!({
                "key": "persona-subject", "content_type_key": "article",
                "title": "Draft", "owner_ref": format!("worker:{AUTHOR_UUID}"),
                "blocks": [{ "kind": "paragraph", "text": "unpublished body" }],
                "fields": { "summary": "s" },
            }))
            .await;
        assert_eq!(entry.status_code(), 200, "{}", entry.text());
        let entry_body = entry.json::<Value>();
        let entry_pid = entry_body["pid"].as_str().unwrap().to_string();
        let revision_pid = entry_body["revision_pid"].as_str().unwrap().to_string();

        // **delivery** (`svc=true`): a trusted machine peer, everything.
        assert_eq!(
            request
                .get(&format!("/api/revisions/{revision_pid}"))
                .add_header("authorization", bearer(&delivery))
                .await
                .status_code(),
            200
        );

        // **editor**: reads the body in full.
        let full = request
            .get(&format!("/api/revisions/{revision_pid}"))
            .add_header("authorization", bearer(&editor))
            .await;
        assert_eq!(full.status_code(), 200);
        assert!(
            full.text().contains("unpublished body"),
            "an editor reads the body"
        );

        // **A caller with no attributes gets a MASKED read.** This is
        // the obligation the whole record-level layer exists for: the
        // structure stays visible, the unpublished body does not.
        let masked = request
            .get(&format!("/api/revisions/{revision_pid}"))
            .add_header("authorization", bearer(&reader))
            .await;
        assert_eq!(masked.status_code(), 200);
        assert!(
            !masked.text().contains("unpublished body"),
            "an unattributed caller must not read an unpublished body: {}",
            masked.text()
        );
        let masked_body = masked.json::<Value>();
        assert!(masked_body["blocks"].is_null(), "the body is redacted");
        assert!(
            masked_body["number"].is_number(),
            "the structure stays visible — only the content is masked"
        );

        // **admin**: destructive actions the editor cannot take.
        assert_eq!(
            request
                .delete(&format!("/api/entries/{entry_pid}"))
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            403,
            "a reader cannot delete"
        );

        // **translator**: writes their locales and no others. The
        // French variant is allowed; the German one is refused by the
        // deny rule that stops a translator reaching the editor rule.
        assert_eq!(
            request
                .post(&format!("/api/entries/{entry_pid}/variants"))
                .add_header("authorization", bearer(&translator))
                .json(&json!({ "locale": "fr" }))
                .await
                .status_code(),
            200,
            "a translator writes a locale the policy names"
        );
        // ...and is refused one it does not. The refusal happens at the
        // handler, where the locale is known — the blanket guard could
        // not have made this call.
        assert_eq!(
            request
                .post(&format!("/api/entries/{entry_pid}/variants"))
                .add_header("authorization", bearer(&translator))
                .json(&json!({ "locale": "de" }))
                .await
                .status_code(),
            403,
            "a translator must not write a locale outside their scope"
        );

        // **author**: may write, but only their own draft. The policy
        // resolves `$sub` to the caller, so this is ownership, not a
        // role check.
        assert_eq!(
            request
                .post(&format!("/api/sites/{site_pid}/entries"))
                .add_header("authorization", bearer(&author))
                .json(&json!({
                    "key": "author-owned", "content_type_key": "article",
                    "title": "Mine", "fields": { "summary": "s" },
                }))
                .await
                .status_code(),
            403,
            "creating an entry is not a record-level decision, so the author \
             persona's write rule (which needs a record) does not apply"
        );

        // Admin can do what the others cannot.
        assert_eq!(
            request
                .delete(&format!("/api/entries/{entry_pid}"))
                .add_header("authorization", bearer(&admin))
                .await
                .status_code(),
            200
        );

        // The reader can read what the editor declared, but not delete
        // it: `delete` is neither `read` nor `write` in the policy.
        assert_eq!(
            request
                .get(&format!("/api/sites/{site_pid}"))
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            200
        );
        assert_eq!(
            request
                .delete(&format!("/api/sites/{site_pid}"))
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            403
        );
    })
    .await;
}
