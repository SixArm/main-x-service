//! The asset library (CMS-R6–R8): upload typing and dedupe, safe
//! delivery headers, metadata, declared renditions, replace, orphans,
//! the delete-refusal, and the alt-text publish gate.

use axum_test_loco_compat::multipart::{MultipartForm, Part};
use content_management_system_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_key, seed_site};

/// A minimal but genuine PNG header with the given dimensions — enough
/// for sniffing and header-read dimensions, which is exactly what the
/// service claims to do (it never decodes pixels).
fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend_from_slice(&[0, 0, 0, 13]);
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes
}

/// A multipart body carrying one file part plus optional text fields.
fn upload_form(
    bytes: Vec<u8>,
    filename: &str,
    mime: &str,
    fields: &[(&str, &str)],
) -> MultipartForm {
    let mut form = MultipartForm::new().add_part(
        "file",
        Part::bytes(bytes)
            .file_name(filename.to_string())
            .mime_type(mime),
    );
    for (name, value) in fields {
        form = form.add_text((*name).to_string(), (*value).to_string());
    }
    form
}

/// Declare a site + an article type with a media field.
async fn seed_site_and_type(request: &loco_rs::TestServer, prefix: &str) -> String {
    let site_pid = seed_site(request, &a_key(prefix)).await;
    request
        .post(&format!("/api/sites/{site_pid}/content-types"))
        .json(&json!({
            "key": "article",
            "name": "Article",
            "fields": [{ "key": "hero", "label": "Hero", "kind": "media" }],
        }))
        .await
        .assert_status_ok();
    site_pid
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn uploads_are_typed_addressed_and_deduplicated() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "assets").await;
        let path = format!("/api/sites/{site_pid}/assets");

        let uploaded: Value = request
            .post(&path)
            .multipart(upload_form(
                png(1200, 800),
                "hero.png",
                "image/png",
                &[("title", "A hero image"), ("tags", "hero, banner")],
            ))
            .await
            .json();
        assert_eq!(uploaded["kind"], "image");
        assert_eq!(uploaded["mime"], "image/png");
        assert_eq!(uploaded["width"], 1200);
        assert_eq!(uploaded["height"], 800);
        assert_eq!(uploaded["deduplicated"], false);
        let checksum = uploaded["checksum_sha256"].as_str().unwrap().to_string();
        assert_eq!(checksum.len(), 64, "a hex SHA-256");
        let first_pid = uploaded["pid"].as_str().unwrap().to_string();

        // The same bytes again: one stored object, and the existing
        // asset is returned rather than a second row splitting its
        // usage history in two.
        let again: Value = request
            .post(&path)
            .multipart(upload_form(png(1200, 800), "copy.png", "image/png", &[]))
            .await
            .json();
        assert_eq!(again["deduplicated"], true);
        assert_eq!(again["pid"], first_pid);

        // ...unless the caller explicitly asks for a separate record.
        let separate: Value = request
            .post(&path)
            .multipart(upload_form(
                png(1200, 800),
                "copy.png",
                "image/png",
                &[("on_duplicate", "new_record")],
            ))
            .await
            .json();
        assert_ne!(separate["pid"], first_pid);
        assert_eq!(
            separate["checksum_sha256"], checksum,
            "same content address"
        );
    })
    .await;
}

/// What a caller *says* a file is carries no weight.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn dangerous_and_mislabelled_uploads_are_refused() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "upload-refusals").await;
        let path = format!("/api/sites/{site_pid}/assets");

        // HTML wearing a .png name and an image/png declaration.
        let html = request
            .post(&path)
            .multipart(upload_form(
                b"<!DOCTYPE html><html><script>alert(1)</script></html>".to_vec(),
                "innocent.png",
                "image/png",
                &[],
            ))
            .await;
        assert_eq!(html.status_code(), 422);
        assert!(html.text().contains("text/html"), "{}", html.text());

        // SVG is refused *with the reason and an alternative*.
        let svg = request
            .post(&path)
            .multipart(upload_form(
                b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script/></svg>".to_vec(),
                "logo.svg",
                "image/svg+xml",
                &[],
            ))
            .await;
        assert_eq!(svg.status_code(), 422);
        assert!(svg.text().contains("PNG/WebP"), "{}", svg.text());

        // A real PNG mislabelled as a JPEG: the mismatch itself is the
        // signal, even though both types are acceptable.
        let mismatch = request
            .post(&path)
            .multipart(upload_form(png(2, 2), "hero.png", "image/jpeg", &[]))
            .await;
        assert_eq!(mismatch.status_code(), 422);
        assert!(
            mismatch.text().contains("does not match"),
            "{}",
            mismatch.text()
        );

        // An unrecognised format, and an empty part.
        let unknown = request
            .post(&path)
            .multipart(upload_form(
                vec![0x00, 0x01, 0x02, 0x03],
                "x.bin",
                "application/octet-stream",
                &[],
            ))
            .await;
        assert_eq!(unknown.status_code(), 422);
        assert!(unknown.text().contains("not recognised"));
    })
    .await;
}

