## 4. Glossary

| Term | Meaning |
|---|---|
| **Person** | The canonical record for an individual, modelled with HumanName, identifiers, addresses, documents, emergency contacts |
| **Identifier** | Typed external reference (`(identifier_type, system, value)`) e.g. MRN / SSN / NPI |
| **Tax ID** | Effective tax identifier (`tax_id` field or TAX-type entry in `identifiers`) |
| **Match** | A comparison between two persons yielding a 0.00–1.00 score plus per-component breakdown |
| **Match quality** | Definite / Probable / Possible / Unlikely buckets keyed off configurable thresholds |
| **Merge** | An operation that transfers a duplicate's data onto a surviving record, soft-deletes the duplicate, and writes a `Replaces` link |
| **Review queue** | A persisted set of candidate duplicate pairs, each `Pending` / `Confirmed` / `Rejected` / `AutoMerged` |
| **Soft delete** | Persistence-level retention with `active = false`; never `DELETE FROM` |

