# Data modeling

Use SQL.

Every repeating domain collection became a child table with a FK (ON DELETE CASCADE) and an explicit position ordering column.

Polymorphic unions (event Location) and multi-role lists (event parties, course string-arrays) use a discriminator column rather than JSONB.

Enums are VARCHAR with CHECK constraints (matching the existing crates' style).

JSONB is only for genuinely opaque snapshots — audit_log.old_values/new_values, merge transferred_data, and review-queue score_breakdown — which aren't structured domain data.
