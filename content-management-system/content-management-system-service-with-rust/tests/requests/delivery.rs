//! Routing and delivery (CMS-R16–R20): auto-`301` on rename, loop
//! refusal, published-only composition, ETag conditionality, sitemap
//! and robots, personalization that varies its cache key, and the
//! public/restricted boundary.

use content_management_system_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_key, a_site_payload};

/// A public site with a base URL and a routable article type.
async fn seed_public_site(request: &axum_test::TestServer, prefix: &str) -> (String, String) {
    let key = a_key(prefix);
    let mut payload = a_site_payload(&key);
    payload["visibility"] = json!("public");
    payload["base_url"] = json!("https://example.test");
    let created: Value = request.post("/api/sites").json(&payload).await.json();
    let site_pid = created["pid"].as_str().unwrap().to_string();
    request
        .post(&format!("/api/sites/{site_pid}/content-types"))
        .json(&json!({
            "key": "article", "name": "Article", "routable": true,
            "fields": [{ "key": "standfirst", "label": "Standfirst", "kind": "text" }],
        }))
        .await
        .assert_status_ok();
    (site_pid, key)
}

/// Create, route, and publish an entry. Returns its pid.
async fn publish_at(
    request: &axum_test::TestServer,
    site_pid: &str,
    key: &str,
    path: &str,
) -> String {
    let created: Value = request
        .post(&format!("/api/sites/{site_pid}/entries"))
        .json(&json!({
            "key": key, "content_type_key": "article", "title": "A page",
            "blocks": [{ "kind": "paragraph", "text": "page body" }],
        }))
        .await
        .json();
    let entry_pid = created["pid"].as_str().unwrap().to_string();
    request
        .put(&format!("/api/entries/{entry_pid}/variants/en/path"))
        .json(&json!({ "path": path }))
        .await
        .assert_status_ok();
    request
        .post(&format!("/api/entries/{entry_pid}/variants/en/transition"))
        .json(&json!({ "action": "publish" }))
        .await
        .assert_status_ok();
    entry_pid
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_published_page_is_delivered_with_an_honest_etag() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_pid, site_key) = seed_public_site(&request, "delivery").await;
        publish_at(&request, &site_pid, "about", "/about").await;

        let response = request.get(&format!("/delivery/{site_key}/en/about")).await;
        response.assert_status_ok();
        let payload: Value = response.json();
        assert_eq!(payload["entry"]["key"], "about");
        assert_eq!(payload["revision"]["title"], "A page");
        assert_eq!(payload["locale_requested"], "en");
        assert_eq!(payload["locale_served"], "en");
        assert_eq!(payload["fallback_applied"], false);
        assert_eq!(
            payload["seo"]["canonical"], "https://example.test/en/about",
            "the canonical is derived from the site's base URL and the page's address"
        );

        // The ETag ignores `as_of`, so an unchanged page keeps its tag.
        let tag = response
            .headers()
            .get("etag")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(tag.starts_with("W/\""));
        let again = request.get(&format!("/delivery/{site_key}/en/about")).await;
        assert_eq!(again.headers().get("etag").unwrap().to_str().unwrap(), tag);

        // A conditional request is answered 304.
        let conditional = request
            .get(&format!("/delivery/{site_key}/en/about"))
            .add_header("if-none-match", tag.clone())
            .await;
        assert_eq!(conditional.status_code(), 304);
    })
    .await;
}