/// Bytes leave with `nosniff` and a disposition, so a stored file
/// cannot be coaxed into being interpreted as something executable.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn stored_bytes_are_served_safely() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "delivery-headers").await;
        let bytes = png(4, 4);
        let uploaded: Value = request
            .post(&format!("/api/sites/{site_pid}/assets"))
            .multipart(upload_form(bytes.clone(), "hero.png", "image/png", &[]))
            .await
            .json();
        let pid = uploaded["pid"].as_str().unwrap();

        let response = request.get(&format!("/api/assets/{pid}/content")).await;
        response.assert_status_ok();
        assert_eq!(
            response.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );
        assert_eq!(response.headers().get("content-type").unwrap(), "image/png");
        let disposition = response.headers().get("content-disposition").unwrap();
        assert!(disposition.to_str().unwrap().starts_with("inline"));
        assert!(disposition.to_str().unwrap().contains("hero.png"));
        assert_eq!(response.as_bytes().to_vec(), bytes, "the bytes round-trip");

        // A PDF downloads instead of rendering: a viewer can execute
        // script, and this response must not be that page's origin.
        let pdf: Value = request
            .post(&format!("/api/sites/{site_pid}/assets"))
            .multipart(upload_form(
                b"%PDF-1.7\n1 0 obj\n".to_vec(),
                "report.pdf",
                "application/pdf",
                &[],
            ))
            .await
            .json();
        let response = request
            .get(&format!(
                "/api/assets/{}/content",
                pdf["pid"].as_str().unwrap()
            ))
            .await;
        assert!(
            response
                .headers()
                .get("content-disposition")
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("attachment")
        );
    })
    .await;
}

/// The accessibility gate, end to end: an image without alt text blocks
/// publication, and filling it in clears the blocker.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn missing_alt_text_blocks_publication_until_it_is_written() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "alt-text").await;
        let uploaded: Value = request
            .post(&format!("/api/sites/{site_pid}/assets"))
            .multipart(upload_form(png(10, 10), "hero.png", "image/png", &[]))
            .await
            .json();
        let asset_pid = uploaded["pid"].as_str().unwrap().to_string();

        let entry: Value = request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .json(&json!({
                "key": "illustrated", "content_type_key": "article", "title": "Illustrated",
                "blocks": [{ "kind": "image", "asset": asset_pid }],
            }))
            .await
            .json();
        let entry_pid = entry["pid"].as_str().unwrap().to_string();

        let check: Value = request
            .get(&format!(
                "/api/entries/{entry_pid}/variants/en/publish-check"
            ))
            .await
            .json();
        assert_eq!(check["ready"], false);
        // Look the rule up rather than assuming its position: the gate
        // reports every blocker, and this entry also has no address yet.
        let alt_blocker = check["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|blocker| blocker["rule"] == "image_alt_text_missing")
            .expect("the alt-text blocker is present");
        assert_eq!(alt_blocker["subject"], asset_pid);
        assert!(
            alt_blocker["remedy"]
                .as_str()
                .unwrap()
                .contains("screen reader"),
            "the blocker says why, not just no"
        );

        request
            .put(&format!("/api/assets/{asset_pid}"))
            .json(&json!({ "alt_text": "The town hall on a clear morning" }))
            .await
            .assert_status_ok();

        let check: Value = request
            .get(&format!(
                "/api/entries/{entry_pid}/variants/en/publish-check"
            ))
            .await
            .json();
        assert!(
            !check["blockers"]
                .as_array()
                .unwrap()
                .iter()
                .any(|blocker| blocker["rule"] == "image_alt_text_missing"),
            "writing the alt text clears that blocker"
        );
    })
    .await;
}

