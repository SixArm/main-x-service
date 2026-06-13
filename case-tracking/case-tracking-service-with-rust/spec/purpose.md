# Purpose

> Part of the [Loco edition specification](index.md). Shared product
> purpose: [root purpose](../../spec/purpose.md).

Track the physical location of paper case-note folders in a UK NHS
hospital setting. Answer fast: _"Where is the folder for NHS Number
`XXX XXX XXXX` right now?"_ Maintain an immutable audit log of every
move.

This subproject exists to prove the **same domain** can be served by a
JSON-only back-end without changing the data model or the user
workflows. The HTML front-end and any progressive-enhancement behaviour
live in sibling projects that consume this API.

**Nothing domain-relevant lives in this app.** Every entity lives in one
of five external HTTP services — see [domain-model.md](domain-model.md).