/// Delivery can only reach published revisions, and only through the
/// composer — there is no parameter that widens it.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn drafts_are_not_deliverable() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_pid, site_key) = seed_public_site(&request, "drafts").await;
        let created: Value = request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .json(&json!({ "key": "secret", "content_type_key": "article", "title": "Embargoed" }))
            .await
            .json();
        let entry_pid = created["pid"].as_str().unwrap().to_string();
        request
            .put(&format!("/api/entries/{entry_pid}/variants/en/path"))
            .json(&json!({ "path": "/secret" }))
            .await
            .assert_status_ok();

        // Routed but never published: not deliverable.
        assert_eq!(
            request
                .get(&format!("/delivery/{site_key}/en/secret"))
                .await
                .status_code(),
            404
        );

        // Publishing makes it deliverable; unpublishing takes it away
        // again.
        request
            .post(&format!("/api/entries/{entry_pid}/variants/en/transition"))
            .json(&json!({ "action": "publish" }))
            .await
            .assert_status_ok();
        request
            .get(&format!("/delivery/{site_key}/en/secret"))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/entries/{entry_pid}/variants/en/transition"))
            .json(&json!({ "action": "unpublish", "reason": "pulled" }))
            .await
            .assert_status_ok();
        // Unpublishing leaves a `410 Gone` marker rather than a bare
        // 404: the address existed, and saying so beats silence
        // (CMS-R10).
        assert_eq!(
            request
                .get(&format!("/delivery/{site_key}/en/secret"))
                .await
                .status_code(),
            410
        );

        // Republishing clears the marker, so the page comes back to the
        // address it always had.
        request
            .post(&format!("/api/entries/{entry_pid}/variants/en/transition"))
            .json(&json!({ "action": "publish" }))
            .await
            .assert_status_ok();
        request
            .get(&format!("/delivery/{site_key}/en/secret"))
            .await
            .assert_status_ok();
    })
    .await;
}

/// Renaming a page leaves a redirect — the default, because the
/// alternative breaks every inbound link at once.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn renaming_a_page_leaves_a_redirect() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_pid, site_key) = seed_public_site(&request, "rename").await;
        let entry_pid = publish_at(&request, &site_pid, "guide", "/old-guide").await;

        let renamed: Value = request
            .put(&format!("/api/entries/{entry_pid}/variants/en/path"))
            .json(&json!({ "path": "/new-guide" }))
            .await
            .json();
        assert_eq!(renamed["redirect_created"], true);

        // The old address redirects...
        let old = request
            .get(&format!("/delivery/{site_key}/en/old-guide"))
            .await;
        assert_eq!(old.status_code(), 301);
        assert!(
            old.headers()
                .get("location")
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with("/new-guide")
        );
        // ...and the new one answers.
        request
            .get(&format!("/delivery/{site_key}/en/new-guide"))
            .await
            .assert_status_ok();

        // Rename again: the *first* address still resolves in one hop,
        // because chains are collapsed on creation.
        request
            .put(&format!("/api/entries/{entry_pid}/variants/en/path"))
            .json(&json!({ "path": "/final-guide" }))
            .await
            .assert_status_ok();
        let redirects: Value = request
            .get(&format!("/api/sites/{site_pid}/redirects"))
            .await
            .json();
        let first = redirects
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["from_path"] == "/old-guide")
            .expect("the original address still redirects");
        assert_eq!(
            first["to_path"], "/final-guide",
            "the chain was collapsed rather than lengthened"
        );

        // Paths are normalized to one form.
        let normalized: Value = request
            .put(&format!("/api/entries/{entry_pid}/variants/en/path"))
            .json(&json!({ "path": "/Final-Guide/" }))
            .await
            .json();
        assert_eq!(normalized["path"], "/final-guide");
        assert_eq!(normalized["changed"], false, "same address, nothing to do");
    })
    .await;
}

/// Loops and clashes are refused at write time, not discovered at
/// request time.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn redirect_loops_and_address_clashes_are_refused() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_pid, _) = seed_public_site(&request, "loops").await;
        publish_at(&request, &site_pid, "first", "/first").await;
        let second = publish_at(&request, &site_pid, "second", "/second").await;

        // Two live pages cannot share an address.
        let clash = request
            .put(&format!("/api/entries/{second}/variants/en/path"))
            .json(&json!({ "path": "/first" }))
            .await;
        assert_eq!(clash.status_code(), 409);
        assert!(clash.text().contains("already the address"));

        // A manual redirect that would close a cycle is refused.
        request
            .post(&format!("/api/sites/{site_pid}/redirects"))
            .json(&json!({ "locale": "en", "from_path": "/a", "to_path": "/b" }))
            .await
            .assert_status_ok();
        let loop_attempt = request
            .post(&format!("/api/sites/{site_pid}/redirects"))
            .json(&json!({ "locale": "en", "from_path": "/b", "to_path": "/a" }))
            .await;
        assert_eq!(loop_attempt.status_code(), 422);
        assert!(loop_attempt.text().contains("would create a loop"));

        // A malformed path never reaches storage.
        for bad in ["/a/../b", "/a b", "/a?x=1"] {
            let refused = request
                .put(&format!("/api/entries/{second}/variants/en/path"))
                .json(&json!({ "path": bad }))
                .await;
            assert_eq!(refused.status_code(), 422, "{bad} should be refused");
        }
    })
    .await;
}