/// Replace keeps the asset's identity — and therefore every reference
/// to it — which is the operation "delete and re-upload" silently
/// botches.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn replace_preserves_identity_references_and_resets_renditions() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "replace").await;
        let uploaded: Value = request
            .post(&format!("/api/sites/{site_pid}/assets"))
            .multipart(upload_form(png(100, 100), "logo.png", "image/png", &[]))
            .await
            .json();
        let asset_pid = uploaded["pid"].as_str().unwrap().to_string();
        let first_checksum = uploaded["checksum_sha256"].as_str().unwrap().to_string();

        request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .json(&json!({
                "key": "uses-logo", "content_type_key": "article", "title": "Uses the logo",
                "fields": { "hero": asset_pid },
            }))
            .await
            .assert_status_ok();

        // Declare a rendition and mark it produced.
        let rendition: Value = request
            .post(&format!("/api/assets/{asset_pid}/renditions"))
            .json(&json!({ "key": "thumb", "width": 200, "height": 200, "format": "webp" }))
            .await
            .json();
        let rendition_pid = rendition["pid"].as_str().unwrap().to_string();
        request
            .put(&format!("/api/renditions/{rendition_pid}"))
            .json(&json!({ "state": "produced", "storage_ref": "file:///tmp/thumb.webp" }))
            .await
            .assert_status_ok();
        let detail: Value = request
            .get(&format!("/api/assets/{asset_pid}"))
            .await
            .json();
        assert_eq!(detail["available_renditions"][0], "thumb");

        // Replacing with a different kind would break every layout.
        let wrong_kind = request
            .post(&format!("/api/assets/{asset_pid}/replace"))
            .multipart(upload_form(
                b"%PDF-1.7\n".to_vec(),
                "logo.pdf",
                "application/pdf",
                &[],
            ))
            .await;
        assert_eq!(wrong_kind.status_code(), 422);
        assert!(
            wrong_kind
                .text()
                .contains("cannot replace a image with a document")
        );

        // A real replacement: same pid, new bytes.
        let replaced: Value = request
            .post(&format!("/api/assets/{asset_pid}/replace"))
            .multipart(upload_form(png(120, 120), "logo-v2.png", "image/png", &[]))
            .await
            .json();
        assert_eq!(replaced["pid"], asset_pid);
        assert_ne!(replaced["checksum_sha256"], first_checksum);
        assert_eq!(replaced["width"], 120);

        // The reference still points here...
        let usage: Value = request
            .get(&format!("/api/assets/{asset_pid}/usage"))
            .await
            .json();
        assert_eq!(usage["referrers"][0]["entry_key"], "uses-logo");

        // ...and the produced rendition went back to `declared`, because
        // it described bytes that no longer exist.
        let detail: Value = request
            .get(&format!("/api/assets/{asset_pid}"))
            .await
            .json();
        assert_eq!(detail["renditions"][0]["state"], "declared");
        assert!(
            detail["available_renditions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    })
    .await;
}

/// Deleting a used asset is refused; orphans are reported, never
/// removed on the system's own initiative.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn deletion_is_refused_while_used_and_orphans_are_only_reported() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "asset-delete").await;
        let used: Value = request
            .post(&format!("/api/sites/{site_pid}/assets"))
            .multipart(upload_form(png(10, 10), "used.png", "image/png", &[]))
            .await
            .json();
        let unused: Value = request
            .post(&format!("/api/sites/{site_pid}/assets"))
            .multipart(upload_form(png(20, 20), "unused.png", "image/png", &[]))
            .await
            .json();
        let used_pid = used["pid"].as_str().unwrap().to_string();
        let unused_pid = unused["pid"].as_str().unwrap().to_string();

        request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .json(&json!({
                "key": "uses-asset", "content_type_key": "article", "title": "Uses it",
                "fields": { "hero": used_pid },
            }))
            .await
            .assert_status_ok();

        let refused = request.delete(&format!("/api/assets/{used_pid}")).await;
        assert_eq!(refused.status_code(), 409);
        assert!(refused.text().contains("uses-asset"), "{}", refused.text());
        assert!(refused.text().contains("force=true"));

        // The orphan report names the unused one only, and says plainly
        // that it deleted nothing.
        let orphans: Value = request
            .get(&format!("/api/sites/{site_pid}/assets/orphans"))
            .await
            .json();
        assert_eq!(orphans["auto_deleted"], false);
        let pids: Vec<&str> = orphans["orphans"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["pid"].as_str().unwrap())
            .collect();
        assert!(pids.contains(&unused_pid.as_str()));
        assert!(!pids.contains(&used_pid.as_str()));
        assert!(orphans["bytes_reclaimable"].as_i64().unwrap() > 0);

        // The forced delete needs a reason and records what it broke.
        assert_eq!(
            request
                .delete(&format!("/api/assets/{used_pid}?force=true"))
                .await
                .status_code(),
            422
        );
        request
            .delete(&format!(
                "/api/assets/{used_pid}?force=true&reason=rights+expired"
            ))
            .await
            .assert_status_ok();
        let trail: Value = request.get(&format!("/api/audits/{used_pid}")).await.json();
        let forced = trail
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["action"] == "force_deleted")
            .expect("a forced delete is audited as such");
        assert_eq!(forced["snapshot"]["reason"], "rights expired");
        assert_eq!(
            forced["snapshot"]["broken_references"][0]["entry_key"],
            "uses-asset"
        );

        // The deleted asset now blocks publication of the page using it.
        let entries: Value = request
            .get(&format!("/api/sites/{site_pid}/entries"))
            .await
            .json();
        let entry_pid = entries[0]["pid"].as_str().unwrap();
        let check: Value = request
            .get(&format!(
                "/api/entries/{entry_pid}/variants/en/publish-check"
            ))
            .await
            .json();
        assert_eq!(check["ready"], false);
        assert!(
            check["blockers"]
                .as_array()
                .unwrap()
                .iter()
                .any(|blocker| blocker["rule"] == "reference_missing"),
            "the deleted asset now blocks publication: {}",
            check["blockers"]
        );
    })
    .await;
}

