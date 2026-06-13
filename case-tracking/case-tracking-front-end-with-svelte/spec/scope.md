# Scope

> Part of the [Svelte edition specification](index.md). Project-level
> scope: [root scope](../../spec/scope.md).

## In scope

- A **browser client** for every Loco API endpoint covered in the
  [Loco routes](../../case-tracker-service-with-rust/spec/routes.md).
- Dashboard with KPIs from `/api/stats`, recent moves, cabinet
  utilisation.
- Patient register + detail (`/patients`, `/patients/{nhs}`) including
  the fallback-to-snapshot view when the Main Patient Service has no
  record (the API surfaces `patient_service_match: false`).
- Folder register, detail, history, create (`/folders[/...]`).
- Building / room / cabinet management via the unified Place endpoints.
- Move-folder workflow with live NHS-Number lookup, worker picker,
  cabinet picker.
- Audit log with free-text filter (`/history` → `GET /api/moves?q=`).
- NHS Number client-side validation (Modulus 11) before the form
  submits — the API enforces it again.

## Explicitly out of scope

- **Any local persistence.** No seed data, no `localStorage`, no
  IndexedDB. Reload re-fetches from the API.
- A `/patients/new` route — `POST /api/patients` doesn't exist on the
  back-end (patient registration is a side effect of `POST /api/folders`).
- Authentication / RBAC — the demo runs unauthenticated. Production
  gates listed in [regulatory.md](regulatory.md).
- Authoring of clinical content.
- SSR. The app is client-only for now.