/// A `410` marker says the page is gone rather than sending a reader
/// somewhere they did not ask for.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_gone_marker_answers_410() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_pid, site_key) = seed_public_site(&request, "gone").await;
        request
            .post(&format!("/api/sites/{site_pid}/redirects"))
            .json(&json!({ "locale": "en", "from_path": "/retired", "status": 410 }))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .get(&format!("/delivery/{site_key}/en/retired"))
                .await
                .status_code(),
            410
        );

        // A 410 with a target is a contradiction.
        let contradiction = request
            .post(&format!("/api/sites/{site_pid}/redirects"))
            .json(&json!({ "locale": "en", "from_path": "/x", "to_path": "/y", "status": 410 }))
            .await;
        assert_eq!(contradiction.status_code(), 422);
    })
    .await;
}

/// The sitemap is derived from what is published, escapes its XML, and
/// carries reciprocal `hreflang` alternates.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_sitemap_lists_only_published_indexable_pages() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_pid, site_key) = seed_public_site(&request, "sitemap").await;
        publish_at(&request, &site_pid, "listed", "/listed").await;

        // A drafted page is routed but never published.
        let draft: Value = request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .json(&json!({ "key": "draft", "content_type_key": "article", "title": "Draft" }))
            .await
            .json();
        request
            .put(&format!(
                "/api/entries/{}/variants/en/path",
                draft["pid"].as_str().unwrap()
            ))
            .json(&json!({ "path": "/draft" }))
            .await
            .assert_status_ok();

        // A published page marked noindex.
        let hidden: Value = request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .json(&json!({
                "key": "hidden", "content_type_key": "article", "title": "Hidden",
                "seo": { "robots": "noindex,follow" },
            }))
            .await
            .json();
        let hidden_pid = hidden["pid"].as_str().unwrap().to_string();
        request
            .put(&format!("/api/entries/{hidden_pid}/variants/en/path"))
            .json(&json!({ "path": "/hidden" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/entries/{hidden_pid}/variants/en/transition"))
            .json(&json!({ "action": "publish" }))
            .await
            .assert_status_ok();

        let response = request
            .get(&format!("/delivery/{site_key}/sitemap.xml"))
            .await;
        response.assert_status_ok();
        assert!(
            response
                .headers()
                .get("content-type")
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("application/xml")
        );
        let xml = response.text();
        assert!(xml.contains("https://example.test/en/listed"));
        assert!(!xml.contains("/draft"), "unpublished pages are not listed");
        assert!(!xml.contains("/hidden"), "noindex pages are not listed");
        assert!(xml.contains("<lastmod>"));
    })
    .await;
}

