## 4. Glossary

| Term | Meaning |
|---|---|
| **Thing** | A discrete object — book, paper, software, device, product, asset |
| **Deterministic identifier** | DOI / ISBN / ISSN / GTIN / MPN / SerialNumber / UUID — globally unique by construction; match short-circuits to 1.0 |
| **Non-deterministic identifier** | SKU / URI / Custom — used as evidence, not as a hard pin |
| **PropertyValue** | The schema.org shape `{ propertyID, value, name?, url? }` |
| **Match quality** | Certain / Probable / Possible / Unlikely buckets keyed off configurable thresholds |
| **Soft delete** | `is_deleted = true`; rows are never `DELETE`d |

