//! `GET /api/places/{id}/descendants` lists every place below one place
//! in the `contained_in_place` hierarchy, at any depth, paged by signed
//! cursor (spec §13 T-2).
//!
//! `#[ignore]`d — run explicitly with
//! `DB_URL=… cargo test --test descendants_endpoint -- --ignored`.

use axa_test::TestApp;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use place_service::app::App;
use place_service::config::Config;
use place_service::db::{PgPlaceRepository, PlaceRepository, create_connection};
use place_service::models::place::Place;
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

async fn setup() -> (axum::Router, PgPlaceRepository) {
    let config = Config::from_env().expect("load config from env");
    std::fs::create_dir_all(&config.search.index_path).expect("create search index dir");
    let app = App::boot_test()
        .await
        .expect("boot the app: set DB_URL to a running PostgreSQL")
        .router
        .expect("boot_test builds the router");
    let db = create_connection(&config.database)
        .await
        .expect("Postgres connection");
    (app, PgPlaceRepository::new(db))
}

/// Create a place named for this run, inside `parent` when given.
async fn child_of(repo: &PgPlaceRepository, parent: Option<Uuid>, tag: &str) -> Uuid {
    let mut place = Place::new(&format!("Descendants {tag} {}", Uuid::new_v4()));
    place.contained_in_place = parent;
    repo.create(&place).await.expect("seed").id
}

/// Status, and every result id across all pages of `uri`.
async fn walk(app: &axum::Router, uri: &str) -> (StatusCode, Vec<Uuid>) {
    let mut ids = Vec::new();
    let mut next = Some(uri.to_owned());
    while let Some(uri) = next.take() {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        if status != StatusCode::OK {
            return (status, ids);
        }
        next = response
            .headers()
            .get(header::LINK)
            .and_then(|v| v.to_str().ok())
            .and_then(|link| link.strip_prefix('<'))
            .and_then(|link| link.split_once('>'))
            .map(|(target, _)| target.to_owned());
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        for p in body["results"].as_array().unwrap() {
            ids.push(p["id"].as_str().unwrap().parse().unwrap());
        }
    }
    (StatusCode::OK, ids)
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DB_URL=… cargo test --test descendants_endpoint -- --ignored"]
async fn every_level_below_a_place_is_listed_and_nothing_else() {
    let (app, repo) = setup().await;

    // root > l1 > l2 > l3 > l4 > l5 > l6, a branch under l2, a soft-deleted
    // place under l1 with a child of its own, and an unrelated place.
    let root = child_of(&repo, None, "root").await;
    let mut chain = Vec::new();
    let mut parent = root;
    for level in 1..=6 {
        parent = child_of(&repo, Some(parent), &format!("l{level}")).await;
        chain.push(parent);
    }
    let branch = child_of(&repo, Some(chain[1]), "branch").await;
    let gone = child_of(&repo, Some(chain[0]), "gone").await;
    let under_gone = child_of(&repo, Some(gone), "under-gone").await;
    repo.soft_delete(&gone).await.unwrap();
    let unrelated = child_of(&repo, None, "unrelated").await;

    let (status, mut found) = walk(&app, &format!("/api/places/{root}/descendants?limit=2")).await;
    assert_eq!(status, StatusCode::OK);
    let distinct: std::collections::HashSet<&Uuid> = found.iter().collect();
    assert_eq!(distinct.len(), found.len(), "a place repeated across pages");
    let mut expected = chain.clone();
    expected.push(branch);
    expected.sort();
    let in_id_order = found.clone();
    found.sort();
    assert_eq!(found, expected, "six levels and the branch, nothing else");
    assert_eq!(in_id_order, found, "pages come in id order");
    for absent in [root, gone, under_gone, unrelated] {
        assert!(!found.contains(&absent), "{absent} should not be listed");
    }

    // From the middle: only what is below it.
    let (_, mid) = walk(&app, &format!("/api/places/{}/descendants", chain[3])).await;
    let mut mid = mid;
    mid.sort();
    let mut below = chain[4..].to_vec();
    below.sort();
    assert_eq!(mid, below);

    // A leaf has none; an unknown place is a 404.
    let (status, leaf) = walk(&app, &format!("/api/places/{}/descendants", chain[5])).await;
    assert_eq!(status, StatusCode::OK);
    assert!(leaf.is_empty());
    let (status, _) = walk(&app, &format!("/api/places/{}/descendants", Uuid::new_v4())).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = walk(&app, &format!("/api/places/{root}/descendants?limit=101")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
