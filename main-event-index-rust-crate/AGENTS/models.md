# Domain Model Reference

## Event

The central domain model. Represents a event identity record.

**File:** `src/models/event.rs`

| Field                 | Type                      | Description                           |
| --------------------- | ------------------------- | ------------------------------------- |
| id                    | Uuid                      | Unique event identifier               |
| identifiers           | Vec\<Identifier\>         | External identifiers (MRN, SSN, etc.) |
| active                | bool                      | Whether record is active              |
| name                  | HumanName                 | Primary name                          |
| additional_names      | Vec\<HumanName\>          | Aliases, former names                 |
| telecom               | Vec\<ContactPoint\>       | Phone, email, fax contacts            |
| gender                | Gender                    | Male, Female, Other, Unknown          |
| birth_date            | Option\<NaiveDate\>       | Date of birth                         |
| tax_id                | Option\<String\>          | Tax identifier (CPF, SSN, TIN)        |
| documents             | Vec\<IdentityDocument\>   | Identity documents                    |
| emergency_contacts    | Vec\<EmergencyContact\>   | Emergency contacts                    |
| deceased              | bool                      | Whether event is deceased             |
| deceased_datetime     | Option\<DateTime\<Utc\>\> | Date/time of death                    |
| addresses             | Vec\<Address\>            | Physical addresses                    |
| marital_status        | Option\<String\>          | Marital status                        |
| multiple_birth        | Option\<bool\>            | Multiple birth indicator              |
| photo                 | Vec\<String\>             | Photo references                      |
| managing_organization | Option\<Uuid\>            | Managing organization ID              |
| links                 | Vec\<EventLink\>          | Links to other events                 |
| created_at            | DateTime\<Utc\>           | Creation timestamp                    |
| updated_at            | DateTime\<Utc\>           | Last update timestamp                 |

**Methods:**

- `Event::new(name, gender) -> Self` — Creates with UUID and timestamps
- `Event::full_name() -> String` — "Given Family" format
- `Event::effective_tax_id() -> Option<&str>` — tax_id or TAX-type identifier

## HumanName

**File:** `src/models/event.rs`

| Field    | Type              | Description                                             |
| -------- | ----------------- | ------------------------------------------------------- |
| use_type | Option\<NameUse\> | Usual, Official, Temp, Nickname, Anonymous, Old, Maiden |
| family   | String            | Family/last name                                        |
| given    | Vec\<String\>     | Given/first names                                       |
| prefix   | Vec\<String\>     | Name prefixes (Dr., Mr.)                                |
| suffix   | Vec\<String\>     | Name suffixes (Jr., III)                                |

## Gender

**File:** `src/models/mod.rs`

Enum: `Male`, `Female`, `Other`, `Unknown`

## Address

**File:** `src/models/mod.rs`

| Field       | Type                 | Description                    |
| ----------- | -------------------- | ------------------------------ |
| use_type    | Option\<AddressUse\> | Home, Work, Temp, Old, Billing |
| line1       | Option\<String\>     | Street address line 1          |
| line2       | Option\<String\>     | Street address line 2          |
| city        | Option\<String\>     | City                           |
| state       | Option\<String\>     | State/province                 |
| postal_code | Option\<String\>     | Postal/ZIP code                |
| country     | Option\<String\>     | Country code                   |

## ContactPoint

**File:** `src/models/mod.rs`

| Field    | Type                      | Description                               |
| -------- | ------------------------- | ----------------------------------------- |
| system   | ContactPointSystem        | Phone, Fax, Email, Pager, Url, Sms, Other |
| value    | String                    | The contact value                         |
| use_type | Option\<ContactPointUse\> | Home, Work, Temp, Old, Mobile             |

## Identifier

**File:** `src/models/identifier.rs`

| Field           | Type                    | Description                           |
| --------------- | ----------------------- | ------------------------------------- |
| use_type        | Option\<IdentifierUse\> | Usual, Official, Temp, Secondary, Old |
| identifier_type | IdentifierType          | MRN, SSN, DL, NPI, PPN, TAX, Other    |
| system          | String                  | Identifier system URI                 |
| value           | String                  | Identifier value                      |
| assigner        | Option\<String\>        | Assigning authority                   |

**Factory Methods:** `Identifier::new()`, `Identifier::mrn()`, `Identifier::ssn()`

## IdentityDocument

**File:** `src/models/document.rs`

