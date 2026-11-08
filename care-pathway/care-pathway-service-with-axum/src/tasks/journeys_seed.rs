//! `journeys:seed` task — persist a deterministic, synthetic cohort of
//! pathway-instance journeys (spec `13-tasks.md` T-14m). The generation
//! itself is pure and DB-free ([`crate::data::journeys`]); this task is
//! only the thin impure layer that writes what it returns.
//!
//! ```text
//! cargo axa task journeys:seed pathway:<pid>
//! cargo axa task journeys:seed pathway:<pid> n:50 seed:7 open_share:0.3
//! cargo axa task journeys:seed pathway:<pid> defects:all
//! cargo axa task journeys:seed pathway:<pid> defects:no_segments,steps_out_of_order
//! ```
//!
//! **Never real data, never derived from real data.** Every generated
//! `subject_ref` / care-team `member_ref` / `location_ref` carries a
//! fixed, recognizably-fake byte prefix (see
//! `crate::data::journeys`'s module docs) — the same discipline
//! IPPA-data's own disclaimer names: this data answers no
//! epidemiological question and must never be mistaken for one that
//! does.

use axa::prelude::*;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::data::journeys::{self, SeedParams};
use crate::models::care_pathways::Model as PathwayModel;
use crate::models::instance_segments::NewSegment;
use crate::models::pathway_instances::NewInstance;
use crate::models::{
    instance_events, instance_segments, instance_steps, instance_team, pathway_instances,
};

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
/// instance's `pid`. Takes the caller's connection, so the caller
/// controls the transaction boundary.
async fn insert_instance(
    db: &mut PgConnection,
    pathway_pid: Uuid,
    generated: &journeys::GeneratedInstance,
) -> Result<Uuid> {
    let instance = pathway_instances::Model::insert(
        &mut *db,
        &NewInstance {
            pathway_pid,
            subject_ref: &generated.subject_ref,
            status: &generated.status,
            urgency: &generated.urgency,
            enrolled_on: generated.enrolled_on,
            next_review_on: None,
            closed_on: generated.closed_on,
            closure_reason: None,
            outcome: generated.outcome.as_deref(),
            clock_start_at: generated.clock_start_at.map(Into::into),
            clock_stop_at: generated.clock_stop_at.map(Into::into),
        },
    )
    .await?;

    for member in &generated.team {
        instance_team::Model::insert(&mut *db, instance.pid, &member.member_ref, &member.role)
            .await?;
    }
    for (position, segment) in generated.segments.iter().enumerate() {
        instance_segments::Model::insert(
            &mut *db,
            &NewSegment {
                instance_pid: instance.pid,
                label: &segment.label,
                stage: &segment.stage,
                category: &segment.category,
                waste: segment.waste.as_deref(),
                started_at: segment.started_at.into(),
                ended_at: segment.ended_at.map(Into::into),
                actor_ref: segment.actor_ref.as_deref(),
                location_ref: segment.location_ref.as_deref(),
                note: None,
                position: i32::try_from(position).unwrap_or(i32::MAX),
            },
        )
        .await?;
    }
    for (position, step) in generated.steps.iter().enumerate() {
        instance_steps::Model::insert(
            &mut *db,
            instance.pid,
            &step.label,
            step.done,
            step.done_on,
            i32::try_from(position).unwrap_or(i32::MAX),
        )
        .await?;
    }
    for event in &generated.events {
        instance_events::Model::insert(
            &mut *db,
            instance.pid,
            &event.kind,
            event.occurred_at.into(),
            None,
            Some("journeys:seed"),
        )
        .await?;
    }
    Ok(instance.pid)
}

/// The `journeys:seed` CLI task.
pub struct JourneysSeed;

#[async_trait]
impl Task for JourneysSeed {
    /// It loads made-up records, which must never reach a real database,
    /// so axa refuses to run it outside development and test.
    fn development_only(&self) -> bool {
        true
    }

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

        let mut txn = ctx.db.begin().await?;
        let mut defect_pids: Vec<(String, Uuid)> = Vec::new();
        for generated in &cohort.instances {
            let instance_pid = insert_instance(&mut txn, template.pid, generated).await?;
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

#[cfg(test)]
mod development_only {
    use axa::task::Task;

    /// Made-up records must not reach a real database.
    #[test]
    fn it_runs_only_in_development_or_test() {
        assert!(super::JourneysSeed.development_only());
    }
}
