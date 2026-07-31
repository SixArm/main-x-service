//! The synthetic content corpus (CMS-T24) — entries, variants,
//! revisions, assets, routes, redirects, menus, and the audit trail
//! that the throughput view reads.
//!
//! ## What this is for
//!
//! A demo whose insights views are empty teaches nothing, and one whose
//! findings appear by accident teaches the wrong thing. So this fixture
//! **plants one instance of every health rule on purpose**, each on an
//! entry whose key says which rule it demonstrates
//! (`plant-stale-content`, `plant-no-alt-text`, …). If a rule stops
//! firing after a refactor, the seeded site says so.
//!
//! ## Synthetic only
//!
//! No real copy, no real imagery, no real people (spec
//! `regulatory.md`). Author references are synthetic `worker:` URNs
//! with fixed UUIDs so a rerun produces the same actors. Asset bytes
//! are **not** written to the artifact store: the rows describe files
//! that were never uploaded, which is honest for a fixture and avoids
//! shipping binary content in a repository. A seeded asset therefore
//! serves metadata but not content — the one place the demo diverges
//! from a real upload, and it is why the seed logs a note saying so.
//!
//! ## Backdating
//!
//! Rows are inserted with explicit `created_at`/`updated_at` rather
//! than letting the database default them, because half the health
//! rules and every duration percentile are *about* elapsed time. A
//! fixture whose rows are all a second old cannot demonstrate a stale
//! page or a stuck review.

use loco_rs::prelude::*;
use sea_orm::ActiveValue;
use uuid::Uuid;

use crate::models::_entities::{
    assets, audit_logs, content_references, content_types, entries, entry_variants, menus,
    redirects, renditions, revisions, routes,
};

/// A synthetic time, `days` ago.
fn days_ago(days: i64) -> chrono::DateTime<chrono::FixedOffset> {
    (chrono::Utc::now() - chrono::Duration::days(days)).into()
}

/// A synthetic time, `hours` ago.
fn hours_ago(hours: i64) -> chrono::DateTime<chrono::FixedOffset> {
    (chrono::Utc::now() - chrono::Duration::hours(hours)).into()
}

/// Fixed synthetic actors, so a rerun produces the same trail.
const AUTHORS: [&str; 3] = [
    "worker:11111111-1111-4111-8111-111111111111",
    "worker:22222222-2222-4222-8222-222222222222",
    "worker:33333333-3333-4333-8333-333333333333",
];
/// The synthetic editor who approves and publishes.
const EDITOR: &str = "worker:44444444-4444-4444-8444-444444444444";

/// What one seeded revision says.
struct Rev {
    number: i32,
    title: String,
    blocks: serde_json::Value,
    fields: serde_json::Value,
    seo: serde_json::Value,
    schema_version: i32,
    author: &'static str,
    created_at: chrono::DateTime<chrono::FixedOffset>,
}

impl Rev {
    /// A healthy revision: real SEO, a body, today's schema.
    fn healthy(number: i32, title: &str, days: i64, author: &'static str) -> Self {
        Self {
            number,
            title: title.to_string(),
            blocks: serde_json::json!([
                { "kind": "heading", "level": 2, "text": title },
                { "kind": "paragraph", "text": "Synthetic body copy for the demo corpus." },
            ]),
            fields: serde_json::json!({ "section": "guide", "summary": "A synthetic summary." }),
            seo: serde_json::json!({
                "meta_title": title,
                "meta_description": "A synthetic description for the demo corpus.",
                "robots": "index,follow",
            }),
            schema_version: 2,
            author,
            created_at: days_ago(days),
        }
    }
}

/// Insert one revision.
async fn add_revision(db: &DatabaseConnection, variant_pid: Uuid, rev: Rev) -> Result<Uuid> {
    let pid = Uuid::new_v4();
    revisions::ActiveModel {
        pid: ActiveValue::set(pid),
        variant_pid: ActiveValue::set(variant_pid),
        number: ActiveValue::set(rev.number),
        title: ActiveValue::set(rev.title),
        blocks: ActiveValue::set(rev.blocks),
        fields: ActiveValue::set(rev.fields),
        seo: ActiveValue::set(rev.seo),
        type_schema_version: ActiveValue::set(rev.schema_version),
        author_ref: ActiveValue::set(Some(rev.author.to_string())),
        note: ActiveValue::set(None),
        restored_from_pid: ActiveValue::set(None),
        created_at: ActiveValue::set(rev.created_at),
        updated_at: ActiveValue::set(rev.created_at),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(pid)
}

/// How a seeded variant should look.
///
/// The `pid` is supplied by the caller, not minted here: a variant's
/// revisions have to name it before it exists, so the caller mints it
/// first. Letting this function generate its own left every route and
/// reference pointing at a variant that was never inserted — which is
/// exactly what happened, and what the orphan count caught.
struct VariantSpec<'a> {
    pid: Uuid,
    entry_pid: Uuid,
    locale: &'a str,
    status: &'a str,
    /// Set when the variant is published (or was, before archiving).
    published: Option<Uuid>,
    current: Uuid,
    /// The source revision a translation was made from.
    translation_of: Option<Uuid>,
    /// Where this variant sits in the translation workflow. Anything
    /// other than `translated` counts as an open request in the
    /// backlog view, so finished translations must say so.
    translation_status: Option<&'a str>,
    /// A future publish time, for the scheduled-work backlog.
    scheduled_publish_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    updated_at: chrono::DateTime<chrono::FixedOffset>,
}

