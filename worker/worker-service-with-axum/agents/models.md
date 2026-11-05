# Domain Model Reference

## Worker

The central domain model. Represents a worker identity record.

**File:** `src/models/worker.rs`

| Field                 | Type                      | Description                           |
| --------------------- | ------------------------- | ------------------------------------- |
| id                    | Uuid                      | Unique worker identifier              |
| identifiers           | Vec\<Identifier\>         | External identifiers (MRN, SSN, etc.) |
| active                | bool                      | Whether record is active              |
| name                  | HumanName                 | Primary name                          |
| additional_names      | Vec\<HumanName\>          | Aliases, former names                 |
| telecom               | Vec\<ContactPoint\>       | Phone, email, fax contacts            |
| gender                | Gender                    | Male, Female, Other, Unknown          |
| worker_type           | Option\<WorkerType\>      | Doctor, Nurse, Carer, Staff, Employee, Manager, Supervisor, Consultant, Other |
| birth_date            | Option\<NaiveDate\>       | Date of birth                         |
| tax_id                | Option\<String\>          | Tax identifier (CPF, SSN, TIN)        |
| documents             | Vec\<IdentityDocument\>   | Identity documents                    |
| emergency_contacts    | Vec\<EmergencyContact\>   | Emergency contacts                    |
| deceased              | bool                      | Whether worker is deceased            |
| deceased_datetime     | Option\<DateTime\<Utc\>\> | Date/time of death                    |
| addresses             | Vec\<Address\>            | Physical addresses                    |
| marital_status        | Option\<String\>          | Marital status                        |
| multiple_birth        | Option\<bool\>            | Multiple birth indicator              |
| photo                 | Vec\<String\>             | Photo references                      |
| managing_organization | Option\<Uuid\>            | Managing organization ID              |
| links                 | Vec\<WorkerLink\>         | Links to other workers                |
| created_at            | DateTime\<Utc\>           | Creation timestamp                    |
| updated_at            | DateTime\<Utc\>           | Last update timestamp                 |

**Methods:**

- `Worker::new(name, gender) -> Self` — Creates with UUID and timestamps
- `Worker::full_name() -> String` — "Given Family" format
- `Worker::effective_tax_id() -> Option<&str>` — tax_id or TAX-type identifier

## HumanName

**File:** `src/models/worker.rs`

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

## WorkerType

**File:** `src/models/worker.rs`

Enum: `Doctor`, `Nurse`, `Carer`, `Staff`, `Employee`, `Manager`,
`Supervisor`, `Consultant`, `Other`. Persisted, Tantivy-indexed
(searchable), and scrubbed by GDPR erasure alongside
`deceased_datetime` / `marital_status`.

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

## WorkerLink

**File:** `src/models/worker.rs`

| Field           | Type     | Description                          |
| --------------- | -------- | ------------------------------------ |
| other_worker_id | Uuid     | Linked worker ID                     |
| link_type       | LinkType | ReplacedBy, Replaces, Refer, Seealso |

## MergeRequest / MergeResponse / MergeRecord

**File:** `src/models/merge.rs`

**MergeRequest:** `main_worker_id`, `duplicate_worker_id`, `merge_reason`, `merged_by`

**MergeRecord:** `id`, `main_worker_id`, `duplicate_worker_id`, `status` (Completed/Reversed), `merged_by`, `merge_reason`, `match_score`, `transferred_data` (JSON), `merged_at`

**MergeResponse:** `merge_record`, `main_worker` (merged result)

## ReviewQueueItem

**File:** `src/models/review_queue.rs`

| Field                    | Type             | Description                              |
| ------------------------ | ---------------- | ---------------------------------------- |
| id                       | Uuid             | Queue item ID                            |
| worker_id_a              | Uuid             | First worker                             |
| worker_id_b              | Uuid             | Second worker                            |
| match_score              | f64              | Similarity score                         |
| match_quality            | String           | certain/probable/possible                |
| detection_method         | String           | How detected                             |
| score_breakdown          | Option\<Value\>  | Per-component scores                     |
| status                   | ReviewStatus     | Pending, Confirmed, Rejected, AutoMerged |
| reviewed_by              | Option\<String\> | Reviewer                                 |
| created_at / reviewed_at | DateTime         | Timestamps                               |

**BatchDeduplicationRequest:** `threshold` (0.7), `max_candidates` (50), `auto_merge_threshold` (0.95)

