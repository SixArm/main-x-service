//! `cargo loco task seed` — create a synthetic book of business so
//! the CRM views demo instantly. **Synthetic data only** (spec
//! `regulatory.md`): person/organization/worker URNs are random
//! UUIDs, names are obviously fictional, no email is sent.

use loco_rs::prelude::*;
use loco_rs::task::{TaskInfo, Vars};
use sea_orm::ActiveValue;
use uuid::Uuid;

use crate::models::_entities::{
    accounts, contacts, deals, leads, pipeline_stages, pipelines, sla_policies,
};

/// The demo book-of-business seed task.
pub struct Seed;

#[async_trait]
impl Task for Seed {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "seed".to_string(),
            detail: "Seed a synthetic book of business (accounts, contacts, pipeline, deals, leads, SLAs)"
                .to_string(),
        }
    }

    #[allow(clippy::too_many_lines)] // one linear walk building the demo book
    async fn run(&self, ctx: &AppContext, _vars: &Vars) -> Result<()> {
        let db = &ctx.db;
        // Accounts + contacts.
        let tiers = ["customer", "customer", "prospect", "partner", "prospect"];
        let mut account_pids = Vec::new();
        for (index, tier) in tiers.iter().enumerate() {
            let account = accounts::ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                organization_ref: ActiveValue::set(format!("organization:{}", Uuid::new_v4())),
                owner_ref: ActiveValue::set(Some(format!("worker:{}", Uuid::new_v4()))),
                display_name: ActiveValue::set(format!("Demo Account {:02}", index + 1)),
                tier: ActiveValue::set((*tier).to_string()),
                industry: ActiveValue::set(Some("technology".to_string())),
                deleted_at: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(db)
            .await?;
            account_pids.push(account.pid);
            for c in 0..3 {
                contacts::ActiveModel {
                    pid: ActiveValue::set(Uuid::new_v4()),
                    person_ref: ActiveValue::set(format!("person:{}", Uuid::new_v4())),
                    account_pid: ActiveValue::set(Some(account.pid)),
                    owner_ref: ActiveValue::set(None),
                    display_name: ActiveValue::set(format!("Demo Contact {:02}-{c}", index + 1)),
                    status: ActiveValue::set("active".to_string()),
                    job_title: ActiveValue::set(Some("Buyer".to_string())),
                    preferred_channel: ActiveValue::set("email".to_string()),
                    // Two of three consented, one never — the segment
                    // gate is visible in the demo.
                    marketing_consent: ActiveValue::set(
                        if c == 2 { "never" } else { "granted" }.to_string(),
                    ),
                    consent_changed_at: ActiveValue::set(None),
                    deleted_at: ActiveValue::set(None),
                    ..Default::default()
                }
                .insert(db)
                .await?;
            }
        }
        // One pipeline with five stages.
        let pipeline = pipelines::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            name: ActiveValue::set("New Business".to_string()),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(db)
        .await?;
        let stage_defs = [
            ("Qualification", 10, false, false),
            ("Discovery", 25, false, false),
            ("Proposal", 55, false, false),
            ("Won", 100, true, false),
            ("Lost", 0, false, true),
        ];
        let mut stage_pids = Vec::new();
        for (position, (name, probability, is_won, is_lost)) in stage_defs.iter().enumerate() {
            let stage = pipeline_stages::ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                pipeline_pid: ActiveValue::set(pipeline.pid),
                name: ActiveValue::set((*name).to_string()),
                position: ActiveValue::set(i32::try_from(position).unwrap_or(0)),
                probability_percent: ActiveValue::set(*probability),
                is_won: ActiveValue::set(*is_won),
                is_lost: ActiveValue::set(*is_lost),
                deleted_at: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(db)
            .await?;
            stage_pids.push(stage.pid);
        }
        // Deals across the stages (some closed).
        for i in 0..12_usize {
            let stage_index = i % 5;
            let closed = stage_index >= 3;
            deals::ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                account_pid: ActiveValue::set(Some(account_pids[i % account_pids.len()])),
                primary_contact_pid: ActiveValue::set(None),
                owner_ref: ActiveValue::set(Some(format!("worker:{}", Uuid::new_v4()))),
                pipeline_pid: ActiveValue::set(pipeline.pid),
                stage_pid: ActiveValue::set(stage_pids[stage_index]),
                name: ActiveValue::set(format!("Demo Deal {:02}", i + 1)),
                amount_minor: ActiveValue::set(1_000_000 + i64::try_from(i).unwrap_or(0) * 250_000),
                currency: ActiveValue::set("GBP".to_string()),
                expected_close_on: ActiveValue::set(chrono::NaiveDate::from_ymd_opt(2026, 9, 30)),
                kanban_position: ActiveValue::set(i32::try_from(i).unwrap_or(0)),
                source_campaign_pid: ActiveValue::set(None),
                closed_at: ActiveValue::set(closed.then(|| chrono::Utc::now().into())),
                won: ActiveValue::set(stage_index == 3),
                lost_reason: ActiveValue::set((stage_index == 4).then(|| "budget cut".to_string())),
                deleted_at: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
        // Leads in every status.
        for (i, status) in ["new", "new", "contacted", "qualified", "disqualified"]
            .iter()
            .enumerate()
        {
            leads::ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                source: ActiveValue::set(if i % 2 == 0 { "web" } else { "referral" }.to_string()),
                campaign_pid: ActiveValue::set(None),
                contact_pid: ActiveValue::set(None),
                display_name: ActiveValue::set(format!("Demo Lead {:02}", i + 1)),
                email: ActiveValue::set(Some(format!("lead{i}@initech.example"))),
                email_domain: ActiveValue::set(Some("initech.example".to_string())),
                score: ActiveValue::set(if i % 2 == 0 { 10 } else { 30 }),
                campaign_click: ActiveValue::set(false),
                unsubscribed: ActiveValue::set(false),
                status: ActiveValue::set((*status).to_string()),
                deleted_at: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
        // SLA policies for every priority.
        for (priority, first, resolution) in [
            ("low", 480, 4320),
            ("normal", 240, 1440),
            ("high", 60, 480),
            ("urgent", 15, 240),
        ] {
            sla_policies::ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                priority: ActiveValue::set(priority.to_string()),
                first_response_minutes: ActiveValue::set(first),
                resolution_minutes: ActiveValue::set(resolution),
                deleted_at: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
        tracing::info!("seeded synthetic book of business");
        println!("seeded: 5 accounts, 15 contacts, 1 pipeline, 12 deals, 5 leads, 4 SLA policies");
        Ok(())
    }
}