/// Insert one variant.
async fn add_variant(db: &DatabaseConnection, spec: VariantSpec<'_>) -> Result<Uuid> {
    let pid = spec.pid;
    let published_at = spec.published.map(|_| spec.updated_at);
    entry_variants::ActiveModel {
        pid: ActiveValue::set(pid),
        entry_pid: ActiveValue::set(spec.entry_pid),
        locale: ActiveValue::set(spec.locale.to_string()),
        status: ActiveValue::set(spec.status.to_string()),
        current_revision_pid: ActiveValue::set(Some(spec.current)),
        published_revision_pid: ActiveValue::set(spec.published),
        translation_of_revision_pid: ActiveValue::set(spec.translation_of),
        reviewer_ref: ActiveValue::set(
            (spec.status == "in_review" || spec.status == "approved").then(|| EDITOR.to_string()),
        ),
        published_at: ActiveValue::set(published_at),
        first_published_at: ActiveValue::set(published_at),
        translation_status: ActiveValue::set(spec.translation_status.map(ToString::to_string)),
        translation_requested_at: ActiveValue::set(
            spec.translation_status
                .filter(|status| *status != "translated")
                .map(|_| spec.updated_at),
        ),
        translation_requested_by: ActiveValue::set(
            spec.translation_status
                .filter(|status| *status != "translated")
                .map(|_| EDITOR.to_string()),
        ),
        scheduled_publish_at: ActiveValue::set(spec.scheduled_publish_at),
        created_at: ActiveValue::set(spec.updated_at),
        updated_at: ActiveValue::set(spec.updated_at),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(pid)
}

/// Insert one entry.
async fn add_entry(
    db: &DatabaseConnection,
    site_pid: Uuid,
    key: &str,
    type_key: &str,
    owner: &'static str,
    created_days_ago: i64,
) -> Result<Uuid> {
    let pid = Uuid::new_v4();
    entries::ActiveModel {
        pid: ActiveValue::set(pid),
        site_pid: ActiveValue::set(site_pid),
        content_type_key: ActiveValue::set(type_key.to_string()),
        type_schema_version: ActiveValue::set(if type_key == "article" { 2 } else { 1 }),
        key: ActiveValue::set(key.to_string()),
        source_locale: ActiveValue::set("en".to_string()),
        owner_ref: ActiveValue::set(Some(owner.to_string())),
        archived_at: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        created_at: ActiveValue::set(days_ago(created_days_ago)),
        updated_at: ActiveValue::set(days_ago(created_days_ago)),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(pid)
}

/// Record a transition in the audit trail, backdated.
///
/// The throughput view is derived entirely from these rows, so a
/// corpus without them would show an empty dashboard next to forty
/// entries — the most misleading possible demo.
async fn add_transition(
    db: &DatabaseConnection,
    variant_pid: Uuid,
    action: &str,
    actor: &str,
    at: chrono::DateTime<chrono::FixedOffset>,
) -> Result<()> {
    audit_logs::ActiveModel {
        entity: ActiveValue::set("variant".to_string()),
        entity_pid: ActiveValue::set(variant_pid),
        action: ActiveValue::set(action.to_string()),
        actor: ActiveValue::set(Some(actor.to_string())),
        snapshot: ActiveValue::set(Some(serde_json::json!({ "seeded": true }))),
        created_at: ActiveValue::set(at),
        updated_at: ActiveValue::set(at),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

/// The usual draft → review → approve → publish trail, with plausible
/// gaps so the duration percentiles have a real spread.
async fn add_full_trail(
    db: &DatabaseConnection,
    variant_pid: Uuid,
    days: i64,
    spread_hours: i64,
) -> Result<()> {
    let created = days_ago(days);
    add_transition(db, variant_pid, "created", AUTHORS[0], created).await?;
    add_transition(
        db,
        variant_pid,
        "submit",
        AUTHORS[0],
        created + chrono::Duration::hours(spread_hours),
    )
    .await?;
    add_transition(
        db,
        variant_pid,
        "approve",
        EDITOR,
        created + chrono::Duration::hours(spread_hours * 2),
    )
    .await?;
    add_transition(
        db,
        variant_pid,
        "publish",
        EDITOR,
        created + chrono::Duration::hours(spread_hours * 2 + 1),
    )
    .await?;
    Ok(())
}

/// Insert one asset. Bytes are not written — see the module note.
#[allow(clippy::too_many_arguments)] // a fixture row, listed field by field
async fn add_asset(
    db: &DatabaseConnection,
    site_pid: Uuid,
    index: usize,
    kind: &str,
    mime: &str,
    title: &str,
    alt_text: Option<&str>,
    byte_size: i64,
) -> Result<Uuid> {
    let pid = Uuid::new_v4();
    // A deterministic stand-in checksum: distinct per asset (so the
    // dedupe path is not accidentally exercised) and obviously
    // synthetic to anyone reading the table.
    let checksum = format!("{:0>64}", format!("seed{index:04}"));
    assets::ActiveModel {
        pid: ActiveValue::set(pid),
        site_pid: ActiveValue::set(Some(site_pid)),
        kind: ActiveValue::set(kind.to_string()),
        mime: ActiveValue::set(mime.to_string()),
        byte_size: ActiveValue::set(byte_size),
        checksum_sha256: ActiveValue::set(checksum.clone()),
        storage_ref: ActiveValue::set(format!("seed://{checksum}")),
        original_filename: ActiveValue::set(Some(format!("seed-{index:02}"))),
        title: ActiveValue::set(Some(title.to_string())),
        alt_text: ActiveValue::set(alt_text.map(ToString::to_string)),
        caption: ActiveValue::set(None),
        credit: ActiveValue::set(Some("Synthetic fixture".to_string())),
        licence: ActiveValue::set(Some("CC0-1.0".to_string())),
        tags: ActiveValue::set(serde_json::json!(["seed"])),
        width: ActiveValue::set((kind == "image").then_some(1600)),
        height: ActiveValue::set((kind == "image").then_some(900)),
        duration_ms: ActiveValue::set((kind == "video").then_some(42_000)),
        uploaded_by_ref: ActiveValue::set(Some(AUTHORS[index % AUTHORS.len()].to_string())),
        deleted_at: ActiveValue::set(None),
        created_at: ActiveValue::set(days_ago(120 - i64::try_from(index).unwrap_or(0))),
        updated_at: ActiveValue::set(days_ago(120 - i64::try_from(index).unwrap_or(0))),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(pid)
}

/// Point a revision at something.
async fn add_reference(
    db: &DatabaseConnection,
    revision_pid: Uuid,
    variant_pid: Uuid,
    kind: &str,
    field_key: &str,
    to_asset: Option<Uuid>,
    to_entry: Option<Uuid>,
) -> Result<()> {
    content_references::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        from_revision_pid: ActiveValue::set(revision_pid),
        from_variant_pid: ActiveValue::set(variant_pid),
        kind: ActiveValue::set(kind.to_string()),
        to_entry_pid: ActiveValue::set(to_entry),
        to_asset_pid: ActiveValue::set(to_asset),
        to_entity_ref: ActiveValue::set(None),
        field_key: ActiveValue::set(field_key.to_string()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

/// Give a published variant its address.
async fn add_route(
    db: &DatabaseConnection,
    site_pid: Uuid,
    locale: &str,
    path: &str,
    variant_pid: Uuid,
) -> Result<()> {
    routes::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        site_pid: ActiveValue::set(site_pid),
        locale: ActiveValue::set(locale.to_string()),
        path: ActiveValue::set(path.to_string()),
        variant_pid: ActiveValue::set(variant_pid),
        is_current: ActiveValue::set(true),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

/// What the corpus planted, for the task's closing log line.
pub(super) struct Planted {
    /// Entries created.
    pub entries: usize,
    /// Variants created across every locale.
    pub variants: usize,
    /// Assets created.
    pub assets: usize,
    /// Assets deliberately left unreferenced.
    pub orphans: usize,
}

/// Build the corpus.
///
/// # Errors
///
/// When any insert fails.
#[allow(clippy::too_many_lines)] // one fixture, read top to bottom
pub(super) async fn plant(db: &DatabaseConnection, site_pid: Uuid) -> Result<Planted> {
    // ---- assets -------------------------------------------------
    //
    // Twenty-five, of which six are left unreferenced so the orphan
    // rule has something to find, and one image has no alt text so the
    // publish gate and the health rule both have a subject.
    let mut assets_all = Vec::new();
    for index in 0..18 {
        assets_all.push(
            add_asset(
                db,
                site_pid,
                index,
                "image",
                "image/png",
                &format!("Synthetic image {index:02}"),
                Some("A synthetic placeholder image."),
                40_000 + i64::try_from(index).unwrap_or(0) * 1_000,
            )
            .await?,
        );
    }
    let no_alt_text = add_asset(
        db,
        site_pid,
        18,
        "image",
        "image/png",
        "Synthetic image without alt text",
        None,
        61_000,
    )
    .await?;
    assets_all.push(no_alt_text);
    for (offset, (kind, mime, title, size)) in [
        ("document", "application/pdf", "Synthetic handbook", 900_000),
        ("document", "application/pdf", "Synthetic policy", 240_000),
        ("document", "application/pdf", "Synthetic report", 310_000),
        ("video", "video/mp4", "Synthetic clip", 8_400_000),
        ("video", "video/mp4", "Synthetic interview", 12_900_000),
        ("audio", "audio/mpeg", "Synthetic podcast", 5_100_000),
    ]
    .into_iter()
    .enumerate()
    {
        assets_all.push(add_asset(db, site_pid, 19 + offset, kind, mime, title, None, size).await?);
    }

    // A couple of declared renditions, so the asset views are not empty.
    for (key, width, state) in [("thumb", 320, "ready"), ("hero", 1600, "declared")] {
        renditions::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            asset_pid: ActiveValue::set(assets_all[0]),
            key: ActiveValue::set(key.to_string()),
            width: ActiveValue::set(Some(width)),
            height: ActiveValue::set(Some(width * 9 / 16)),
            format: ActiveValue::set("image/webp".to_string()),
            storage_ref: ActiveValue::set(None),
            state: ActiveValue::set(state.to_string()),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    let mut entry_count = 0usize;
    let mut variant_count = 0usize;
    // Assets referenced by at least one revision; the rest are orphans.
    let mut referenced = std::collections::BTreeSet::new();

    // ---- the ordinary corpus ------------------------------------
    //
    // Twelve healthy entries: published in English, translated into
    // French, some also into Canadian French. These are what the demo
    // looks like when nothing is wrong.
    let ordinary = [
        ("guide-getting-started", "page", "Getting started"),
        ("guide-accounts", "page", "Managing accounts"),
        ("guide-permissions", "page", "Permissions"),
        ("guide-integrations", "page", "Integrations"),
        ("news-spring-release", "article", "Spring release"),
        ("news-summer-release", "article", "Summer release"),
        ("news-autumn-release", "article", "Autumn release"),
        ("news-field-notes", "article", "Field notes"),
        ("news-service-update", "article", "Service update"),
        ("news-roadmap", "article", "Roadmap"),
        ("course-intro", "course-listing", "Introductory course"),
        ("course-advanced", "course-listing", "Advanced course"),
    ];
    for (index, (key, type_key, title)) in ordinary.into_iter().enumerate() {
        let age = 30 + i64::try_from(index).unwrap_or(0) * 5;
        let author = AUTHORS[index % AUTHORS.len()];
        let entry_pid = add_entry(db, site_pid, key, type_key, author, age).await?;
        entry_count += 1;

        // English source: two revisions, the second published — so the
        // revision history, the diff view, and restore have something
        // to work with on every ordinary entry.
        let english = Uuid::new_v4();
        add_revision(db, english, Rev::healthy(1, title, age, author)).await?;
        let second = add_revision(db, english, Rev::healthy(2, title, age - 2, author)).await?;
        // The variant pid was minted first so its revisions could name
        // it; insert it now that they exist.
        entry_variants::ActiveModel {
            pid: ActiveValue::set(english),
            entry_pid: ActiveValue::set(entry_pid),
            locale: ActiveValue::set("en".to_string()),
            status: ActiveValue::set("published".to_string()),
            current_revision_pid: ActiveValue::set(Some(second)),
            published_revision_pid: ActiveValue::set(Some(second)),
            published_at: ActiveValue::set(Some(days_ago(age - 2))),
            first_published_at: ActiveValue::set(Some(days_ago(age - 2))),
            created_at: ActiveValue::set(days_ago(age)),
            updated_at: ActiveValue::set(days_ago(age - 2)),
            ..Default::default()
        }
        .insert(db)
        .await?;
        variant_count += 1;
        add_full_trail(db, english, age, 6 + i64::try_from(index).unwrap_or(0)).await?;
        add_route(db, site_pid, "en", &format!("/{key}"), english).await?;

        // Every published English revision shows an image with alt
        // text — the healthy case the planted one contrasts with.
        let asset = assets_all[index % 18];
        referenced.insert(asset);
        add_reference(
            db,
            second,
            english,
            "asset",
            "hero_image",
            Some(asset),
            None,
        )
        .await?;
        // Three of the entries also link a document, so the library is
        // not split cleanly into "images, used" and "everything else,
        // orphaned" — which would make the orphan rule look like it
        // was really a media-type rule.
        if index < 3 {
            let document = assets_all[19 + index];
            referenced.insert(document);
            add_reference(
                db,
                second,
                english,
                "asset",
                "attachment",
                Some(document),
                None,
            )
            .await?;
        }

        // French translation of the *published* revision, so it is
        // current rather than stale.
        let french = Uuid::new_v4();
        let french_rev =
            add_revision(db, french, Rev::healthy(1, title, age - 1, AUTHORS[1])).await?;
        add_variant(
            db,
            VariantSpec {
                pid: french,
                entry_pid,
                locale: "fr",
                status: "published",
                published: Some(french_rev),
                current: french_rev,
                translation_of: Some(second),
                translation_status: Some("translated"),
                scheduled_publish_at: None,
                updated_at: days_ago(age - 1),
            },
        )
        .await?;
        variant_count += 1;
        add_route(db, site_pid, "fr", &format!("/fr/{key}"), french).await?;
        // The translations use the rest of the image library, so an
        // orphan is genuinely unused rather than merely unused *in
        // English*.
        let french_asset = assets_all[(index + 12) % 18];
        referenced.insert(french_asset);
        add_reference(
            db,
            french_rev,
            french,
            "asset",
            "hero_image",
            Some(french_asset),
            None,
        )
        .await?;

        // Half also carry Canadian French.
        if index % 2 == 0 {
            let quebecois = Uuid::new_v4();
            let rev =
                add_revision(db, quebecois, Rev::healthy(1, title, age - 1, AUTHORS[2])).await?;
            add_variant(
                db,
                VariantSpec {
                    pid: quebecois,
                    entry_pid,
                    locale: "fr-CA",
                    status: "published",
                    published: Some(rev),
                    current: rev,
                    translation_of: Some(second),
                    translation_status: Some("translated"),
                    scheduled_publish_at: None,
                    updated_at: days_ago(age - 1),
                },
            )
            .await?;
            variant_count += 1;
        }
    }

    // ---- work in progress ----------------------------------------
    //
    // Draft and archived entries, so the workflow views show more than
    // a wall of "published".
    for (key, status) in [
        ("draft-pricing-refresh", "draft"),
        ("draft-support-hours", "draft"),
        ("draft-status-page", "draft"),
        ("archive-old-announcement", "archived"),
        ("archive-retired-guide", "archived"),
    ] {
        let entry_pid = add_entry(db, site_pid, key, "page", AUTHORS[0], 90).await?;
        entry_count += 1;
        let variant = Uuid::new_v4();
        let rev = add_revision(db, variant, Rev::healthy(1, key, 88, AUTHORS[0])).await?;
        add_variant(
            db,
            VariantSpec {
                pid: variant,
                entry_pid,
                locale: "en",
                status,
                published: None,
                current: rev,
                translation_of: None,
                translation_status: None,
                scheduled_publish_at: None,
                updated_at: days_ago(80),
            },
        )
        .await?;
        variant_count += 1;
        add_transition(db, variant, "created", AUTHORS[0], days_ago(90)).await?;
        if status == "archived" {
            add_transition(db, variant, "archive", EDITOR, days_ago(80)).await?;
        }
    }

    // A rejected variant, so the rejection rate is not a flat zero.
    {
        let entry_pid =
            add_entry(db, site_pid, "draft-rejected-draft", "page", AUTHORS[1], 20).await?;
        entry_count += 1;
        let variant = Uuid::new_v4();
        let rev = add_revision(
            db,
            variant,
            Rev::healthy(1, "Rejected draft", 20, AUTHORS[1]),
        )
        .await?;
        add_variant(
            db,
            VariantSpec {
                pid: variant,
                entry_pid,
                locale: "en",
                status: "draft",
                published: None,
                current: rev,
                translation_of: None,
                translation_status: None,
                scheduled_publish_at: None,
                updated_at: days_ago(18),
            },
        )
        .await?;
        variant_count += 1;
        add_transition(db, variant, "created", AUTHORS[1], days_ago(20)).await?;
        add_transition(db, variant, "submit", AUTHORS[1], days_ago(19)).await?;
        add_transition(db, variant, "reject", EDITOR, days_ago(18)).await?;
    }

    // ---- one planted instance of every health rule ---------------
    //
    // Each entry key names the rule it demonstrates, so a reader of the
    // health view can trace a finding back to the fixture that caused
    // it — and a rule that stops firing is visible immediately.

    // 1. `stuck_in_review` — in review far longer than the window.
    {
        let entry_pid = add_entry(
            db,
            site_pid,
            "plant-stuck-in-review",
            "page",
            AUTHORS[0],
            60,
        )
        .await?;
        entry_count += 1;
        let variant = Uuid::new_v4();
        let rev = add_revision(
            db,
            variant,
            Rev::healthy(1, "Stuck in review", 60, AUTHORS[0]),
        )
        .await?;
        add_variant(
            db,
            VariantSpec {
                pid: variant,
                entry_pid,
                locale: "en",
                status: "in_review",
                published: None,
                current: rev,
                translation_of: None,
                translation_status: None,
                scheduled_publish_at: None,
                updated_at: days_ago(45),
            },
        )
        .await?;
        variant_count += 1;
        add_transition(db, variant, "created", AUTHORS[0], days_ago(60)).await?;
        add_transition(db, variant, "submit", AUTHORS[0], days_ago(45)).await?;
    }

    // 2. `approved_not_published` — approved, then forgotten.
    {
        let entry_pid = add_entry(
            db,
            site_pid,
            "plant-approved-not-published",
            "page",
            AUTHORS[1],
            40,
        )
        .await?;
        entry_count += 1;
        let variant = Uuid::new_v4();
        let rev = add_revision(
            db,
            variant,
            Rev::healthy(1, "Approved, not published", 40, AUTHORS[1]),
        )
        .await?;
        add_variant(
            db,
            VariantSpec {
                pid: variant,
                entry_pid,
                locale: "en",
                status: "approved",
                published: None,
                current: rev,
                translation_of: None,
                translation_status: None,
                scheduled_publish_at: None,
                updated_at: days_ago(30),
            },
        )
        .await?;
        variant_count += 1;
        add_transition(db, variant, "created", AUTHORS[1], days_ago(40)).await?;
        add_transition(db, variant, "submit", AUTHORS[1], days_ago(35)).await?;
        add_transition(db, variant, "approve", EDITOR, days_ago(30)).await?;
    }

    // 3. `stale_content` — published, then never revised. The window
    //    defaults to a year, so this one is deliberately older.
    {
        let entry_pid =
            add_entry(db, site_pid, "plant-stale-content", "page", AUTHORS[2], 500).await?;
        entry_count += 1;
        let variant = Uuid::new_v4();
        let rev = add_revision(db, variant, Rev::healthy(1, "Stale page", 500, AUTHORS[2])).await?;
        add_variant(
            db,
            VariantSpec {
                pid: variant,
                entry_pid,
                locale: "en",
                status: "published",
                published: Some(rev),
                current: rev,
                translation_of: None,
                translation_status: None,
                scheduled_publish_at: None,
                updated_at: days_ago(499),
            },
        )
        .await?;
        variant_count += 1;
        add_route(db, site_pid, "en", "/plant-stale-content", variant).await?;
    }

    // 4. `seo_metadata_missing` — indexable, but nothing to index with.
    {
        let entry_pid = add_entry(db, site_pid, "plant-no-seo", "page", AUTHORS[0], 25).await?;
        entry_count += 1;
        let variant = Uuid::new_v4();
        let rev = add_revision(
            db,
            variant,
            Rev {
                seo: serde_json::json!({ "robots": "index,follow" }),
                ..Rev::healthy(1, "No SEO metadata", 25, AUTHORS[0])
            },
        )
        .await?;
        add_variant(
            db,
            VariantSpec {
                pid: variant,
                entry_pid,
                locale: "en",
                status: "published",
                published: Some(rev),
                current: rev,
                translation_of: None,
                translation_status: None,
                scheduled_publish_at: None,
                updated_at: days_ago(24),
            },
        )
        .await?;
        variant_count += 1;
        add_route(db, site_pid, "en", "/plant-no-seo", variant).await?;
    }

    // 5. `image_alt_text_missing` — a published page showing the one
    //    image nobody described.
    {
        let entry_pid =
            add_entry(db, site_pid, "plant-no-alt-text", "page", AUTHORS[1], 22).await?;
        entry_count += 1;
        let variant = Uuid::new_v4();
        let rev = add_revision(
            db,
            variant,
            Rev::healthy(1, "Undescribed image", 22, AUTHORS[1]),
        )
        .await?;
        add_variant(
            db,
            VariantSpec {
                pid: variant,
                entry_pid,
                locale: "en",
                status: "published",
                published: Some(rev),
                current: rev,
                translation_of: None,
                translation_status: None,
                scheduled_publish_at: None,
                updated_at: days_ago(21),
            },
        )
        .await?;
        variant_count += 1;
        referenced.insert(no_alt_text);
        add_reference(
            db,
            rev,
            variant,
            "asset",
            "hero_image",
            Some(no_alt_text),
            None,
        )
        .await?;
        add_route(db, site_pid, "en", "/plant-no-alt-text", variant).await?;
    }

    // 6. `broken_reference` — pointing at an entry and an asset that
    //    are not there. Both halves of the rule, one subject.
    {
        let entry_pid = add_entry(
            db,
            site_pid,
            "plant-broken-reference",
            "page",
            AUTHORS[2],
            18,
        )
        .await?;
        entry_count += 1;
        let variant = Uuid::new_v4();
        let rev = add_revision(
            db,
            variant,
            Rev::healthy(1, "Broken references", 18, AUTHORS[2]),
        )
        .await?;
        add_variant(
            db,
            VariantSpec {
                pid: variant,
                entry_pid,
                locale: "en",
                status: "published",
                published: Some(rev),
                current: rev,
                translation_of: None,
                translation_status: None,
                scheduled_publish_at: None,
                updated_at: days_ago(17),
            },
        )
        .await?;
        variant_count += 1;
        add_reference(
            db,
            rev,
            variant,
            "entry",
            "related",
            None,
            Some(Uuid::new_v4()),
        )
        .await?;
        add_reference(
            db,
            rev,
            variant,
            "asset",
            "hero_image",
            Some(Uuid::new_v4()),
            None,
        )
        .await?;
        add_route(db, site_pid, "en", "/plant-broken-reference", variant).await?;
    }

    // 7. `needs_migration` — written under v1 of the article schema,
    //    which today's v2 (with its required `summary`) would reject.
    {
        let entry_pid = add_entry(
            db,
            site_pid,
            "plant-needs-migration",
            "article",
            AUTHORS[0],
            300,
        )
        .await?;
        entry_count += 1;
        let variant = Uuid::new_v4();
        let rev = add_revision(
            db,
            variant,
            Rev {
                schema_version: 1,
                fields: serde_json::json!({ "section": "news" }),
                ..Rev::healthy(1, "Written under the old schema", 300, AUTHORS[0])
            },
        )
        .await?;
        add_variant(
            db,
            VariantSpec {
                pid: variant,
                entry_pid,
                locale: "en",
                status: "published",
                published: Some(rev),
                current: rev,
                translation_of: None,
                translation_status: None,
                scheduled_publish_at: None,
                updated_at: days_ago(299),
            },
        )
        .await?;
        variant_count += 1;
        add_route(db, site_pid, "en", "/plant-needs-migration", variant).await?;
    }

    // 8. `stale_translation` — two of them, as the spec asks. The
    //    English source moved on twice; the translations did not.
    for (key, locale) in [
        ("plant-stale-translation-fr", "fr"),
        ("plant-stale-translation-fr-ca", "fr-CA"),
    ] {
        let entry_pid = add_entry(db, site_pid, key, "page", AUTHORS[1], 70).await?;
        entry_count += 1;
        let english = Uuid::new_v4();
        let first = add_revision(db, english, Rev::healthy(1, "Source", 70, AUTHORS[1])).await?;
        add_revision(db, english, Rev::healthy(2, "Source", 40, AUTHORS[1])).await?;
        let third = add_revision(db, english, Rev::healthy(3, "Source", 10, AUTHORS[1])).await?;
        entry_variants::ActiveModel {
            pid: ActiveValue::set(english),
            entry_pid: ActiveValue::set(entry_pid),
            locale: ActiveValue::set("en".to_string()),
            status: ActiveValue::set("published".to_string()),
            current_revision_pid: ActiveValue::set(Some(third)),
            published_revision_pid: ActiveValue::set(Some(third)),
            published_at: ActiveValue::set(Some(days_ago(10))),
            first_published_at: ActiveValue::set(Some(days_ago(70))),
            created_at: ActiveValue::set(days_ago(70)),
            updated_at: ActiveValue::set(days_ago(10)),
            ..Default::default()
        }
        .insert(db)
        .await?;
        variant_count += 1;

        // The translation still points at revision 1.
        let translated = Uuid::new_v4();
        let rev = add_revision(
            db,
            translated,
            Rev::healthy(1, "Traduction", 69, AUTHORS[2]),
        )
        .await?;
        add_variant(
            db,
            VariantSpec {
                pid: translated,
                entry_pid,
                locale,
                status: "published",
                published: Some(rev),
                current: rev,
                translation_of: Some(first),
                translation_status: Some("translated"),
                scheduled_publish_at: None,
                updated_at: days_ago(69),
            },
        )
        .await?;
        variant_count += 1;
    }

    // Work that is queued rather than wrong: an open translation
    // request and a scheduled publish, so the backlog view has
    // something on every one of its three axes.
    {
        let entry_pid = add_entry(
            db,
            site_pid,
            "queued-translation-request",
            "page",
            AUTHORS[2],
            12,
        )
        .await?;
        entry_count += 1;
        let english = Uuid::new_v4();
        let rev = add_revision(
            db,
            english,
            Rev::healthy(1, "Awaiting translation", 12, AUTHORS[2]),
        )
        .await?;
        add_variant(
            db,
            VariantSpec {
                pid: english,
                entry_pid,
                locale: "en",
                status: "published",
                published: Some(rev),
                current: rev,
                translation_of: None,
                translation_status: None,
                scheduled_publish_at: None,
                updated_at: days_ago(11),
            },
        )
        .await?;
        variant_count += 1;
        add_route(db, site_pid, "en", "/queued-translation-request", english).await?;

        // The French side exists but has not been written yet.
        let french = Uuid::new_v4();
        let draft = add_revision(db, french, Rev::healthy(1, "À traduire", 10, AUTHORS[2])).await?;
        add_variant(
            db,
            VariantSpec {
                pid: french,
                entry_pid,
                locale: "fr",
                status: "draft",
                published: None,
                current: draft,
                translation_of: Some(rev),
                translation_status: Some("requested"),
                scheduled_publish_at: None,
                updated_at: days_ago(10),
            },
        )
        .await?;
        variant_count += 1;
    }
    {
        let entry_pid = add_entry(
            db,
            site_pid,
            "queued-scheduled-publish",
            "page",
            AUTHORS[0],
            5,
        )
        .await?;
        entry_count += 1;
        let variant = Uuid::new_v4();
        let rev = add_revision(
            db,
            variant,
            Rev::healthy(1, "Goes live later", 5, AUTHORS[0]),
        )
        .await?;
        add_variant(
            db,
            VariantSpec {
                pid: variant,
                entry_pid,
                locale: "en",
                status: "approved",
                published: None,
                current: rev,
                translation_of: None,
                translation_status: None,
                // Deliberately in the future: a seeded schedule in the
                // past would be swept into publication the first time
                // the sweep ran, and the demo would quietly change.
                scheduled_publish_at: Some(days_ago(-7)),
                updated_at: days_ago(4),
            },
        )
        .await?;
        variant_count += 1;
        add_transition(db, variant, "created", AUTHORS[0], days_ago(5)).await?;
        add_transition(db, variant, "submit", AUTHORS[0], days_ago(5)).await?;
        add_transition(db, variant, "approve", EDITOR, days_ago(4)).await?;
    }

    // 9. `route_hazard` — a redirect chain four hops long against a cap
    //    of five, plus a menu pointing at an entry that is gone.
    for (from, to) in [
        ("/legacy-a", "/legacy-b"),
        ("/legacy-b", "/legacy-c"),
        ("/legacy-c", "/legacy-d"),
        ("/legacy-d", "/guide-getting-started"),
        // An ordinary rename, for contrast: one hop, no hazard.
        ("/old-permissions", "/guide-permissions"),
    ] {
        redirects::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            site_pid: ActiveValue::set(site_pid),
            locale: ActiveValue::set("en".to_string()),
            from_path: ActiveValue::set(from.to_string()),
            to_path: ActiveValue::set(Some(to.to_string())),
            status: ActiveValue::set(301),
            reason: ActiveValue::set("seeded fixture".to_string()),
            created_at: ActiveValue::set(days_ago(15)),
            updated_at: ActiveValue::set(days_ago(15)),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    // A menu, one of whose items points at nothing — the second half of
    // the route-hazard rule.
    menus::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        site_pid: ActiveValue::set(site_pid),
        locale: ActiveValue::set("en".to_string()),
        key: ActiveValue::set("primary".to_string()),
        items: ActiveValue::set(serde_json::json!([
            { "label": "Getting started", "path": "/guide-getting-started" },
            { "label": "Permissions", "path": "/guide-permissions" },
            { "label": "Integrations", "path": "/guide-integrations" },
            { "label": "Gone", "entry_pid": Uuid::new_v4().to_string() },
        ])),
        deleted_at: ActiveValue::set(None),
        created_at: ActiveValue::set(days_ago(15)),
        updated_at: ActiveValue::set(hours_ago(2)),
        ..Default::default()
    }
    .insert(db)
    .await?;

    // 10. `orphan_asset` — whatever nothing referenced. Counted rather
    //     than planted, because the honest number is the one the rule
    //     will report, not the one the fixture intended.
    let orphans = assets_all
        .iter()
        .filter(|pid| !referenced.contains(pid))
        .count();

    Ok(Planted {
        entries: entry_count,
        variants: variant_count,
        assets: assets_all.len(),
        orphans,
    })
}

/// Bump the `article` type to v2, adding a required `summary`.
///
/// This is what makes `needs_migration` demonstrable: the planted v1
/// revision has no `summary`, and today's declaration requires one.
///
/// # Errors
///
/// When the update fails.
pub(super) async fn bump_article_schema(db: &DatabaseConnection, site_pid: Uuid) -> Result<()> {
    let Some(article) = content_types::Entity::find()
        .filter(content_types::Column::SitePid.eq(site_pid))
        .filter(content_types::Column::Key.eq("article"))
        .one(db)
        .await?
    else {
        return Ok(());
    };
    let mut fields = article.fields.as_array().cloned().unwrap_or_default();
    fields.push(serde_json::json!({
        "key": "summary", "label": "Summary", "kind": "text", "required": true,
        "validation": { "max_len": 500 }
    }));
    let mut active: content_types::ActiveModel = article.into();
    active.fields = ActiveValue::set(serde_json::Value::Array(fields));
    active.schema_version = ActiveValue::set(2);
    active.update(db).await?;
    Ok(())
}
