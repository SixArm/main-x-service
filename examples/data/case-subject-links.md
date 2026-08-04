# Case → person `subject_of` links

The `subject_of` edge that connects a case to the person it is about is
**not part of either JSONL file**, and deliberately so. It is a
cross-service link ([`cross-service-linking.md`](../../agents/share/cross-service-linking.md)
§4.1), written through the case service's own `entity_links` endpoint
after both records exist — not a field on the `Case` wire type.

That has a consequence this file exists to handle: **the link needs the
real UUIDs the two services minted at import time**, and those are not
knowable in advance. So this file is a mapping by *fixture line number*
plus the requests that turn it into real edges. It does not — and cannot
honestly — ship pre-baked UUIDs, because invented ones would name records
that do not exist and every request would 404.

For the same reason, the `subjects[]` array inside
[`cases.jsonl`](cases.jsonl) holds opaque agency-local labels
(`claimant-ref-0101`), **not** `person:<uuid>` URNs. `subjects` is a
free-text array on the case record; it is not the cross-service edge and
nothing resolves it.

## The mapping

Line numbers are 1-based into each JSONL file.

| Case line | Case | Person line | Person | Why this pairing |
|---:|---|---:|---|---|
| 1 | Housing benefit appeal — 14 Marlbrook Rise | 5 | Brackenridge, Nora Jean | the case title names her address |
| 2 | Employment tribunal claim — unfair dismissal | 2 | Vantongeren, Pieter | — |
| 3 | Safeguarding review — adult social care referral | 16 | Whitcombe, Ada | oldest person in the file (b. 1949) |
| 4 | Leave to remain application | 47 | Kowalczyk-Behn, Agata | she is the one person holding a `RESIDENCE_PERMIT` document |
| 5 | Council tax liability dispute — single occupancy | 6 | Thistlewood, Cormac | — |
| 6 | Premises licence application — Harlech Grove | 31 | Oyelowo, Femi | the case title names his address |
| 7 | Complaint about service standards | 11 | Yakubu, Halima | — |
| 8 | Legal aid eligibility determination | 10 | Ferreira-Lopes, Tiago | case carries `in_language: ["en","pt"]` |
| 9 | Benefits overpayment investigation | 8 | Wainscott, Delia Mae | the fullest person record — useful for a masking demo |
| 10 | Continuing healthcare funding appeal | 41 | Fitzwilliam, Rupert Alistair | case status `Withdrawn`; this person is `deceased` |

No person is the subject of two cases, and every case has exactly one
subject — so the resulting graph is a clean ten-edge fan-out, which is
what `GET /api/single-view/{ref}` on the link-graph aggregator is easiest
to demonstrate against.

## Creating one edge

`POST /api/cases/{case_pid}/links`, with the case service's
`LinkRequest` body (`src/controllers/links.rs`). Only `kind` and
`to_ref` are required:

```sh
curl -X POST "http://localhost:8089/api/cases/${CASE_PID}/links" \
  -H 'content-type: application/json' \
  -d '{"kind":"subject_of","to_ref":"person:'"${PERSON_PID}"'"}'
```

Response (`LinkView`):

```json
{
  "id": "70261eae-1d1a-4861-a8f9-55b207e10826",
  "from_ref": "case:8702f669-b54d-4d3b-ae58-72725d598d4f",
  "kind": "subject_of",
  "to_ref": "person:0c4f1e2a-1111-4111-8111-111111111111",
  "role": null,
  "confidence": null,
  "provenance": "operator",
  "valid_from": null,
  "valid_to": null
}
```

`provenance` defaults to `operator`; `role`, `confidence`, `valid_from`
and `valid_to` are optional (the dates are strict `YYYY-MM-DD`).

**From the case service, `subject_of` → `person:<uuid>` is the only
accepted combination.** `validate_edge` refuses everything else with
`422`: another edge kind (`same_identity`, `works_at`, `member_of`,
`employed_by`), a non-person target, or a malformed `EntityRef`. Both the
success and the `organization:` refusal above were exercised against a
running case service — see the EX-1 note in the repository root
[`tasks.md`](../../tasks.md).

Read the edges back with `GET /api/cases/{case_pid}/links`, and withdraw
one with `DELETE /api/cases/{case_pid}/links/{id}` (a soft delete that
emits `unlinked`).

> The case service is the reference implementation of the §10 governance
> rules for this edge: creating **and reading** it requires the
> authorisation to read the case, and both are audited. With
> `CASE_REQUIRE_AUTH` off — the shipped default — none of that is
> enforced, which is exactly the point of the enforced compose stack in
> [`examples/compose/enforced.yml`](../compose/enforced.yml).

## Resolving the pids

After importing both files, look each record up by a field you control.
Cases are easiest: rows 1–8 carry a stable `(agency_id, case_number)`
pair, so the case number is a unique handle.

```sh
CASE_BASE=http://localhost:8089
PERSON_BASE=http://localhost:8081

# Case line 1, by its case number.
CASE_PID=$(curl -s "${CASE_BASE}/api/cases/search?q=HB-2026-000101" \
  | jq -r '.[0].pid')

# Person line 5, by family name.
PERSON_PID=$(curl -s "${PERSON_BASE}/api/persons/search?q=Brackenridge" \
  | jq -r '.data[0].id')

curl -X POST "${CASE_BASE}/api/cases/${CASE_PID}/links" \
  -H 'content-type: application/json' \
  -d "{\"kind\":\"subject_of\",\"to_ref\":\"person:${PERSON_PID}\"}"
```

The two response shapes differ (the case service returns a bare array,
person wraps in `{success, data}`), which is why the `jq` paths are not
the same. Ports are the `full-family.yml` ones from
[`examples/compose/README.md`](../compose/README.md).

> **Verified vs. not.** The link request body, its response, the
> `422` on a non-person target, and the `GET`-back were all run against a
> live case service. The two `search` lookups in this last block were
> **not** run end-to-end here — they need both services up at once — so
> treat them as the shape to adapt, not a tested script. TUT-4 is where
> they get exercised for real.