| Field             | Type                | Description                                                                                                     |
| ----------------- | ------------------- | --------------------------------------------------------------------------------------------------------------- |
| document_type     | DocumentType        | Passport, BirthCertificate, NationalId, DriversLicense, VoterId, MilitaryId, ResidencePermit, WorkPermit, Other |
| number            | String              | Document number                                                                                                 |
| issuing_country   | Option\<String\>    | Issuing country                                                                                                 |
| issuing_authority | Option\<String\>    | Issuing authority                                                                                               |
| issue_date        | Option\<NaiveDate\> | Issue date                                                                                                      |
| expiry_date       | Option\<NaiveDate\> | Expiry date                                                                                                     |
| verified          | bool                | Whether document is verified                                                                                    |

## EmergencyContact

**File:** `src/models/emergency_contact.rs`

| Field        | Type                | Description                         |
| ------------ | ------------------- | ----------------------------------- |
| name         | String              | Contact name                        |
| relationship | String              | Relationship (spouse, parent, etc.) |
| telecom      | Vec\<ContactPoint\> | Contact methods                     |
| address      | Option\<Address\>   | Contact address                     |
| is_primary   | bool                | Primary contact flag                |

## EventLink

**File:** `src/models/event.rs`

| Field          | Type     | Description                          |
| -------------- | -------- | ------------------------------------ |
| other_event_id | Uuid     | Linked event ID                      |
| link_type      | LinkType | ReplacedBy, Replaces, Refer, Seealso |

## MergeRequest / MergeResponse / MergeRecord

**File:** `src/models/merge.rs`

**MergeRequest:** `main_event_id`, `duplicate_event_id`, `merge_reason`, `merged_by`

**MergeRecord:** `id`, `main_event_id`, `duplicate_event_id`, `status` (Completed/Reversed), `merged_by`, `merge_reason`, `match_score`, `transferred_data` (JSON), `merged_at`

**MergeResponse:** `merge_record`, `main_event` (merged result)

## ReviewQueueItem

**File:** `src/models/review_queue.rs`

| Field                    | Type             | Description                              |
| ------------------------ | ---------------- | ---------------------------------------- |
| id                       | Uuid             | Queue item ID                            |
| event_id_a               | Uuid             | First event                              |
| event_id_b               | Uuid             | Second event                             |
| match_score              | f64              | Similarity score                         |
| match_quality            | String           | certain/probable/possible                |
| detection_method         | String           | How detected                             |
| score_breakdown          | Option\<Value\>  | Per-component scores                     |
| status                   | ReviewStatus     | Pending, Confirmed, Rejected, AutoMerged |
| reviewed_by              | Option\<String\> | Reviewer                                 |
| created_at / reviewed_at | DateTime         | Timestamps                               |

**BatchDeduplicationRequest:** `threshold` (0.7), `max_candidates` (50), `auto_merge_threshold` (0.95)

**BatchDeduplicationResponse:** `events_scanned`, `duplicates_found`, `auto_merged`, `queued_for_review`, `review_items`

## Consent

**File:** `src/models/consent.rs`

| Field        | Type                | Description                                                       |
| ------------ | ------------------- | ----------------------------------------------------------------- |
| id           | Uuid                | Consent record ID                                                 |
| event_id     | Uuid                | Event ID                                                          |
| consent_type | ConsentType         | DataProcessing, DataSharing, Marketing, Research, EmergencyAccess |
| status       | ConsentStatus       | Active, Revoked, Expired                                          |
| granted_date | NaiveDate           | When granted                                                      |
| expiry_date  | Option\<NaiveDate\> | When expires                                                      |
| revoked_date | Option\<NaiveDate\> | When revoked                                                      |
| purpose      | Option\<String\>    | Purpose description                                               |
| method       | Option\<String\>    | How obtained (written, electronic)                                |

## Organization

**File:** `src/models/organization.rs`

| Field       | Type                | Description              |
| ----------- | ------------------- | ------------------------ |
| id          | Uuid                | Organization ID          |
| identifiers | Vec\<Identifier\>   | Organization identifiers |
| active      | bool                | Active status            |
| org_type    | Vec\<String\>       | Organization types       |
| name        | String              | Organization name        |
| alias       | Vec\<String\>       | Alternative names        |
| telecom     | Vec\<ContactPoint\> | Contact points           |
| addresses   | Vec\<Address\>      | Physical addresses       |
| part_of     | Option\<Uuid\>      | Parent organization      |

## Database Models

**File:** `src/db/models.rs`

SeaORM entity modules for PostgreSQL persistence:

- `events` — Core event table
- `event_names` — Event names (primary + additional)
- `event_identifiers` — External identifiers
- `event_addresses` — Physical addresses
- `event_contacts` — Contact points
- `event_links` — Event-to-event links
- `organizations` — Organization records
- `organization_addresses` — Organization addresses
- `organization_contacts` — Organization contacts
- `organization_identifiers` — Organization identifiers
- `event_match_scores` — Match score history
- `audit_log` — HIPAA-compliant audit trail
