# Domain model reference — Course Service

Based on [schema.org/Course](https://schema.org/Course) and its
parent types `LearningResource`, `CreativeWork`, `Thing`. The
service exposes only the properties we actively model — the full
schema.org surface is enormous and most properties are out of MVP
scope.

## Course

`src/models/course.rs`

Core entity. The *template* — `CourseInstance` represents specific
offerings.

### Thing-derived properties

| Field | Rust type | schema.org | Purpose |
|---|---|---|---|
| `id` | `Uuid` | — | System-generated identifier |
| `name` | `String` | `name` | Course title (required) |
| `alternate_names` | `Vec<String>` | `alternateName` | Aliases |
| `description` | `Option<String>` | `description` | Long-form description |
| `disambiguating_description` | `Option<String>` | `disambiguatingDescription` | Short distinguishing description |
| `url` | `Option<String>` | `url` | Canonical URL |
| `image` | `Vec<String>` | `image` | Image URLs |
| `same_as` | `Vec<String>` | `sameAs` | Cross-system identity URLs (Wikidata, OER repos) |
| `keywords` | `Vec<String>` | `keywords` | Tags |
| `identifiers` | `Vec<CourseIdentifier>` | `identifier` | Typed external identifiers |
| `additional_type` | `Option<String>` | `additionalType` | More specific schema.org subtype |
| `active` | `bool` | — | Soft-delete / hidden flag |

### CreativeWork-derived properties

| Field | Rust type | schema.org |
|---|---|---|
| `about` | `Vec<String>` | `about` |
| `audience` | `Option<String>` | `audience` |
| `in_language` | `Vec<String>` | `inLanguage` |
| `license` | `Option<String>` | `license` |
| `typical_age_range` | `Option<String>` | `typicalAgeRange` |
| `time_required` | `Option<String>` | `timeRequired` (ISO 8601) |
| `version` | `Option<String>` | `version` |
| `is_accessible_for_free` | `Option<bool>` | `isAccessibleForFree` |

### LearningResource-derived properties

| Field | Rust type | schema.org |
|---|---|---|
| `teaches` | `Vec<String>` | `teaches` |
| `assesses` | `Vec<String>` | `assesses` |
| `competency_required` | `Vec<String>` | `competencyRequired` |
| `educational_level` | `Option<EducationalLevel>` | `educationalLevel` |
| `educational_use` | `Option<String>` | `educationalUse` |
| `learning_resource_type` | `Option<LearningResourceType>` | `learningResourceType` |
| `interactivity_type` | `Option<InteractivityType>` | `interactivityType` |

### Course-specific properties

| Field | Rust type | schema.org |
|---|---|---|
| `course_code` | `Option<String>` | `courseCode` |
| `number_of_credits` | `Option<u32>` | `numberOfCredits` |
| `course_prerequisites` | `Vec<String>` | `coursePrerequisites` |
| `available_language` | `Vec<String>` | `availableLanguage` |
| `financial_aid_eligible` | `Vec<String>` | `financialAidEligible` |
| `educational_credential_awarded` | `Option<EducationalCredential>` | `educationalCredentialAwarded` |
| `occupational_credential_awarded` | `Option<EducationalCredential>` | `occupationalCredentialAwarded` |
| `total_historical_enrollment` | `Option<u64>` | `totalHistoricalEnrollment` |
| `syllabus_sections` | `Vec<Syllabus>` | `syllabusSections` |
| `instances` | `Vec<CourseInstance>` | `hasCourseInstance` |

### Registry-internal

| Field | Rust type | Purpose |
|---|---|---|
| `status` | `CourseStatus` | Draft / Published / Archived / Retired |
| `links` | `Vec<CourseLink>` | Course-to-course cross-references |
| `provider_id` | `Option<Uuid>` | FK to the issuing organisation |
| `deleted_at` | `Option<DateTime<Utc>>` | Soft-delete timestamp |
| `created_at` / `updated_at` | `DateTime<Utc>` | Standard audit timestamps |

### Enums

- `CourseStatus` — `Draft`, `Published` (default), `Archived`, `Retired`.
- `EducationalLevel` — `Beginner`, `Intermediate`, `Advanced`, `Expert`, `PrimaryEducation`, `SecondaryEducation`, `HigherEducation`, `Undergraduate`, `Graduate`, `Postgraduate`, `Vocational`, `ProfessionalDevelopment`, `Custom(String)`.
- `LearningResourceType` — `Lecture`, `Tutorial`, `Workshop`, `Assignment`, `Reading`, `Video`, `Audio`, `Exam`, `Simulation`, `Project`, `Discussion`, `Custom(String)`.
- `InteractivityType` — `Active`, `Expositive`, `Mixed`.
- `LinkType` — `Replaces`, `ReplacedBy`, `Seealso`, `Prerequisite`, `Successor`.

## CourseInstance

`src/models/course_instance.rs`

A specific offering — schema.org/CourseInstance. Multiple instances
per parent Course (`course_id`).

| Field | Rust type | Purpose |
|---|---|---|
| `id` | `Uuid` | System-generated |
| `course_id` | `Uuid` | FK to Course |
| `name` | `Option<String>` | Override of parent name (e.g. "CS101 — Fall 2026") |
| `course_mode` | `Option<CourseMode>` | `Online` / `Onsite` / `Blended` / `SelfPaced` |
| `status` | `CourseInstanceStatus` | `Scheduled` (default) / `EnrollmentOpen` / `EnrollmentClosed` / `InProgress` / `Completed` / `Cancelled` |
| `schedule` | `Option<Schedule>` | Window + sessions |
| `in_language` | `Vec<String>` | Language for this instance |
| `location` / `location_id` | free-text or FK | Physical / virtual location |
| `instructor_ids` / `instructor_names` | external IDs or free-text | Instructors |
| `maximum_attendee_capacity` / `enrolled_count` | `Option<u32>` | Capacity tracking |
| `enrollment_opens` / `enrollment_closes` | `Option<DateTime<Utc>>` | Enrollment window |

`Schedule` contains `start_date`, `end_date`, `time_zone`,
`recurrence` (ISO 8601 RRULE), and a `sessions: Vec<Session>` list
for non-uniform cadence.

## CourseIdentifier

`src/models/identifier.rs` — schema.org/PropertyValue shape.

| Field | Rust type |
|---|---|
| `property_id` | `IdentifierType` |
| `value` | `String` |
| `name` | `Option<String>` |
| `url` | `Option<String>` |

`IdentifierType` variants:

| Variant | Use | Deterministic? |
|---|---|---|
| `LmsCourseId` | Canvas / Moodle / Blackboard | no (scoped to LMS instance, not the value) |
| `CourseCode` | Provider catalog code (e.g. `CS101`) | no |
| `PlatformSlug` | Coursera / edX / Udemy slug | no |
| `Oer` | OER repository ID | **yes** |
| `Doi` | DOI | **yes** |
| `Lom` | IEEE LOM ID | **yes** |
| `Wikidata` | Wikidata Q-id | **yes** |
| `Isced` | ISCED programme code | no |
| `Ror` | ROR ID (provider-scoped) | no |
| `Uri` | URI / URN | **yes** |
| `Uuid` | UUID | **yes** |
| `Custom(String)` | Free-form scheme | no |

`IdentifierType::is_deterministic()` exposes the **yes** rows; the
matcher short-circuits scoring to `1.0` when both records share a
deterministic identifier.

## Provider

`src/models/organization.rs`

| Field | Rust type |
|---|---|
| `id` | `Uuid` |
| `name` | `String` |
| `alternate_names` | `Vec<String>` |
| `url` | `Option<String>` |
| `same_as` | `Vec<String>` |
| `kind` | `Option<ProviderKind>` |

`ProviderKind` — `University`, `College`, `School`, `OnlinePlatform`,
`Bootcamp`, `CertificationBody`, `GovernmentAgency`, `Other(String)`.

## Syllabus

`src/models/syllabus.rs` — schema.org/Syllabus, hierarchical.

| Field | Rust type |
|---|---|
| `id` | `Uuid` |
| `name` | `String` |
| `description` | `Option<String>` |
| `position` | `Option<u32>` |
| `teaches` | `Vec<String>` |
| `time_required` | `Option<String>` (ISO 8601 duration) |
| `resources` | `Vec<String>` (URLs) |
| `sub_sections` | `Vec<Syllabus>` |

## EducationalCredential

`src/models/credential.rs` — schema.org/EducationalOccupationalCredential.

| Field | Rust type |
|---|---|
| `name` | `String` |
| `category` | `Option<CredentialCategory>` |
| `educational_level` | `Option<String>` |
| `recognized_by` | `Option<String>` |
| `url` | `Option<String>` |

`CredentialCategory` — `Certificate`, `Diploma`, `Degree`, `Badge`,
`Microcredential`, `License`, `Custom(String)`.

## Merge / dedup

`src/models/merge.rs` + `src/models/review_queue.rs`. Same shape
as the sibling services: `MergeRequest` /
`MergeRecord` / `MergeResponse`; `ReviewQueueItem` (`Pending`,
`Confirmed`, `Rejected`, `AutoMerged`); `BatchDeduplicationRequest`
/ `BatchDeduplicationResponse`.

## Database mapping

SeaORM entity modules (in `src/db/models.rs`, planned T-2) map to
these tables:

- `providers` — issuing organisations.
- `courses` — Course (scalar fields + JSONB collections for
  `alternate_names`, `keywords`, `teaches`, `assesses`, …).
- `course_identifiers` — typed external identifiers.
- `course_links` — course-to-course cross-references.
- `course_instances` — specific offerings.
- `syllabus_sections` — hierarchical (parent_id self-FK).
- `course_match_scores` — match score history / review queue.
- `course_merge_records` — merge audit trail.
- `audit_log` — HIPAA / FERPA trail.

Migrations live in `migrations/` as numbered SQL `up.sql` /
`down.sql` pairs.

## Invariants

- `name` is required, non-empty after trim.
- `end_date >= start_date` when both set (on `CourseInstance.schedule`).
- `door_time` n/a (we don't model it; use `enrollment_opens`).
- `enrollment_closes >= enrollment_opens` when both set.
- `enrolled_count <= maximum_attendee_capacity` when both set.
- An `Identifier` is unique within `(course_id, property_id, value)`.
- `provider_id` references `providers(id)` when present.
