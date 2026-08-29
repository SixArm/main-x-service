# Batch

## What is a batch?

A **batch** is a *transient operational grouping* of multiple folders
and/or volumes — possibly belonging to **many different patients** — that
are physically handled together in a **single bulk action**. The classic
example is a trolley-load of case notes wheeled between departments, or a
stack of folders pulled for a clinic list, scanned, moved, or transferred
in one go.

A batch exists only for the duration of the action: it groups records so
one operation can touch them all at once, then it dissolves. It is **not**
a lasting property of any folder.

## Batch vs volume

A batch is easy to confuse with a [volume](volume.md); they are different:

| | **Volume** | **Batch** |
| --- | --- | --- |
| Kind         | domain grouping            | operational grouping            |
| Patients     | exactly **one** patient    | **any number** of patients      |
| Members      | folders                    | folders and/or whole volumes    |
| Lifetime     | persists over time         | transient — one action, then gone |
| Identity     | a stored `Volume` record   | usually no stored record (a selection) |

In short: a **volume** is *whose folders these are* (one patient's case
file); a **batch** is *which folders are being handled right now, together*.

## Status — proposed, not yet modelled

The tracker does **not** currently model batches. Per
[domain-model invariant 8](domain-model.md), _"moving a volume is the only
operation that moves a group of folders at once."_ A batch (a multi-patient
bulk action) would be a **new** capability beyond that invariant, so it is a
roadmap concept rather than an implemented one.

If/when added, a batch operation MUST preserve the existing audit guarantees:

- It appends **one move event per folder** (the per-folder audit trail
  stays append-only and complete — never a single coarse "batch" event).
- Each event keeps its own snapshot (patient, NHS Number, folder title,
  cabinet labels, worker) at action time.
- A batch does **not** change volume membership: moving a batch that happens
  to include a volume's folders does not merge, split, or reassign that
  volume; it only records the moves.

Adding batches would therefore relax invariant 8 (introduce a second
group-move operation). It is listed as an
[idea-stage entry in the roadmap](roadmap.md#idea-stage--not-designed-not-queued) —
not a queued task, and not a current behaviour — until it has a
`requirements.md` entry and a `design.md` decision.
