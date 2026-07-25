//! `cargo loco task seed` — create a synthetic demo hospital so the
//! whiteboards demo instantly. **Synthetic data only** (spec
//! `regulatory.md`): person/worker URNs are random UUIDs, names are
//! obviously fictional.

use loco_rs::prelude::*;
use loco_rs::task::{TaskInfo, Vars};
use sea_orm::ActiveValue;
use uuid::Uuid;

use crate::flow::bed_state::BedState;
use crate::models::_entities::{bays, beds, sites, stays, transfers, wards};

/// The demo-hospital seed task.
pub struct Seed;

#[async_trait]
impl Task for Seed {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "seed".to_string(),
            detail: "Seed a synthetic demo hospital (sites, wards, beds, stays)".to_string(),
        }
    }

    #[allow(clippy::too_many_lines)] // one linear walk building the demo estate
    async fn run(&self, ctx: &AppContext, _vars: &Vars) -> Result<()> {
        let db = &ctx.db;
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        let site_pid = Uuid::new_v4();
        sites::ActiveModel {
            pid: ActiveValue::set(site_pid),
            name: ActiveValue::set("St Elsewhere General".to_string()),
            place_ref: ActiveValue::set(None),
            organization_ref: ActiveValue::set(None),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(db)
        .await?;
        // (name, code, kind, specialty, escalation)
        let ward_defs = [
            (
                "Ward 7 — Respiratory",
                "W7",
                "inpatient",
                Some("respiratory"),
                false,
            ),
            (
                "Ward 8 — Orthopaedics",
                "W8",
                "inpatient",
                Some("orthopaedics"),
                false,
            ),
            ("Acute Assessment Unit", "AAU", "assessment", None, false),
            ("Escalation Ward", "ESC", "inpatient", None, true),
            (
                "Respiratory Virtual Ward",
                "VW1",
                "virtual",
                Some("respiratory"),
                false,
            ),
        ];
        let mut demo_names = (1..=200).map(|i| format!("Test Patient {i:03}"));
        for (name, code, kind, specialty, escalation) in ward_defs {
            let ward_pid = Uuid::new_v4();
            wards::ActiveModel {
                pid: ActiveValue::set(ward_pid),
                site_pid: ActiveValue::set(site_pid),
                name: ActiveValue::set(name.to_string()),
                code: ActiveValue::set(code.to_string()),
                kind: ActiveValue::set(kind.to_string()),
                specialty: ActiveValue::set(specialty.map(ToString::to_string)),
                open: ActiveValue::set(true),
                escalation: ActiveValue::set(escalation),
                closed_to_admissions: ActiveValue::set(false),
                place_ref: ActiveValue::set(None),
                deleted_at: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(db)
            .await?;
            let is_virtual = kind == "virtual";
            let bay_defs: &[(&str, &str, bool)] = if is_virtual {
                &[("Virtual", "flexible", false)]
            } else {
                &[
                    ("Bay A", "female", false),
                    ("Bay B", "male", false),
                    ("Side Room 1", "flexible", true),
                    ("Side Room 2", "flexible", true),
                ]
            };
            for (bay_name, sex, side_room) in bay_defs {
                let bay_pid = Uuid::new_v4();
                bays::ActiveModel {
                    pid: ActiveValue::set(bay_pid),
                    ward_pid: ActiveValue::set(ward_pid),
                    name: ActiveValue::set((*bay_name).to_string()),
                    sex_designation: ActiveValue::set((*sex).to_string()),
                    side_room: ActiveValue::set(*side_room),
                    closed_to_admissions: ActiveValue::set(false),
                    deleted_at: ActiveValue::set(None),
                    ..Default::default()
                }
                .insert(db)
                .await?;
                let bed_count = if *side_room {
                    1
                } else if is_virtual {
                    20
                } else {
                    6
                };
                for n in 1..=bed_count {
                    let bed_pid = Uuid::new_v4();
                    // Occupy roughly two-thirds of the beds.
                    let occupied = n % 3 != 0;
                    beds::ActiveModel {
                        pid: ActiveValue::set(bed_pid),
                        bay_pid: ActiveValue::set(bay_pid),
                        number: ActiveValue::set(format!("{code}-{bay_name}-{n}")),
                        state: ActiveValue::set(
                            if occupied {
                                BedState::Occupied
                            } else {
                                BedState::Available
                            }
                            .token()
                            .to_string(),
                        ),
                        state_since: ActiveValue::set(now),
                        closure_reason: ActiveValue::set(None),
                        deep_clean_required: ActiveValue::set(false),
                        isolation_capable: ActiveValue::set(*side_room),
                        oxygen: ActiveValue::set(!is_virtual),
                        bariatric: ActiveValue::set(false),
                        is_virtual: ActiveValue::set(is_virtual),
                        deleted_at: ActiveValue::set(None),
                        ..Default::default()
                    }
                    .insert(db)
                    .await?;
                    if occupied {
                        let stay_pid = Uuid::new_v4();
                        let display_name = demo_names
                            .next()
                            .unwrap_or_else(|| "Test Patient".to_string());
                        stays::ActiveModel {
                            pid: ActiveValue::set(stay_pid),
                            person_ref: ActiveValue::set(format!("person:{}", Uuid::new_v4())),
                            display_name: ActiveValue::set(display_name),
                            status: ActiveValue::set("admitted".to_string()),
                            admitted_at: ActiveValue::set(now),
                            source: ActiveValue::set(
                                if is_virtual {
                                    "virtual_admission"
                                } else {
                                    "ed"
                                }
                                .to_string(),
                            ),
                            ward_pid: ActiveValue::set(Some(ward_pid)),
                            bed_pid: ActiveValue::set(Some(bed_pid)),
                            home_location_note: ActiveValue::set(
                                is_virtual.then(|| "at home (synthetic)".to_string()),
                            ),
                            named_nurse_ref: ActiveValue::set(Some(format!(
                                "worker:{}",
                                Uuid::new_v4()
                            ))),
                            consultant_ref: ActiveValue::set(Some(format!(
                                "worker:{}",
                                Uuid::new_v4()
                            ))),
                            senior_review_at: ActiveValue::set(None),
                            edd: ActiveValue::set(Some(
                                now.date_naive()
                                    + chrono::Days::new(u64::try_from(n % 5).unwrap_or(0)),
                            )),
                            ccd: ActiveValue::set(Some("clinically stable".to_string())),
                            ccd_met: ActiveValue::set(false),
                            discharge_pathway: ActiveValue::set(None),
                            discharge_ready_at: ActiveValue::set(None),
                            discharged_at: ActiveValue::set(None),
                            discharge_destination: ActiveValue::set(None),
                            alerts: ActiveValue::set(serde_json::json!([])),
                            deleted_at: ActiveValue::set(None),
                            ..Default::default()
                        }
                        .insert(db)
                        .await?;
                        transfers::ActiveModel {
                            pid: ActiveValue::set(Uuid::new_v4()),
                            stay_pid: ActiveValue::set(stay_pid),
                            from_bed_pid: ActiveValue::set(None),
                            to_bed_pid: ActiveValue::set(Some(bed_pid)),
                            reason: ActiveValue::set("admission".to_string()),
                            moved_at: ActiveValue::set(now),
                            moved_by_ref: ActiveValue::set(None),
                            ..Default::default()
                        }
                        .insert(db)
                        .await?;
                    }
                }
            }
        }
        tracing::info!("seeded the synthetic demo hospital");
        Ok(())
    }
}