**BatchDeduplicationResponse:** `workers_scanned`, `duplicates_found`, `auto_merged`, `queued_for_review`, `review_items`

## Consent

**File:** `src/models/consent.rs`

| Field        | Type                | Description                                                       |
| ------------ | ------------------- | ----------------------------------------------------------------- |
| id           | Uuid                | Consent record ID                                                 |
| worker_id    | Uuid                | Worker ID                                                         |
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

## Assessment

Workforce assessment: one administration of one instrument to one worker.

**File:** `src/models/assessment.rs`

| Field           | Type                     | Description                                                     |
| --------------- | ------------------------ | --------------------------------------------------------------- |
| id              | Uuid                     | Assessment ID                                                    |
| worker_id       | Uuid                     | The assessed worker                                              |
| category        | AssessmentCategory       | Aptitude, Personality, Psychometric, Selection                   |
| instrument      | String                   | The named test (required)                                        |
| provider        | Option\<String\>         | Publisher / administering provider                               |
| status          | AssessmentStatus         | Scheduled, InProgress, Completed, Expired, Cancelled             |
| administered_on | Option\<NaiveDate\>      | When it was taken                                                |
| expires_on      | Option\<NaiveDate\>      | When results stop counting as current                            |
| administered_by | Option\<String\>         | Administering identity                                           |
| notes           | Option\<String\>         | Operator notes (redacted under `mask`)                           |
| results         | Vec\<AssessmentResult\>  | Per-scale outcomes                                               |
| created_at / updated_at | DateTime\<Utc\>  | Timestamps                                                       |

**Methods:**

- `Assessment::new(worker_id, category, instrument) -> Self` — scheduled, fresh UUID
- `Assessment::is_valid_on(date) -> bool` — completed and unexpired
- `Assessment::mean_percentile() -> Option<f64>` — real scores only
- `Assessment::masked() -> Self` — bands survive; scores / narratives / notes do not

### AssessmentCategory → AssessmentScale

| Category | Scales it owns |
| --- | --- |
| `aptitude` | `numerical_reasoning`, `verbal_reasoning`, `problem_solving`, `logical_thinking` |
| `personality` | `work_style`, `team_compatibility`, `introversion_extraversion` |
| `psychometric` | `behavioural_style`, `emotional_intelligence`, `cognitive_ability` |
| `selection` | `job_simulation`, `skills_assessment`, `judgement_test` |

`AssessmentCategory::permits(scale)` accepts a category's own scales —
and, for `psychometric` only, every aptitude and personality scale too
(a psychometric test covers both by definition). Anything else is a
`422` from `validate_assessment`.

### AssessmentResult

**File:** `src/models/assessment.rs`

| Field      | Type                 | Description                                  |
| ---------- | -------------------- | -------------------------------------------- |
| scale      | AssessmentScale      | The measured dimension                       |
| raw_score  | Option\<f64\>        | As reported by the instrument                |
| max_score  | Option\<f64\>        | Denominator for `raw_score`                  |
| percentile | Option\<f64\>        | Norm-referenced, `[0, 100]`                  |
| band       | Option\<ScoreBand\>  | Explicit band when the instrument reports one |
| narrative  | Option\<String\>     | Free-text interpretation                     |

`effective_band()` = the explicit `band`, else derived from
`percentile`. `ScoreBand::from_percentile`: `low` < 10, `below_average`
< 30, `average` < 70, `above_average` < 90, `high` ≥ 90.

### AssessmentStatus lifecycle

`scheduled → in_progress → completed → expired`; `cancelled` from any
open state; `scheduled → completed` directly (a test recorded after the
fact). `expired` / `cancelled` are terminal.
`AssessmentStatus::can_transition_to` is the machine the update handler
enforces.

## Database Models

**File:** `src/db/models.rs`

SeaORM entity modules for PostgreSQL persistence:

- `workers` — Core worker table
- `worker_names` — Worker names (primary + additional)
- `worker_identifiers` — External identifiers
- `worker_addresses` — Physical addresses
- `worker_contacts` — Contact points
- `worker_links` — Worker-to-worker links
- `organizations` — Organization records
- `organization_addresses` — Organization addresses
- `organization_contacts` — Organization contacts
- `organization_identifiers` — Organization identifiers
- `worker_match_scores` — Match score history
- `worker_assessments` — Workforce assessments (per-scale results as JSONB)
- `audit_log` — HIPAA-compliant audit trail
