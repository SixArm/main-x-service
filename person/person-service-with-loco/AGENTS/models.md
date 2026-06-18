# Domain Model Reference

## Person

The central domain model. Represents a person identity record.

**File:** `src/models/person.rs`

| Field                 | Type                      | Description                           |
| --------------------- | ------------------------- | ------------------------------------- |
| id                    | Uuid                      | Unique person identifier             |
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
| deceased              | bool                      | Whether person is deceased           |
| deceased_datetime     | Option\<DateTime\<Utc\>\> | Date/time of death                    |
| addresses             | Vec\<Address\>            | Physical addresses                    |
| marital_status        | Option\<String\>          | Marital status                        |
| multiple_birth        | Option\<bool\>            | Multiple birth indicator              |
| photo                 | Vec\<String\>             | Photo references                      |
| managing_organization | Option\<Uuid\>            | Managing organization ID              |
| links                 | Vec\<PersonLink\>        | Links to other persons               |
| created_at            | DateTime\<Utc\>           | Creation timestamp                    |
| updated_at            | DateTime\<Utc\>           | Last update timestamp                 |

**Methods:**

- `Person::new(name, gender) -> Self` — Creates with UUID and timestamps
- `Person::full_name() -> String` — "Given Family" format
- `Person::effective_tax_id() -> Option<&str>` — tax_id or TAX-type identifier

## HumanName

**File:** `src/models/person.rs`

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

## PersonLink

**File:** `src/models/person.rs`

| Field            | Type     | Description                          |
| ---------------- | -------- | ------------------------------------ |
| other_person_id | Uuid     | Linked person ID                    |
| link_type        | LinkType | ReplacedBy, Replaces, Refer, Seealso |

## MergeRequest / MergeResponse / MergeRecord

**File:** `src/models/merge.rs`

**MergeRequest:** `main_person_id`, `duplicate_person_id`, `merge_reason`, `merged_by`

**MergeRecord:** `id`, `main_person_id`, `duplicate_person_id`, `status` (Completed/Reversed), `merged_by`, `merge_reason`, `match_score`, `transferred_data` (JSON), `merged_at`

**MergeResponse:** `merge_record`, `main_person` (merged result)

## ReviewQueueItem

**File:** `src/models/review_queue.rs`

| Field                    | Type             | Description                              |
| ------------------------ | ---------------- | ---------------------------------------- |
| id                       | Uuid             | Queue item ID                            |
| person_id_a             | Uuid             | First person                            |
| person_id_b             | Uuid             | Second person                           |
| match_score              | f64              | Similarity score                         |
| match_quality            | String           | certain/probable/possible                |
| detection_method         | String           | How detected                             |
| score_breakdown          | Option\<Value\>  | Per-component scores                     |
| status                   | ReviewStatus     | Pending, Confirmed, Rejected, AutoMerged |
| reviewed_by              | Option\<String\> | Reviewer                                 |
| created_at / reviewed_at | DateTime         | Timestamps                               |

**BatchDeduplicationRequest:** `threshold` (0.7), `max_candidates` (50), `auto_merge_threshold` (0.95)

**BatchDeduplicationResponse:** `persons_scanned`, `duplicates_found`, `auto_merged`, `queued_for_review`, `review_items`

## Consent

**File:** `src/models/consent.rs`

| Field        | Type                | Description                                                       |
| ------------ | ------------------- | ----------------------------------------------------------------- |
| id           | Uuid                | Consent record ID                                                 |
| person_id   | Uuid                | Person ID                                                        |
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

- `persons` — Core person table
- `person_names` — Person names (primary + additional)
- `person_identifiers` — External identifiers
- `person_addresses` — Physical addresses
- `person_contacts` — Contact points
- `person_links` — Person-to-person links
- `organizations` — Organization records
- `organization_addresses` — Organization addresses
- `organization_contacts` — Organization contacts
- `organization_identifiers` — Organization identifiers
- `person_match_scores` — Match score history
- `audit_log` — HIPAA-compliant audit trail
