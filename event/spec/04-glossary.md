## 4. Glossary

Entity-level terms; per-crate glossaries are in the subproject specs.

| Term | Meaning |
|---|---|
| **Event** (capital E) | The domain entity — a time-bounded occurrence with parties + (optional) location + offers, aligned with schema.org/Event |
| **event stream / index-level events** | The CRUD-change records (`Created`, `Updated`, `Deleted`, `Merged`, `Linked`, `Unlinked`) the service publishes when registry records change. **Not** the domain Events themselves — an unfortunate but unavoidable name collision in this entity |
| **The trio** | event-service-rust-crate + event-matcher-rust-crate + event-front-end-with-svelte |
| **Service** | event-service-rust-crate — the system of record |
| **Matcher** | event-matcher-rust-crate — the canonical pairwise-comparison library |
| **Front-end** | event-front-end-with-svelte — the operator UI |
| **Adapter / bridge** | `src/matching/adapter.rs` in the service: `to_matcher_event` projects a service `Event` into the matcher's `Event` shape (§5.3) |
| **Bridge tests** | `tests/duplicate_detection.rs` in the service — pins both sides of the service ↔ matcher contract |
| **Time window** | `[start_date, end_date)`; `start_date` required, UTC; `end_date` optional (open-ended) |
| **Window overlap** | Jaccard ratio of two `[start, end)` intervals — the time-window-specific match component |
| **Location** (service) | Union of `Place` / `PostalAddress` / `VirtualLocation` / `Text`; an Event carries `Vec<Location>` |
| **Location** (matcher) | A single flat struct (venue name + address + lat/lon + virtual URL); the adapter dispatches the first populated service variant into it |
| **Party** | Typed reference (`Person` or `Organization`) with name, optional external ID, email, URL — personal data when it names a person |
| **Offer** | Pricing tier (price, ISO 4217 currency, availability, validity window) — descriptive, not transactional |
| **Strong identifier** | `BookingNumber`, `ConfirmationCode`, `TicketNumber`, `EncounterId`, `TransactionId` — in-service match short-circuits to 1.0 on exact match |
| **EventIdScheme** | Matcher-side external-ID scheme (Eventbrite, Meetup, Ticketmaster, Songkick, Bandsintown, Facebook, Luma, Wikidata, Google Calendar, iCalendar UID) |
| **Match quality** | Service buckets: Definite / Probable / Possible / Unlikely. Matcher bands: High / Medium / Low confidence. Distinct scales — do not conflate |
| **Soft delete** | `active = false`; rows are never `DELETE`d — the only delete in this entity |
| **Operator** | A registry user working through the front-end |