/// The feed carries what was published, newest first, and nothing a
/// reader has no business seeing (CMS-R19).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_feed_lists_published_pages_newest_first() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_pid, site_key) = seed_public_site(&request, "feed").await;
        publish_at(&request, &site_pid, "first-published", "/first").await;
        publish_at(&request, &site_pid, "second-published", "/second").await;

        // A drafted page: routed, never published.
        let draft: Value = request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .json(&json!({ "key": "unfinished", "content_type_key": "article", "title": "Draft" }))
            .await
            .json();
        request
            .put(&format!(
                "/api/entries/{}/variants/en/path",
                draft["pid"].as_str().unwrap()
            ))
            .json(&json!({ "path": "/unfinished" }))
            .await
            .assert_status_ok();

        let response = request
            .get(&format!("/delivery/{site_key}/en/feed.xml"))
            .await;
        response.assert_status_ok();
        assert!(
            response
                .headers()
                .get("content-type")
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("application/atom+xml")
        );
        let xml = response.text();
        assert!(xml.contains("http://www.w3.org/2005/Atom"));
        assert!(xml.contains("/en/first"));
        assert!(xml.contains("/en/second"));
        // A feed is a syndication surface; an unpublished page has not
        // asked to be syndicated.
        assert!(!xml.contains("/unfinished"), "drafts are not syndicated");
        // Newest first.
        let first = xml.find("/en/first").unwrap();
        let second = xml.find("/en/second").unwrap();
        assert!(second < first, "the most recently published comes first");
        // The identity is the entry, not the address, so a rename does
        // not resurface the page as a new item.
        assert!(xml.contains("<id>urn:uuid:"));
        assert!(xml.contains("<updated>"));
    })
    .await;
}

/// An empty feed is still a valid feed.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_empty_feed_is_still_a_valid_feed() {
    request::<App, _, _>(|request, _ctx| async move {
        let (_, empty_key) = seed_public_site(&request, "feed-empty").await;
        let empty = request
            .get(&format!("/delivery/{empty_key}/en/feed.xml"))
            .await;
        empty.assert_status_ok();
        let xml = empty.text();
        // Atom requires an `updated`; inventing "now" would make an
        // unchanged empty feed look fresh on every poll.
        assert!(xml.contains("<updated>"));
        assert!(!xml.contains("<entry>"));

        // A restricted site's feed **is** served here, and that is not
        // a bug in the feed: with `CMS_REQUIRE_AUTH` off — the shipped
        // default — there is no authentication and no authorization
        // anywhere, exactly as for every other delivery read. The
        // property that a restricted site does not syndicate is a
        // property of the *activated* deployment, so it is pinned in
        // the enforcement binary rather than asserted here where it
        // would be false.
        let restricted_key = a_key("feed-restricted");
        let created: Value = request
            .post("/api/sites")
            .json(&a_site_payload(&restricted_key))
            .await
            .json();
        assert!(created["pid"].is_string());
        assert_eq!(
            request
                .get(&format!("/delivery/{restricted_key}/en/feed.xml"))
                .await
                .status_code(),
            200,
            "with enforcement off every delivery read is open"
        );
    })
    .await;
}

/// `robots.txt` tells a crawler the truth about the site it is looking
/// at.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn robots_reflects_the_sites_visibility() {
    request::<App, _, _>(|request, _ctx| async move {
        let (_, public_key) = seed_public_site(&request, "robots-public").await;
        let robots = request
            .get(&format!("/delivery/{public_key}/robots.txt"))
            .await;
        robots.assert_status_ok();
        assert!(robots.text().contains("Allow: /"));
        assert!(robots.text().contains("sitemap.xml"));

        // A restricted site is not advertised at all.
        let key = a_key("robots-restricted");
        request
            .post("/api/sites")
            .json(&a_site_payload(&key))
            .await
            .assert_status_ok();
        let robots = request.get(&format!("/delivery/{key}/robots.txt")).await;
        assert!(robots.text().contains("Disallow: /"));
        assert!(!robots.text().contains("Sitemap:"));
    })
    .await;
}

