# Bed management

## The bed state machine

Modelled on the HL7 v2 `bedStatus` vocabulary and standard
housekeeping turnaround practice, adapted to NHS ward operations:

```
                 ┌────────────────────────────────────────────┐
                 ▼                                            │
  ┌───────────┐ allocate ┌──────────┐  admit/transfer  ┌──────────┐
  │ available │────────▶ │ reserved │────────────────▶ │ occupied │
  └───────────┘          └──────────┘                  └──────────┘
        ▲  ▲                  │ release                      │ vacate
        │  │                  ▼                              ▼ (discharge/transfer-out)
        │  │             (available)                  ┌───────────────┐
        │  │                                          │ awaiting_clean│
        │  │ clean-complete   ┌──────────┐ clean-start└───────────────┘
        │  └──────────────────│ cleaning │◀───────────────────┘
        │                     └──────────┘
        │   reopen  ┌────────┐
        └───────────│ closed │◀── close (from any state except occupied)
                    └────────┘
```

Transitions (each is an API action, audited, evented, with
`state_since` reset):

| Transition | From → To | Trigger |
|---|---|---|
| `allocate` | available → reserved | bed-request allocation |
| `release` | reserved → available | allocation cancelled / expired |
| `admit` / `transfer-in` | available or reserved → occupied | stay placed in bed |
| `vacate` | occupied → awaiting_clean | discharge or transfer-out |
| `clean-start` | awaiting_clean → cleaning | domestic team starts |
| `clean-complete` | cleaning → available | inspected and ready |
| `close` | any non-occupied → closed | with `closure_reason` |
| `reopen` | closed → available (via awaiting_clean if `deep_clean_required`) | |

Invariants:

- A bed is `occupied` **iff** exactly one active stay's `bed_pid`
  points at it. The service enforces this transactionally.
- `vacate` sets `deep_clean_required = true` when the departing stay
  has an uncleared `contact`/`droplet`/`airborne` infection flag; a
  deep-clean-required bed returning from `cleaning` requires an
  explicit deep-clean completion, not the routine one.
- Illegal transitions return `422` with the current state named.
- Every transition records `state_since`, making **turnaround time**
  (vacate → available) a first-class metric ([capacity.md](capacity.md)).

## Closures

Beds close for `infection`, `maintenance`, `staffing`, or `other`
reasons. Bays and wards additionally carry `closed_to_admissions`
(outbreak control): their beds keep their individual states but the
allocator refuses them. Closed capacity is reported distinctly in
capacity views — a closed bed is neither supply nor demand.

## Allocation rules

Allocation (bed-request → bed) is rule-checked, not free-form. A bed
is **eligible** for a request when all hold:

1. Bed `state = available`; bay, ward, and site are open to
   admissions.
2. **Sex segregation**: the bay's `sex_designation` matches the
   patient's recorded sex, or the bay is `flexible`/`mixed`, or the
   bay is a side room. (Mixed-sex accommodation breaches are an NHS
   reportable event; the allocator prevents, an override records.)
3. **Isolation**: a request flagged `isolation`/`side_room` only
   matches side rooms or isolation-capable beds; conversely a
   protective-isolation bay refuses non-matching admissions.
4. **Equipment**: `oxygen` / `bariatric` requirements match bed
   attributes.
5. **Ward fit**: target ward matches, or specialty matches the ward's
   specialty; an explicit operator override is allowed and audited
   (`outlier` placement — a tracked quality metric).

The allocator returns eligible beds ranked (right ward first, then
side-room conservation — don't burn a side room on a patient who
doesn't need one). The operator picks; the system never auto-places
in v1. Overrides of rules 2 and 5 require an override reason and are
flagged in the audit row.
