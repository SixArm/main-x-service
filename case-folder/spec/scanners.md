# Scanner codes & tags

## What they are

Some case folders carry a **scanner code or tag** — a machine-readable label
or chip affixed to the physical folder so staff can *scan* or *read* it
instead of typing. Not every folder has one ("some case folders have scanner
codes"); where present, it lets a worker identify, locate, move, or receive
the folder quickly. Codes/tags come in two families: **optical** (read by
line-of-sight) and **wireless / radio** (read over the air).

## What a code or tag encodes

Whatever the technology, it can carry identifying information about the
folder, such as:

- the **folder name / title**,
- the **folder id** (the tracker's opaque identifier),
- the **patient / NHS Number** reference,
- a description of the **folder contents**,
- …and so on.

The tracker **keys on a stable identifier** (folder id and/or NHS Number);
any other encoded data (name, contents) is auxiliary / human-readable. NHS
Numbers are validated with the [Modulus 11 rules](nhs-number.md), and every
decoded value is treated as **untrusted input** regardless of how it was
read.

## Technologies

### Optical codes (line-of-sight)

| Format       | Carries                                                        |
| ------------ | ------------------------------------------------------------- |
| **Barcode** (1D, e.g. Code 128) | A single string — typically the folder id or NHS Number. |
| **QR code** (2D)                | More data — an id, or a small structured payload (id + name + contents pointer + URL). |

### Wireless / radio tags (read over the air)

| Technology   | Typical range | Carries / use                                                 |
| ------------ | ------------- | ------------------------------------------------------------- |
| **RFID** (passive UHF/HF) | cm–m  | A tag id (EPC/UID); read in bulk without line of sight — e.g. a whole shelf or trolley at once. |
| **NFC** (a close-range RFID) | < ~4 cm | A tag id / small record; tap-to-read with a phone or reader. |
| **Bluetooth / BLE** (beacon) | m–tens of m | A beacon id broadcast periodically; supports proximity / zone presence rather than a single precise scan. |
| **…others** | — | Any reader that yields a decoded identifier (the model is technology-agnostic). |

A wireless tag's read still resolves to the same thing an optical scan does:
**a decoded identifier** the tracker maps to a folder. The differences are
operational — no line of sight, bulk reads, and (for BLE) zone-level
proximity instead of a point scan.

## How they're used

- **Scan-to-move** ([requirements FR-22 / UC-I3](requirements.md),
  "Scan4Safety"): a scan/read yields the folder's NHS Number / folder id,
  which routes straight to that folder's move form.
- A **per-cabinet QR code** that opens the move workflow pre-filled is on the
  [roadmap](roadmap.md) (P2).
- The same read could drive [receive-it](receive-it.md) (scan-to-receive) and
  surface a folder someone has [tagged](tag-it.md). Bulk RFID reads pair
  naturally with a [batch](batch.md) action; BLE proximity could auto-update
  a folder's zone.

## Scope & status

- **Today (demo):** reads arrive as a **decoded identifier** via keyboard /
  wedge input (the reader "types" the value) — **no hardware integration is
  required** (AC-I3). This already works for any technology that can emit a
  string to the keyboard buffer.
- **Roadmap, not built in the demo:** dedicated capture hardware — handheld
  barcode/RFID/NFC readers, fixed RFID portals, BLE beacons/gateways, and the
  bulk/proximity workflows they enable — is a roadmap concern. The demo does
  **not** integrate readers, beacons, GPS, or fixed sensors (see `design.md`
  and [requirements §scope](requirements.md)); this file specifies the model
  those integrations would feed, not the integrations themselves.
- **Not yet specified:** generating / printing / encoding the codes and tags,
  the canonical identifier to encode (folder id vs NHS Number), and any
  richer QR / NDEF payload schema.

## Open questions (TODO)

- Which identifier is canonical inside the code/tag — folder id or NHS Number?
- For a QR code or NFC NDEF record with a richer payload (name, contents),
  what is the schema?
- How are **bulk RFID reads** (many tags at once) and **BLE proximity**
  events modelled — one event per folder? a zone update vs a precise move?
- Is code/tag **generation / encoding** in scope, or are they produced
  upstream?
- How is a read of an unknown / malformed / duplicate code handled and
  surfaced?
