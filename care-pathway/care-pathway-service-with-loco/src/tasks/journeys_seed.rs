//! `journeys:seed` task — persist a deterministic, synthetic cohort of
//! pathway-instance journeys (spec `13-tasks.md` T-14m). The generation
//! itself is pure and DB-free ([`crate::data::journeys`]); this task is
//! only the thin impure layer that writes what it returns.
//!
//! ```text
//! cargo loco task journeys:seed pathway:<pid>
//! cargo loco task journeys:seed pathway:<pid> n:50 seed:7 open_share:0.3
//! cargo loco task journeys:seed pathway:<pid> defects:all
//! cargo loco task journeys:seed pathway:<pid> defects:no_segments,steps_out_of_order
//! ```
//!
//! **Never real data, never derived from real data.** Every generated
//! `subject_ref` / care-team `member_ref` / `location_ref` carries a
//! fixed, recognizably-fake byte prefix (see
//! `crate::data::journeys`'s module docs) — the same discipline
//! IPPA-data's own disclaimer names: this data answers no
//! epidemiological question and must never be mistaken for one that
//! does.

use loco_rs::prelude::*;
use sea_orm::ActiveValue;
use uuid::Uuid;

use crate::data::journeys::{self, SeedParams};
use crate::models::_entities::{
    instance_events, instance_segments, instance_steps, instance_team, pathway_instances,
};
use crate::models::care_pathways::Model as PathwayModel;

/// Parse one `key:value` CLI argument, falling back to `default` when
/// absent.
///
/// # Errors
///
/// The value is present but does not parse as `T`.
fn parse_arg<T: std::str::FromStr>(vars: &task::Vars, key: &str, default: T) -> Result<T> {
    match vars.cli.get(key) {
        None => Ok(default),
        Some(raw) => raw
            .parse::<T>()
            .map_err(|_| Error::string(&format!("{key} must be a valid value, got '{raw}'"))),
    }
}

/// Parse `defects:` — `all` for every `DEFECT_CODES` entry, a
/// comma-separated list of codes, or absent for none. Validity of each
/// code is checked by [`journeys::generate_cohort`], not here, so the
/// error message names the actual offending code.
fn parse_defects(vars: &task::Vars) -> Vec<String> {
    match vars.cli.get("defects").map(String::as_str) {
        None | Some("") => Vec::new(),
        Some("all") => journeys::DEFECT_CODES
            .iter()
            .map(ToString::to_string)
            .collect(),
        Some(list) => list
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .collect(),
    }
}

