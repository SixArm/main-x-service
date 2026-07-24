//! `cargo loco task seed` — create a synthetic demo organization so
//! the HR views demo instantly. **Synthetic data only** (spec
//! `regulatory.md`): person/worker URNs are random UUIDs, names are
//! obviously fictional.

use loco_rs::prelude::*;
use loco_rs::task::{TaskInfo, Vars};
use sea_orm::ActiveValue;
use uuid::Uuid;

use crate::models::_entities::{
    benchmarks, benefit_plans, employees, leave_entitlements, requisitions,
};

/// The demo-organization seed task.
pub struct Seed;

#[async_trait]
impl Task for Seed {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "seed".to_string(),
            detail: "Seed a synthetic demo organization (~40 employees, requisitions, plans, benchmarks)"
                .to_string(),
        }
    }

    #[allow(clippy::too_many_lines)] // one linear walk building the demo org
    async fn run(&self, ctx: &AppContext, _vars: &Vars) -> Result<()> {
        let db = &ctx.db;
        let org = format!("organization:{}", Uuid::new_v4());
        let departments = ["engineering", "clinical", "operations", "finance"];
        let titles = [
            ("Engineer", 3_600_000_i64),
            ("Senior Engineer", 4_800_000),
            ("Nurse", 3_200_000),
            ("Analyst", 3_000_000),
            ("Coordinator", 2_800_000),
        ];
        let mut managers: Vec<Uuid> = Vec::new();
        let mut n = 0;
        for department in departments {
            // One department head (no manager), then nine reports.
            let head = Uuid::new_v4();
            n += 1;
            employees::ActiveModel {
                pid: ActiveValue::set(head),
                person_ref: ActiveValue::set(format!("person:{}", Uuid::new_v4())),
                worker_ref: ActiveValue::set(Some(format!("worker:{}", Uuid::new_v4()))),
                organization_ref: ActiveValue::set(org.clone()),
                employee_number: ActiveValue::set(format!("E-{n:04}")),
                display_name: ActiveValue::set(format!("Test Head {n:03}")),
                status: ActiveValue::set("active".to_string()),
                employment_type: ActiveValue::set("permanent".to_string()),
                fte_percent: ActiveValue::set(100),
                department: ActiveValue::set(department.to_string()),
                job_title: ActiveValue::set("Head of Department".to_string()),
                manager_pid: ActiveValue::set(None),
                salary_minor: ActiveValue::set(Some(6_500_000)),
                salary_currency: ActiveValue::set(Some("GBP".to_string())),
                hired_on: ActiveValue::set(
                    chrono::NaiveDate::from_ymd_opt(2020, 1, 6).unwrap(),
                ),
                terminated_on: ActiveValue::set(None),
                deleted_at: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(db)
            .await?;
            managers.push(head);
            for i in 0..9 {
                n += 1;
                let (title, salary) = titles[usize::try_from(n).unwrap_or(0) % titles.len()];
                let pid = Uuid::new_v4();
                employees::ActiveModel {
                    pid: ActiveValue::set(pid),
                    person_ref: ActiveValue::set(format!("person:{}", Uuid::new_v4())),
                    worker_ref: ActiveValue::set(None),
                    organization_ref: ActiveValue::set(org.clone()),
                    employee_number: ActiveValue::set(format!("E-{n:04}")),
                    display_name: ActiveValue::set(format!("Test Employee {n:03}")),
                    status: ActiveValue::set(if i == 8 { "onboarding" } else { "active" }.to_string()),
                    employment_type: ActiveValue::set(
                        if i % 4 == 3 { "fixed_term" } else { "permanent" }.to_string(),
                    ),
                    fte_percent: ActiveValue::set(if i % 5 == 4 { 60 } else { 100 }),
                    department: ActiveValue::set(department.to_string()),
                    job_title: ActiveValue::set(title.to_string()),
                    manager_pid: ActiveValue::set(Some(head)),
                    salary_minor: ActiveValue::set(Some(salary)),
                    salary_currency: ActiveValue::set(Some("GBP".to_string())),
                    hired_on: ActiveValue::set(
                        chrono::NaiveDate::from_ymd_opt(2023, 3, 1).unwrap(),
                    ),
                    terminated_on: ActiveValue::set(None),
                    deleted_at: ActiveValue::set(None),
                    ..Default::default()
                }
                .insert(db)
                .await?;
                // Annual + sick entitlements for the current year.
                for (kind, days) in [("annual", 25), ("sick", 10)] {
                    leave_entitlements::ActiveModel {
                        pid: ActiveValue::set(Uuid::new_v4()),
                        employee_pid: ActiveValue::set(pid),
                        kind: ActiveValue::set(kind.to_string()),
                        year: ActiveValue::set(2026),
                        entitled_days: ActiveValue::set(days),
                        used_days: ActiveValue::set(0),
                        deleted_at: ActiveValue::set(None),
                        ..Default::default()
                    }
                    .insert(db)
                    .await?;
                }
            }
        }
        // Requisitions across the pipeline.
        for (department, title, status) in [
            ("engineering", "Platform Engineer", "open"),
            ("clinical", "Ward Nurse", "interviewing"),
            ("operations", "Scheduler", "draft"),
            ("finance", "Payroll Analyst", "offer"),
        ] {
            requisitions::ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                organization_ref: ActiveValue::set(org.clone()),
                department: ActiveValue::set(department.to_string()),
                job_title: ActiveValue::set(title.to_string()),
                headcount: ActiveValue::set(1),
                salary_min_minor: ActiveValue::set(Some(2_800_000)),
                salary_max_minor: ActiveValue::set(Some(5_000_000)),
                salary_currency: ActiveValue::set(Some("GBP".to_string())),
                status: ActiveValue::set(status.to_string()),
                opened_on: ActiveValue::set(Some(
                    chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
                )),
                deleted_at: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
        // Benefit plans + benchmarks.
        for (name, kind, employee_cost, employer_cost) in [
            ("Pension 5%", "pension", 15_000_i64, 30_000_i64),
            ("Health Cover", "health", 8_000, 20_000),
        ] {
            benefit_plans::ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                name: ActiveValue::set(name.to_string()),
                kind: ActiveValue::set(kind.to_string()),
                provider: ActiveValue::set("Demo Provider Ltd".to_string()),
                employee_cost_minor: ActiveValue::set(employee_cost),
                employer_cost_minor: ActiveValue::set(employer_cost),
                currency: ActiveValue::set("GBP".to_string()),
                deleted_at: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
        for (title, min, median, max) in [
            ("Engineer", 3_200_000_i64, 3_800_000_i64, 4_600_000_i64),
            ("Nurse", 2_900_000, 3_300_000, 3_900_000),
            ("Analyst", 2_600_000, 3_100_000, 3_700_000),
        ] {
            benchmarks::ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                job_title: ActiveValue::set(title.to_string()),
                currency: ActiveValue::set("GBP".to_string()),
                min_minor: ActiveValue::set(min),
                median_minor: ActiveValue::set(median),
                max_minor: ActiveValue::set(max),
                source: ActiveValue::set("Demo Salary Survey 2026".to_string()),
                as_of: ActiveValue::set(chrono::NaiveDate::from_ymd_opt(2026, 4, 1).unwrap()),
                deleted_at: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
        tracing::info!(organization = %org, employees = n, "seeded synthetic demo organization");
        println!("seeded: organization {org} with {n} employees");
        Ok(())
    }
}
