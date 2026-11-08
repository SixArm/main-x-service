//! `cargo axa task seed` — create a synthetic demo hospital so the
//! whiteboards demo instantly. **Synthetic data only** (spec
//! `regulatory.md`): person/worker URNs are random UUIDs, names are
//! obviously fictional.

use axa::prelude::*;
use axa::task::{TaskInfo, Vars};
use uuid::Uuid;

use crate::flow::bed_state::BedState;
use crate::models::{bays, beds, sites, stays, transfers, wards};

/// The demo-hospital seed task.
pub struct Seed;

#[async_trait]
impl Task for Seed {
    /// It loads made-up records, which must never reach a real database,
    /// so axa refuses to run it outside development and test.
    fn development_only(&self) -> bool {
        true
    }

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
        sites::Model::insert(
            db,
            &sites::NewSite {
                pid: site_pid,
                name: "St Elsewhere General",
                place_ref: None,
                organization_ref: None,
            },
        )
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
            wards::Model::insert(
                db,
                &wards::NewWard {
                    pid: ward_pid,
                    site_pid,
                    name,
                    code,
                    kind,
                    specialty,
                    open: true,
                    escalation,
                    closed_to_admissions: false,
                    place_ref: None,
                },
            )
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
                bays::Model::insert(
                    db,
                    &bays::NewBay {
                        pid: bay_pid,
                        ward_pid,
                        name: bay_name,
                        sex_designation: sex,
                        side_room: *side_room,
                        closed_to_admissions: false,
                    },
                )
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
                    beds::Model::insert(
                        db,
                        &beds::NewBed {
                            pid: bed_pid,
                            bay_pid,
                            number: &format!("{code}-{bay_name}-{n}"),
                            state: if occupied {
                                BedState::Occupied
                            } else {
                                BedState::Available
                            }
                            .token(),
                            state_since: now,
                            isolation_capable: *side_room,
                            oxygen: !is_virtual,
                            bariatric: false,
                            is_virtual,
                        },
                    )
                    .await?;
                    if occupied {
                        let stay_pid = Uuid::new_v4();
                        let display_name = demo_names
                            .next()
                            .unwrap_or_else(|| "Test Patient".to_string());
                        let person_ref = format!("person:{}", Uuid::new_v4());
                        let named_nurse_ref = format!("worker:{}", Uuid::new_v4());
                        let consultant_ref = format!("worker:{}", Uuid::new_v4());
                        stays::Model::insert(
                            db,
                            &stays::NewStay {
                                pid: stay_pid,
                                person_ref: &person_ref,
                                display_name: &display_name,
                                status: "admitted",
                                admitted_at: now,
                                source: if is_virtual {
                                    "virtual_admission"
                                } else {
                                    "ed"
                                },
                                ward_pid: Some(ward_pid),
                                bed_pid: Some(bed_pid),
                                home_location_note: is_virtual.then_some("at home (synthetic)"),
                                named_nurse_ref: Some(&named_nurse_ref),
                                consultant_ref: Some(&consultant_ref),
                                edd: Some(
                                    now.date_naive()
                                        + chrono::Days::new(u64::try_from(n % 5).unwrap_or(0)),
                                ),
                                ccd: Some("clinically stable"),
                                alerts: serde_json::json!([]),
                            },
                        )
                        .await?;
                        transfers::Model::insert(
                            db,
                            &transfers::NewTransfer {
                                pid: Uuid::new_v4(),
                                stay_pid,
                                from_bed_pid: None,
                                to_bed_pid: Some(bed_pid),
                                reason: "admission",
                                moved_at: now,
                                moved_by_ref: None,
                            },
                        )
                        .await?;
                    }
                }
            }
        }
        tracing::info!("seeded the synthetic demo hospital");
        Ok(())
    }
}

#[cfg(test)]
mod development_only {
    use axa::task::Task;

    /// Made-up records must not reach a real database.
    #[test]
    fn it_runs_only_in_development_or_test() {
        assert!(super::Seed.development_only());
    }
}
