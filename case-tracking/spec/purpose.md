# Purpose

> Part of the [Case Tracking specification](index.md).

## The problem

Many hospitals and clinics still manage vast amounts of **physical paper
charts**. A single patient may have several volumes of notes, and those
folders move constantly — between Records, Outpatients, wards, clinics,
and back. When a folder is needed and nobody knows where it physically
is, patient care is delayed and staff waste time searching.

## What this system does

Track the physical location of paper case-note folders so that anyone
can answer, quickly and reliably:

> _"Where is the folder for NHS Number `XXX XXX XXXX` right now?"_

and trust the answer because every move is recorded in an immutable
audit log.

### How it works

- Physical files are tagged with **barcodes, QR codes, or RFID** tags.
- When a folder moves between departments (e.g. Records → Outpatients),
  its tag is scanned, creating a digital trail of its exact location.
- The system records the move as an append-only **MoveEvent** and
  updates the folder's current cabinet pointer.

### Benefits

- Drastically cuts the time spent manually searching for missing
  records, preventing delays in patient care.
- Provides a complete, tamper-evident chain of custody for each folder.
- Surfaces aggregate state — cabinet utilisation, folders in transit,
  recent activity — for records-management staff.

## Who it serves

| Audience                | Need                                                       |
| ----------------------- | ---------------------------------------------------------- |
| Records / porter staff  | Find a folder, record a move, register a new folder         |
| Clinicians / admin      | Confirm a folder's whereabouts before a patient appointment |
| Records managers        | Cabinet utilisation, audit completeness, throughput         |
| Integrators             | A clean JSON API to build other front-ends against          |

## Non-goals

This is not an electronic health record. It tracks **where the paper
is**, not what is in it. See [scope.md](scope.md) for the full
in/out-of-scope boundary and [regulatory.md](regulatory.md) for why the
system is deliberately kept below SaMD (Software as a Medical Device)
classification.
