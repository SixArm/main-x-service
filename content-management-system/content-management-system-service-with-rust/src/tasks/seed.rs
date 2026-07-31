//! `task seed` — a synthetic demo site with templates and content
//! types, so the API can be exercised end-to-end without hand-crafting
//! JSON.
//!
//! Two layers. The **declaration** layer is here: one site with a
//! fallback chain, two templates, and three content types exercising
//! most field kinds. The **content corpus** — entries across three
//! locales, revisions in every workflow state, assets with planted
//! orphans, a redirect chain, stale translations, and one deliberate
//! instance of every content-health rule — lives in
//! [`super::seed_corpus`], which is long enough to deserve its own
//! file and is read as a fixture rather than as logic.
//!
//! Synthetic data only — no real copy, no real imagery
//! (spec `regulatory.md`).

use loco_rs::prelude::*;
use uuid::Uuid;

use crate::models::_entities::{content_types, sites, templates};

/// The seed task.
pub struct Seed;

#[async_trait]
impl Task for Seed {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "seed".to_string(),
            detail: "Seed a synthetic demo site with templates and content types".to_string(),
        }
    }

    /// Insert the demo declaration layer, skipping if the site already
    /// exists so a rerun is harmless.
    ///
    /// # Errors
    ///
    /// When a database write fails.
    #[allow(clippy::too_many_lines)] // one literal fixture set
    async fn run(&self, ctx: &AppContext, _vars: &task::Vars) -> Result<()> {
        if crate::models::records::find_site_by_key(&ctx.db, "demo")
            .await
            .is_ok()
        {
            tracing::info!("seed: site `demo` already exists; nothing to do");
            return Ok(());
        }

        let site_pid = Uuid::new_v4();
        sites::ActiveModel {
            pid: ActiveValue::set(site_pid),
            key: ActiveValue::set("demo".to_string()),
            name: ActiveValue::set("Demo site".to_string()),
            owner_ref: ActiveValue::set(None),
            default_locale: ActiveValue::set("en".to_string()),
            locales: ActiveValue::set(serde_json::json!(["en", "fr", "fr-CA"])),
            fallback_chains: ActiveValue::set(serde_json::json!({
                "fr-CA": ["fr", "en"],
                "fr": ["en"],
            })),
            strict_locales: ActiveValue::set(serde_json::json!([])),
            // Restricted, like every site that nobody has deliberately
            // opened (CMS-D7) — a seeded demo is not a reason to ship a
            // world-readable default.
            visibility: ActiveValue::set("restricted".to_string()),
            base_url: ActiveValue::set(Some("https://demo.example.test".to_string())),
            robots_default: ActiveValue::set("index,follow".to_string()),
            require_distinct_approver: ActiveValue::set(true),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await?;

        for (key, name, regions) in [
            (
                "article",
                "Article layout",
                serde_json::json!([
                    { "key": "hero", "label": "Hero", "allowed_block_kinds": ["image"], "min": 0, "max": 1 },
                    { "key": "body", "label": "Body", "allowed_block_kinds": ["heading", "paragraph", "list", "quote", "image"], "min": 1 },
                ]),
            ),
            (
                "page",
                "Page layout",
                serde_json::json!([
                    { "key": "body", "label": "Body", "min": 1 },
                ]),
            ),
        ] {
            templates::ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                site_pid: ActiveValue::set(site_pid),
                key: ActiveValue::set(key.to_string()),
                name: ActiveValue::set(name.to_string()),
                regions: ActiveValue::set(regions),
                applies_to_type_keys: ActiveValue::set(serde_json::json!([key])),
                deleted_at: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(&ctx.db)
            .await?;
        }

        for (key, name, template, routable, fields) in [
            (
                "article",
                "Article",
                Some("article"),
                true,
                serde_json::json!([
                    { "key": "standfirst", "label": "Standfirst", "kind": "text", "validation": { "max_len": 300 } },
                    { "key": "section", "label": "Section", "kind": "choice", "required": true,
                      "validation": { "options": ["news", "guide", "opinion"] } },
                    { "key": "hero_image", "label": "Hero image", "kind": "media" },
                    { "key": "published_on", "label": "Published on", "kind": "date" },
                    { "key": "related", "label": "Related articles", "kind": "reference", "repeatable": true,
                      "validation": { "type_keys": ["article"] } },
                ]),
            ),
            (
                "page",
                "Page",
                Some("page"),
                true,
                serde_json::json!([
                    { "key": "summary", "label": "Summary", "kind": "text", "validation": { "max_len": 500 } },
                    { "key": "show_in_nav", "label": "Show in navigation", "kind": "boolean" },
                ]),
            ),
            (
                "course-listing",
                "Course listing",
                None,
                true,
                serde_json::json!([
                    // The pointer pattern (spec `scope.md`): editorial
                    // copy *about* a registered course, never a copy of
                    // the course record itself.
                    { "key": "course", "label": "Course", "kind": "entity_ref", "required": true,
                      "validation": { "entity_types": ["course"] } },
                    { "key": "blurb", "label": "Blurb", "kind": "text" },
                ]),
            ),
        ] {
            content_types::ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                site_pid: ActiveValue::set(site_pid),
                key: ActiveValue::set(key.to_string()),
                name: ActiveValue::set(name.to_string()),
                description: ActiveValue::set(None),
                fields: ActiveValue::set(fields),
                routable: ActiveValue::set(routable),
                template_key: ActiveValue::set(template.map(ToString::to_string)),
                schema_version: ActiveValue::set(1),
                deleted_at: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(&ctx.db)
            .await?;
        }

        // The article type moves to v2 *after* the corpus is planted
        // would be wrong: the corpus writes one revision at v1 on
        // purpose, and the bump is what makes today's declaration
        // reject it. Bumping first keeps that relationship explicit.
        super::seed_corpus::bump_article_schema(&ctx.db, site_pid).await?;
        let planted = super::seed_corpus::plant(&ctx.db, site_pid).await?;

        tracing::info!(
            entries = planted.entries,
            variants = planted.variants,
            assets = planted.assets,
            orphans = planted.orphans,
            "seed: site `demo` with 2 templates, 3 content types, and the synthetic corpus"
        );
        // Said out loud because it is the one place a seeded row does
        // not behave like a real one.
        tracing::info!(
            "seed: asset rows describe files that were never uploaded, so their metadata \
             serves but their content does not"
        );
        Ok(())
    }
}