/// Persist one generated instance and its child rows, returning the new
/// instance's `pid`. Generic over `ConnectionTrait` so the caller
/// controls the transaction boundary.
async fn insert_instance<C: sea_orm::ConnectionTrait>(
    db: &C,
    pathway_pid: Uuid,
    generated: &journeys::GeneratedInstance,
) -> Result<Uuid> {
    let instance = pathway_instances::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        pathway_pid: ActiveValue::set(pathway_pid),
        subject_ref: ActiveValue::set(generated.subject_ref.clone()),
        status: ActiveValue::set(generated.status.clone()),
        urgency: ActiveValue::set(generated.urgency.clone()),
        enrolled_on: ActiveValue::set(generated.enrolled_on),
        next_review_on: ActiveValue::set(None),
        closed_on: ActiveValue::set(generated.closed_on),
        closure_reason: ActiveValue::set(None),
        outcome: ActiveValue::set(generated.outcome.clone()),
        clock_start_at: ActiveValue::set(generated.clock_start_at.map(Into::into)),
        clock_stop_at: ActiveValue::set(generated.clock_stop_at.map(Into::into)),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(db)
    .await?;

    for member in &generated.team {
        instance_team::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            instance_pid: ActiveValue::set(instance.pid),
            member_ref: ActiveValue::set(member.member_ref.clone()),
            role: ActiveValue::set(member.role.clone()),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    for (position, segment) in generated.segments.iter().enumerate() {
        instance_segments::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            instance_pid: ActiveValue::set(instance.pid),
            label: ActiveValue::set(segment.label.clone()),
            stage: ActiveValue::set(segment.stage.clone()),
            category: ActiveValue::set(segment.category.clone()),
            waste: ActiveValue::set(segment.waste.clone()),
            started_at: ActiveValue::set(segment.started_at.into()),
            ended_at: ActiveValue::set(segment.ended_at.map(Into::into)),
            actor_ref: ActiveValue::set(segment.actor_ref.clone()),
            location_ref: ActiveValue::set(segment.location_ref.clone()),
            note: ActiveValue::set(None),
            position: ActiveValue::set(i32::try_from(position).unwrap_or(i32::MAX)),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    for (position, step) in generated.steps.iter().enumerate() {
        instance_steps::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            instance_pid: ActiveValue::set(instance.pid),
            label: ActiveValue::set(step.label.clone()),
            done: ActiveValue::set(step.done),
            done_on: ActiveValue::set(step.done_on),
            position: ActiveValue::set(i32::try_from(position).unwrap_or(i32::MAX)),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    for event in &generated.events {
        instance_events::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            instance_pid: ActiveValue::set(instance.pid),
            kind: ActiveValue::set(event.kind.clone()),
            occurred_at: ActiveValue::set(event.occurred_at.into()),
            note: ActiveValue::set(None),
            actor: ActiveValue::set(Some("journeys:seed".to_string())),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(instance.pid)
}

/// The `journeys:seed` CLI task.
pub struct JourneysSeed;

#[async_trait]
impl Task for JourneysSeed {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "journeys:seed".to_string(),
            detail: "Generate a deterministic synthetic journey cohort on a pathway (synthetic \
                     data only). Args: pathway:<pid> [n:<count>=10] [seed:<u64>=42] \
                     [open_share:<0..1>=0.2] [defects:<all|code,code,...>]."
                .to_string(),
        }
    }

    /// # Errors
    ///
    /// `pathway:` is missing or names no live pathway; `n:`/`seed:`/
    /// `open_share:` do not parse; a `defects:` code is not one of
    /// [`journeys::DEFECT_CODES`]; or a database write fails.
    async fn run(&self, ctx: &AppContext, vars: &task::Vars) -> Result<()> {
        let pathway_raw = vars
            .cli
            .get("pathway")
            .ok_or_else(|| Error::string("pathway:<pid> is required"))?;
        let template = PathwayModel::find_by_pid(&ctx.db, pathway_raw)
            .await
            .map_err(|_| Error::string(&format!("unknown pathway {pathway_raw}")))?;

        let params = SeedParams {
            n: parse_arg(vars, "n", 10usize)?,
            seed: parse_arg(vars, "seed", 42u64)?,
            open_share: parse_arg(vars, "open_share", 0.2f64)?,
            defects: parse_defects(vars),
            base_date: journeys::default_base_date(),
        };
        let cohort = journeys::generate_cohort(&params).map_err(|e| Error::string(&e))?;

        let txn = ctx.db.begin().await?;
        let mut defect_pids: Vec<(String, Uuid)> = Vec::new();
        for generated in &cohort.instances {
            let instance_pid = insert_instance(&txn, template.pid, generated).await?;
            if let Some(code) = &generated.defect {
                defect_pids.push((code.clone(), instance_pid));
            }
        }
        txn.commit().await?;

        println!(
            "journeys:seed: created {} synthetic instance(s) on pathway {} (seed {})",
            cohort.instances.len(),
            template.pid,
            params.seed
        );
        for (code, pid) in &defect_pids {
            println!("  defect {code} -> instance {pid}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> std::collections::BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    /// `defects:` parses `all`, a comma list, and absence — each
    /// distinctly, never conflated.
    #[test]
    fn defects_arg_parses() {
        let task_vars = task::Vars { cli: vars(&[]) };
        assert_eq!(parse_defects(&task_vars), Vec::<String>::new());

        let task_vars = task::Vars {
            cli: vars(&[("defects", "all")]),
        };
        assert_eq!(parse_defects(&task_vars), journeys::DEFECT_CODES.to_vec());

        let task_vars = task::Vars {
            cli: vars(&[("defects", "no_segments, steps_out_of_order")]),
        };
        assert_eq!(
            parse_defects(&task_vars),
            vec!["no_segments".to_string(), "steps_out_of_order".to_string()]
        );
    }

    /// A missing numeric arg falls back to the default; a present one
    /// must actually parse.
    #[test]
    fn numeric_args_default_or_parse() {
        let task_vars = task::Vars { cli: vars(&[]) };
        assert_eq!(parse_arg(&task_vars, "n", 10usize).expect("default"), 10);

        let task_vars = task::Vars {
            cli: vars(&[("n", "50")]),
        };
        assert_eq!(parse_arg(&task_vars, "n", 10usize).expect("parses"), 50);

        let task_vars = task::Vars {
            cli: vars(&[("n", "not-a-number")]),
        };
        assert!(parse_arg(&task_vars, "n", 10usize).is_err());
    }
}
