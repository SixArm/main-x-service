## 1. Purpose and Vision

### 1.1 Purpose

The Thing Service is a **generic registry** for arbitrary discrete
objects — books, papers, software, digital assets, devices, products,
instances of any physical or virtual object. The domain model is
faithful to [schema.org/Thing](https://schema.org/Thing) and is the
most general entity in the Main X Index family: anything that does not
fit one of the more opinionated sibling crates (`person`, `worker`,
`event`, `place`) belongs here.

### 1.2 Vision

A stable identity for any "thing" with:

- Typed identifiers (DOI, ISBN, ISSN, GTIN, SKU, MPN, SerialNumber,
  URI, UUID, Custom) drawn from
  [schema.org/PropertyValue](https://schema.org/PropertyValue).
- Probabilistic + deterministic matching by name, identifier,
  description, URL, and `sameAs` cross-reference.
- Real-time and batch duplicate detection with auto-merge for
  high-confidence cases.
- Stable cross-system Thing IDs so downstream systems converge on a
  single ID per real-world object.
- Audit logs and an event stream covering every CRUD / merge / link.

### 1.3 Non-goals

- **Not** an inventory system — we record identity, not stock or location.
- **Not** a catalogue manager — we hold canonical properties, not
  marketing copy or pricing.
- **Not** a recommendation engine — `same_as` and `additional_type`
  give downstream systems the hooks they need.
- **Not** an authentication / authorisation provider — the central
  authentication-service owns identity; this service only verifies its
  PASETO v4.public tokens offline (blanket enforcement is planned,
  §15) and identity proofing is out of scope.

