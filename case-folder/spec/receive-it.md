# Receive It (confirm receipt)

> 🚧 **Status: TODO / draft.** This captures the idea; the data model, API,
> and UI are not yet designed or implemented.

## Idea

**Confirm receiving a case folder.** When a folder has been moved or sent
somewhere, the person who physically takes delivery confirms they have it —
closing the loop on the move and turning an `in-transit` folder into a known,
held location.

Typical case: a folder is moved out of a cabinet toward a department
(`in-transit`); when it arrives, the receiving worker scans / taps "Receive
it" to record that the folder is now in their hands (and, ideally, in which
cabinet or shelf it now sits).

## Sketch (to be designed)

A **receipt** would record, at minimum:

- **Which folder** — the case folder (and/or volume) received.
- **Who received it** — the receiving person (worker reference, snapshotted
  name + role per the audit conventions).
- **Where** — the destination leaf container it now occupies (cabinet or
  shelf), if known — see [places.md](places.md).
- **Received at** — the timestamp of confirmation.

## How it relates to moves and status

- A folder's status is **derived from the latest move event**: a destination
  cabinet → `in-cabinet`, none → `in-transit`
  ([domain-model invariant 5](domain-model.md)).
- "Receive it" is the acknowledgement that resolves an `in-transit` folder:
  it should record the destination and so flip the folder to `in-cabinet`.
- It MUST stay within the append-only audit model: confirming receipt
  appends an event (with its own snapshot of patient, NHS Number, folder
  title, container labels, worker) rather than mutating history.

## Open questions (TODO)

- Is a receipt a **move event** (a `to` location with no `from`, or pairing
  with the dispatch) or a **new event type**? It almost certainly reuses /
  extends the existing append-only move event.
- Does receiving **require** a destination container, or can a folder be
  "received, location TBD" (still effectively in-transit / in-hand)?
- Who may confirm — only the named recipient, anyone at the destination, or
  any authorised worker? (RBAC.)
- Exceptions: received the **wrong** folder, a folder that was **never
  dispatched**, or a dispatched folder that **never arrives** (overdue) —
  how are these surfaced and recorded?
- Tag-driven: does receipt interact with [tag-it.md](tag-it.md) (notify /
  fulfil the interest of whoever was waiting on the folder)?
- Bulk: can a [batch](batch.md) of folders be received in one confirmation
  (one receipt event per folder)?

## Next steps

1. Decide whether receipt is a move-event variant or a new event type.
2. Define whether a destination container is required on receipt.
3. Specify the UI affordance ("Receive it" / scan-to-receive) and the
   overdue / exception flows.
4. Write requirements + acceptance criteria and a tasks entry.