/// A rendition is a declaration until something produces bytes, and it
/// cannot claim otherwise.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn renditions_are_declarations_until_produced() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "renditions").await;
        let image: Value = request
            .post(&format!("/api/sites/{site_pid}/assets"))
            .multipart(upload_form(png(50, 50), "a.png", "image/png", &[]))
            .await
            .json();
        let asset_pid = image["pid"].as_str().unwrap().to_string();

        let declared: Value = request
            .post(&format!("/api/assets/{asset_pid}/renditions"))
            .json(&json!({ "key": "wide", "width": 1600, "format": "webp" }))
            .await
            .json();
        assert_eq!(declared["state"], "declared");
        assert!(declared["storage_ref"].is_null());

        // Declared, so not yet available to a channel.
        let detail: Value = request
            .get(&format!("/api/assets/{asset_pid}"))
            .await
            .json();
        assert!(
            detail["available_renditions"]
                .as_array()
                .unwrap()
                .is_empty()
        );

        // The same key twice is a conflict.
        assert_eq!(
            request
                .post(&format!("/api/assets/{asset_pid}/renditions"))
                .json(&json!({ "key": "wide", "format": "webp" }))
                .await
                .status_code(),
            409
        );

        // "Produced" without bytes would put a 404 in a delivery payload.
        let dishonest = request
            .put(&format!(
                "/api/renditions/{}",
                declared["pid"].as_str().unwrap()
            ))
            .json(&json!({ "state": "produced" }))
            .await;
        assert_eq!(dishonest.status_code(), 422);
        assert!(dishonest.text().contains("needs a storage_ref"));

        // A document has no renditions to declare.
        let pdf: Value = request
            .post(&format!("/api/sites/{site_pid}/assets"))
            .multipart(upload_form(
                b"%PDF-1.7\n".to_vec(),
                "d.pdf",
                "application/pdf",
                &[],
            ))
            .await
            .json();
        let refused = request
            .post(&format!(
                "/api/assets/{}/renditions",
                pdf["pid"].as_str().unwrap()
            ))
            .json(&json!({ "key": "thumb", "format": "webp" }))
            .await;
        assert_eq!(refused.status_code(), 422);
        assert!(refused.text().contains("image concern"));
    })
    .await;
}
