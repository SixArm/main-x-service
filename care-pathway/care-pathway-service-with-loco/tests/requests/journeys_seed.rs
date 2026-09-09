//! The `journeys:seed` task (spec `13-tasks.md` T-14m), end to end
//! against a real Postgres: it persists exactly what the pure generator
//! ([`care_pathway_service::data::journeys`]) returns, `defects:`
//! reaches the database as the requested conditions, and a bad
//! argument is refused rather than silently defaulted.

use care_pathway_service::app::App;
use care_pathway_service::data::journeys::DEFECT_CODES;
use care_pathway_service::models::_entities::{
    instance_segments, instance_team, pathway_instances,
};
use care_pathway_service::tasks::journeys_seed::JourneysSeed;
use loco_rs::TestServer;
use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use serial_test::serial;

fn vars(pairs: &[(&str, &str)]) -> std::collections::BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

async fn seed_pathway(request: &TestServer) -> String {
    let created = request
        .post("/api/care-pathways")
        .json(&json!({
            "name": format!("journeys:seed pathway {}", uuid::Uuid::new_v4()),
            "care_setting": "Outpatient",
            "condition_codes": [{"system": "Icd10", "code": "M54"}],
        }))
        .await;
    created.assert_status_ok();
    let template: Value = created.json();
    template["pid"].as_str().expect("pathway pid").to_string()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn journeys_seed_persists_exactly_what_it_generates() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let pathway = seed_pathway(&request).await;

        let task_vars = task::Vars {
            cli: vars(&[
                ("pathway", &pathway),
                ("n", "6"),
                ("seed", "99"),
                ("open_share", "0.5"),
                ("defects", "no_segments,steps_out_of_order"),
            ]),
        };
        JourneysSeed
            .run(&ctx, &task_vars)
            .await
            .expect("journeys:seed succeeds against a real pathway");

        let pathway_pid = uuid::Uuid::parse_str(&pathway).expect("valid pid");
        let instances = pathway_instances::Entity::find()
            .filter(pathway_instances::Column::PathwayPid.eq(pathway_pid))
            .all(&ctx.db)
            .await
            .expect("query instances");
        // 6 clean + 2 defect instances.
        assert_eq!(instances.len(), 8);

        // Every subject_ref is recognizably synthetic — never real data.
        for instance in &instances {
            assert!(
                instance.subject_ref.starts_with("person:facade50"),
                "{}",
                instance.subject_ref
            );
        }

        // The `no_segments` defect instance really has none.
        let mut found_no_segments = false;
        for instance in &instances {
            let segments = instance_segments::Entity::find()
                .filter(instance_segments::Column::InstancePid.eq(instance.pid))
                .all(&ctx.db)
                .await
                .expect("query segments");
            if segments.is_empty() {
                found_no_segments = true;
            }
        }
        assert!(
            found_no_segments,
            "expected exactly one instance carrying the no_segments defect"
        );

        // Every generated team member is also recognizably synthetic.
        let team_rows = instance_team::Entity::find()
            .all(&ctx.db)
            .await
            .expect("query team");
        for member in &team_rows {
            assert!(
                member.member_ref.contains(":facade51"),
                "{}",
                member.member_ref
            );
        }
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn journeys_seed_rejects_bad_arguments() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let pathway = seed_pathway(&request).await;

        let missing_pathway = task::Vars { cli: vars(&[]) };
        assert!(JourneysSeed.run(&ctx, &missing_pathway).await.is_err());

        let unknown_pathway = task::Vars {
            cli: vars(&[("pathway", "00000000-0000-0000-0000-000000000000")]),
        };
        assert!(JourneysSeed.run(&ctx, &unknown_pathway).await.is_err());

        let bad_n = task::Vars {
            cli: vars(&[("pathway", &pathway), ("n", "not-a-number")]),
        };
        assert!(JourneysSeed.run(&ctx, &bad_n).await.is_err());

        let bad_defect = task::Vars {
            cli: vars(&[("pathway", &pathway), ("defects", "not_a_real_code")]),
        };
        assert!(JourneysSeed.run(&ctx, &bad_defect).await.is_err());

        // Every real defect code is at least accepted (n:0 keeps this fast).
        let all_defects = task::Vars {
            cli: vars(&[
                ("pathway", &pathway),
                ("n", "0"),
                ("defects", &DEFECT_CODES.join(",")),
            ]),
        };
        JourneysSeed
            .run(&ctx, &all_defects)
            .await
            .expect("every DEFECT_CODES entry is accepted");
    })
    .await;
}