/// Personalization reads only what the caller declares — and the
/// response varies by exactly what the rules consulted.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn audience_rules_report_matches_and_vary_the_cache_key() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_pid, site_key) = seed_public_site(&request, "audience").await;
        publish_at(&request, &site_pid, "home", "/home").await;

        // A rule nobody can spoof: it reads the channel the caller
        // declares, and nothing else.
        request
            .post(&format!("/api/sites/{site_pid}/audience-rules"))
            .json(&json!({
                "key": "kiosk", "name": "Lobby kiosk",
                "predicate": { "channel": ["screen"], "audience_tag": ["lobby"] },
            }))
            .await
            .assert_status_ok();

        // Rules that read anything else are refused at declaration.
        let refused = request
            .post(&format!("/api/sites/{site_pid}/audience-rules"))
            .json(&json!({
                "key": "tracking", "name": "By cookie",
                "predicate": { "cookie": ["returning"] },
            }))
            .await;
        assert_eq!(refused.status_code(), 422);
        assert!(refused.text().contains("no cookies"), "{}", refused.text());

        // A plain web request: the rule does not match, but the keys it
        // consulted are still reported so the response can vary by them.
        let plain: Value = request
            .get(&format!("/delivery/{site_key}/en/home"))
            .await
            .json();
        assert!(
            plain["personalization"]["matched_rules"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            plain["personalization"]["consulted_context"],
            json!(["audience_tag", "channel"])
        );

        // The kiosk gets its rule, and a different ETag — a
        // personalized response must not share a cache entry with the
        // unpersonalized one.
        let kiosk = request
            .get(&format!(
                "/delivery/{site_key}/en/home?channel=screen&audience_tag=lobby"
            ))
            .await;
        let kiosk_payload: Value = kiosk.json();
        assert_eq!(
            kiosk_payload["personalization"]["matched_rules"],
            json!(["kiosk"])
        );
        let plain_tag = request
            .get(&format!("/delivery/{site_key}/en/home"))
            .await
            .headers()
            .get("etag")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_ne!(
            kiosk.headers().get("etag").unwrap().to_str().unwrap(),
            plain_tag,
            "the tag mixes in the context the rules consulted"
        );
        assert!(kiosk.headers().get("vary").is_some());
    })
    .await;
}

/// A menu omits links into unpublished pages rather than serving a
/// navigation item that 404s.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_menu_omits_unpublished_targets() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_pid, site_key) = seed_public_site(&request, "menu").await;
        let live = publish_at(&request, &site_pid, "live", "/live").await;
        let draft: Value = request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .json(&json!({ "key": "coming", "content_type_key": "article", "title": "Coming" }))
            .await
            .json();

        request
            .post(&format!("/api/sites/{site_pid}/menus"))
            .json(&json!({
                "locale": "en", "key": "main",
                "items": [
                    { "label": "Live", "entry_pid": live },
                    { "label": "Coming", "entry_pid": draft["pid"] },
                    { "label": "External", "url": "https://elsewhere.test" },
                ],
            }))
            .await
            .assert_status_ok();

        let menu: Value = request
            .get(&format!("/delivery/{site_key}/en/menus/main"))
            .await
            .json();
        let labels: Vec<&str> = menu["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["label"].as_str().unwrap())
            .collect();
        assert_eq!(labels, vec!["Live", "External"]);
        assert_eq!(menu["items"][0]["path"], "/live");
    })
    .await;
}

/// Delivery composes references one hop deep, listing only renditions
/// that exist.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn delivery_expands_references_one_hop() {
    request::<App, _, _>(|request, _ctx| async move {
        let (site_pid, site_key) = seed_public_site(&request, "references").await;
        let target = publish_at(&request, &site_pid, "target", "/target").await;

        let referring: Value = request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .json(&json!({
                "key": "referring", "content_type_key": "article", "title": "Referring",
                "blocks": [{ "kind": "reference", "entry": target }],
            }))
            .await
            .json();
        let referring_pid = referring["pid"].as_str().unwrap().to_string();
        request
            .put(&format!("/api/entries/{referring_pid}/variants/en/path"))
            .json(&json!({ "path": "/referring" }))
            .await
            .assert_status_ok();
        request
            .post(&format!(
                "/api/entries/{referring_pid}/variants/en/transition"
            ))
            .json(&json!({ "action": "publish" }))
            .await
            .assert_status_ok();

        let payload: Value = request
            .get(&format!("/delivery/{site_key}/en/referring"))
            .await
            .json();
        let referenced = &payload["references"]["entries"][0];
        assert_eq!(referenced["key"], "target");
        assert_eq!(referenced["path"], "/target");
        assert_eq!(referenced["published"], true);
        assert!(
            referenced.get("blocks").is_none(),
            "a referenced entry is summarised, not expanded — one hop, not a graph walk"
        );
    })
    .await;
}
