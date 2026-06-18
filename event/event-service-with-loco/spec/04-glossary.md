## 4. Glossary

| Term | Meaning |
|---|---|
| **Event** | A time-bounded occurrence with parties + (optional) location + offers |
| **Strong identifier** | `BookingNumber`, `ConfirmationCode`, `TicketNumber`, `EncounterId`, `TransactionId` — match short-circuits to 1.0 |
| **Location** | Union of `Place`, `PostalAddress`, `VirtualLocation`, `Text` |
| **Party** | Typed reference (`Person` or `Organization`) with name, optional external ID, email, URL |
| **Match quality** | Definite / Probable / Possible / Unlikely buckets keyed off configurable thresholds |
| **Window overlap** | Jaccard ratio of two `[start, end)` intervals |
| **Soft delete** | `active = false`; rows are never `DELETE`d |

