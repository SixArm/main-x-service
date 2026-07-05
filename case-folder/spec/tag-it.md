# Tag It (declare an interest)

> 🚧 **Status: TODO / draft.** This captures the idea; the data model, API,
> and UI are not yet designed or implemented.

## Idea

**A person has declared an interest in this case folder, with desired
dates.** "Tag it" lets someone register that they want a particular case
folder, and when they want it — so the system can record who is waiting on
a folder and route it to them when it becomes available.

Typical case: a clinician needs a patient's case notes for an upcoming
clinic or review, so they *tag* the folder with the date(s) they need it.

## Sketch (to be designed)

An **interest** would record, at minimum:

- **Who** — the person declaring the interest (typically a worker /
  clinician; an opaque worker reference, snapshotted name + role per the
  audit conventions).
- **Which folder** — the case folder (and/or volume) of interest.
- **Desired dates** — when the person wants the folder: a single date, a
  range (from / to), or a "by" deadline. TBD.
- **Declared at** — when the interest was registered.
- **Status** — e.g. open / fulfilled / withdrawn / expired. TBD.

## Open questions (TODO)

- What exactly are "desired dates" — a single date, a `[from, to]` range, a
  "needed by" deadline, or several?
- Can more than one person tag the same folder? If so, how are competing
  interests ordered / surfaced (a queue)?
- What happens when a folder a person tagged **moves** or becomes
  available — notify? auto-route? just surface in a list?
- Where does an interest live — a new tracker-owned entity, or an attribute
  on the folder? It is almost certainly tracker-owned (the upstream
  services do not model "interest").
- How does it interact with [moves](domain-model.md), [volumes](volume.md),
  and any future [batch](batch.md) handling?
- Audit + ABAC: who may declare, view, or withdraw an interest (which
  subject attributes gate it), and how is it logged? (Follow the
  append-only snapshot conventions.)

## Next steps

1. Pin down the "desired dates" shape and the interest lifecycle/status.
2. Decide the entity + API surface (tracker-owned).
3. Define the UI affordance ("Tag it" / "I want this folder") and where
   declared interests are listed.
4. Write requirements + acceptance criteria and a tasks entry.
