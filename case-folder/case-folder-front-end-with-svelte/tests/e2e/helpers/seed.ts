// Names of records created by `cargo run -- task seed` in the
// Loco subproject. Mirroring them here lets tests assert presence
// without having to query the API for IDs first.
//
// See ../../../case-folder-service-with-rust/src/tasks/seed.rs.

export const SEED = {
    buildings: ['Main Hospital', 'Outpatients Wing', 'Off-site Archive'] as const,
    rooms: [
        'Ward A Records Room',
        'Ward B Records Room',
        'Outpatients Reception',
        'Long-term Archive Basement'
    ] as const,
    cabinets: ['Cabinet A1', 'Cabinet A2', 'Cabinet B1', 'Cabinet C1', 'Archive Cabinet 1'] as const,
    patients: {
        alice: { nhs: '943 476 5919', name: 'Alice Johnson' },
        bob: { nhs: '987 654 3210', name: 'Bob Smith' },
        carol: { nhs: '999 999 9999', name: 'Carol Williams' },
        david: { nhs: '614 309 0432', name: 'David Brown' },
        eleanor: { nhs: '630 162 4483', name: 'Eleanor Patel' },
        frank: { nhs: '485 777 3457', name: 'Frank O\u2019Connor' }
    },
    folders: {
        aliceVolume1: 'Volume 1 — General',
        aliceMaternity: 'Maternity 2023',
        bobCardiology: 'Cardiology 2019',
        bobArchived: 'General — Archived',
        carolGeneral: 'General',
        davidGeneral: 'General',
        davidOutpatients: 'Outpatients 2026',
        eleanorGeneral: 'General',
        frankGeneral: 'General'
    }
} as const;
