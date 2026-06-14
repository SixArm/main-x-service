// TypeScript domain types mirroring the Rust Course Service models
// (schema.org/Course-aligned). Reference:
// course-service-rust-crate/AGENTS/models.md.

// ─── HTTP envelope ───────────────────────────────────────────────────

/**
 * Uniform success/error envelope wrapping every Course Service JSON
 * response. The service always returns this shape, so the client
 * can branch on `success` rather than HTTP status alone.
 *
 * @typeParam T - The payload type carried in `data` on success.
 */
export interface ApiResponse<T> {
    success: boolean;
    data: T | null;
    error: ApiErrorBody | null;
}

/**
 * Structured error body nested under {@link ApiResponse.error}. `code`
 * is a stable machine token (e.g. `NOT_FOUND`, `DUPLICATE`); `details`
 * is endpoint-specific (e.g. a `MatchResult[]` on a 409 duplicate
 * conflict).
 */
export interface ApiErrorBody {
    code: string;
    message: string;
    details?: unknown;
}

// ─── Identifier types ────────────────────────────────────────────────

/**
 * Tagged union of identifier schemes a course can carry (mirrors the
 * Rust `IdentifierType` enum). String variants are well-known schemes;
 * the `{ Custom: string }` object variant carries a free-text scheme
 * label for anything not enumerated. Serde serialises it the same way,
 * so this shape is the wire shape.
 */
export type IdentifierType =
    | "LmsCourseId"
    | "CourseCode"
    | "PlatformSlug"
    | "Oer"
    | "Doi"
    | "Lom"
    | "Wikidata"
    | "Isced"
    | "Ror"
    | "Uri"
    | "Uuid"
    | { Custom: string };

/**
 * Identifier schemes that, on an exact value match between two courses,
 * short-circuit the matcher to a deterministic score of 1.0 (per spec
 * §6 FR-20). Globally unique schemes (DOI, Wikidata, …) only — a shared
 * value is conclusive proof of the same course.
 */
export const DETERMINISTIC_TYPES: IdentifierType[] = [
    "Doi", "Wikidata", "Lom", "Oer", "Uri", "Uuid",
];

/**
 * Enumerated identifier schemes offered in the identifier-type
 * dropdown (excludes the `Custom` variant, which the UI surfaces as a
 * separate "Custom…" option with a free-text label).
 */
export const IDENTIFIER_TYPE_OPTIONS: Exclude<IdentifierType, { Custom: string }>[] = [
    "LmsCourseId", "CourseCode", "PlatformSlug", "Oer", "Doi", "Lom",
    "Wikidata", "Isced", "Ror", "Uri", "Uuid",
];

/**
 * A single identifier attached to a course: its scheme
 * (`property_id`), the `value`, plus an optional human-readable `name`
 * and a resolvable `url`. Mirrors schema.org/PropertyValue.
 */
export interface CourseIdentifier {
    property_id: IdentifierType;
    value: string;
    name?: string | null;
    url?: string | null;
}

/**
 * Construct an empty identifier row for the form to bind to. Defaults
 * to the `CourseCode` scheme with an empty value; `name`/`url` are
 * explicit nulls so the object round-trips cleanly through the wire.
 *
 * @returns A fresh, mutable {@link CourseIdentifier}.
 */
export function blankCourseIdentifier(): CourseIdentifier {
    return { property_id: "CourseCode", value: "", name: null, url: null };
}

// ─── Educational level / learning resource type ──────────────────────

/**
 * schema.org/educationalLevel as a tagged union: enumerated bands
 * (skill-based and academic-stage), plus a `{ Custom: string }` escape
 * hatch for institution-specific levels. Used as a matching component.
 */
export type EducationalLevel =
    | "Beginner" | "Intermediate" | "Advanced" | "Expert"
    | "PrimaryEducation" | "SecondaryEducation" | "HigherEducation"
    | "Undergraduate" | "Graduate" | "Postgraduate"
    | "Vocational" | "ProfessionalDevelopment"
    | { Custom: string };

/** Enumerated educational levels for the level dropdown (no `Custom`). */
export const EDUCATIONAL_LEVEL_OPTIONS: Exclude<EducationalLevel, { Custom: string }>[] = [
    "Beginner", "Intermediate", "Advanced", "Expert",
    "PrimaryEducation", "SecondaryEducation", "HigherEducation",
    "Undergraduate", "Graduate", "Postgraduate",
    "Vocational", "ProfessionalDevelopment",
];

/**
 * schema.org/learningResourceType — the pedagogical form of a resource
 * (lecture, lab, exam, …), with a `{ Custom: string }` escape hatch.
 */
export type LearningResourceType =
    | "Lecture" | "Tutorial" | "Workshop" | "Assignment" | "Reading"
    | "Video" | "Audio" | "Exam" | "Simulation" | "Project" | "Discussion"
    | { Custom: string };

/** Enumerated learning-resource types for dropdowns (no `Custom`). */
export const LEARNING_RESOURCE_TYPE_OPTIONS: Exclude<LearningResourceType, { Custom: string }>[] = [
    "Lecture", "Tutorial", "Workshop", "Assignment", "Reading",
    "Video", "Audio", "Exam", "Simulation", "Project", "Discussion",
];

/** schema.org/interactivityType — learner-vs-instructor activity balance. */
export type InteractivityType = "active" | "expositive" | "mixed";
/** All {@link InteractivityType} values, for dropdown population. */
export const INTERACTIVITY_TYPES: InteractivityType[] = ["active", "expositive", "mixed"];

/** Lifecycle status of a course template (distinct from per-instance status). */
export type CourseStatus = "draft" | "published" | "archived" | "retired";
/** All {@link CourseStatus} values, in lifecycle order, for the status dropdown. */
export const COURSE_STATUSES: CourseStatus[] = ["draft", "published", "archived", "retired"];

// ─── Educational credential ──────────────────────────────────────────

/** Category of credential a course can award, with a `Custom` escape hatch. */
export type CredentialCategory =
    | "Certificate" | "Diploma" | "Degree" | "Badge"
    | "Microcredential" | "License" | { Custom: string };

/**
 * A credential a course confers on completion (schema.org's
 * `educationalCredentialAwarded` / `occupationalCredentialAwarded`).
 * `recognized_by` names the awarding/accrediting body.
 */
export interface EducationalCredential {
    name: string;
    category?: CredentialCategory | null;
    educational_level?: string | null;
    recognized_by?: string | null;
    url?: string | null;
}

// ─── Course-to-course links ──────────────────────────────────────────

/**
 * Relationship between two courses. `replaces`/`replaced-by` model the
 * merge supersession chain; the rest are editorial cross-references.
 */
export type LinkType = "replaces" | "replaced-by" | "seealso" | "prerequisite" | "successor";

/** A typed edge from this course to another course by ID. */
export interface CourseLink {
    other_course_id: string;
    link_type: LinkType;
}

// ─── Syllabus ────────────────────────────────────────────────────────

/**
 * A syllabus section (schema.org/syllabusSections). Recursive via
 * `sub_sections` to model nested outlines; `position` orders siblings,
 * and `teaches` lists the competencies covered by this section.
 */
export interface Syllabus {
    id?: string;
    name: string;
    description?: string | null;
    position?: number | null;
    teaches?: string[];
    time_required?: string | null;
    resources?: string[];
    sub_sections?: Syllabus[];
}

// ─── CourseInstance ──────────────────────────────────────────────────

/** schema.org/courseMode — delivery mode of a specific course offering. */
export type CourseMode = "online" | "onsite" | "blended" | "self_paced";
/** All {@link CourseMode} values, for dropdown population. */
export const COURSE_MODES: CourseMode[] = ["online", "onsite", "blended", "self_paced"];

/** Lifecycle/enrollment status of a single {@link CourseInstance}. */
export type CourseInstanceStatus =
    | "scheduled" | "enrollment_open" | "enrollment_closed"
    | "in_progress" | "completed" | "cancelled";
/** All {@link CourseInstanceStatus} values, in lifecycle order. */
export const COURSE_INSTANCE_STATUSES: CourseInstanceStatus[] = [
    "scheduled", "enrollment_open", "enrollment_closed",
    "in_progress", "completed", "cancelled",
];

/** A single meeting/session within a {@link Schedule}; `start`/`end` are ISO-8601 timestamps. */
export interface Session {
    start: string;
    end?: string | null;
    label?: string | null;
}

/**
 * When a course instance runs: overall date range, time zone, an
 * optional `recurrence` rule, and the concrete list of `sessions`.
 */
export interface Schedule {
    start_date?: string | null;
    end_date?: string | null;
    time_zone?: string | null;
    recurrence?: string | null;
    sessions?: Session[];
}

/**
 * A concrete offering of a {@link Course} (schema.org/CourseInstance) —
 * one scheduled run with its own mode, schedule, instructors,
 * capacity, and enrollment counts. A course template has zero or more
 * instances, linked back via `course_id`.
 */
export interface CourseInstance {
    id?: string;
    course_id: string;
    name?: string | null;
    course_mode?: CourseMode | null;
    status?: CourseInstanceStatus;
    schedule?: Schedule | null;
    in_language?: string[];
    location?: string | null;
    location_id?: string | null;
    instructor_ids?: string[];
    instructor_names?: string[];
    maximum_attendee_capacity?: number | null;
    enrolled_count?: number | null;
    enrollment_opens?: string | null;
    enrollment_closes?: string | null;
    created_at?: string;
    updated_at?: string;
}

// ─── Course ──────────────────────────────────────────────────────────

/**
 * The central Course aggregate (schema.org/Course), mirroring the Rust
 * service model field-for-field. A course is a reusable *template*
 * (curriculum, prerequisites, credential awarded); specific scheduled
 * runs live in `instances` as {@link CourseInstance}s. Only `name` is
 * required by the service; everything else is optional. Server-managed
 * fields (`id`, `created_at`, `updated_at`, `deleted_at`) are absent on
 * create and populated on read.
 */
export interface Course {
    id?: string;
    name: string;
    alternate_names?: string[];
    description?: string | null;
    disambiguating_description?: string | null;
    url?: string | null;
    image?: string[];
    same_as?: string[];
    keywords?: string[];
    identifiers?: CourseIdentifier[];
    additional_type?: string | null;
    active?: boolean;
    about?: string[];
    audience?: string | null;
    in_language?: string[];
    license?: string | null;
    typical_age_range?: string | null;
    time_required?: string | null;
    version?: string | null;
    is_accessible_for_free?: boolean | null;
    teaches?: string[];
    assesses?: string[];
    competency_required?: string[];
    educational_level?: EducationalLevel | null;
    educational_use?: string | null;
    learning_resource_type?: LearningResourceType | null;
    interactivity_type?: InteractivityType | null;
    course_code?: string | null;
    number_of_credits?: number | null;
    course_prerequisites?: string[];
    available_language?: string[];
    financial_aid_eligible?: string[];
    educational_credential_awarded?: EducationalCredential | null;
    occupational_credential_awarded?: EducationalCredential | null;
    total_historical_enrollment?: number | null;
    syllabus_sections?: Syllabus[];
    instances?: CourseInstance[];
    status?: CourseStatus;
    links?: CourseLink[];
    provider_id?: string | null;
    deleted_at?: string | null;
    created_at?: string;
    updated_at?: string;
}

// ─── Matching ────────────────────────────────────────────────────────

/** Coarse confidence band the service assigns to a match score. */
export type MatchConfidence = "High" | "Medium" | "Low";

/**
 * Per-component scores (each in [0,1]) that fed the overall match
 * score, so operators can see *why* two courses matched. Any component
 * may be `null` when the field was absent from one side.
 * `deterministic_match` is true when a short-circuit rule (shared
 * deterministic identifier / provider+code / same-as URL) pinned the
 * score to 1.0, in which case the weighted components are moot.
 */
export interface MatchBreakdown {
    name_score?: number | null;
    course_code_score?: number | null;
    provider_score?: number | null;
    educational_level_score?: number | null;
    keywords_score?: number | null;
    teaches_score?: number | null;
    deterministic_match?: boolean;
}

/**
 * Flat match result emitted by `/api/courses/match`,
 * `/api/courses/check-duplicates`, and (via `review_items`) by
 * `/api/courses/deduplicate`. Carries the candidate `course_id`
 * plus a slim in-line summary (`name`, `course_code`) so the UI
 * can render a hit list without a per-row round-trip.
 */
export interface MatchResult {
    course_id: string;
    name: string;
    course_code?: string | null;
    score: number;
    is_match: boolean;
    confidence: MatchConfidence;
    breakdown?: MatchBreakdown;
}

/**
 * Probe body for `POST /api/courses/match`. The service accepts a
 * `Course`-shaped body (every field is optional except `name` —
 * empty / blank `name` → 422). Result thresholds live client-side:
 * the service returns every blocked candidate sorted by descending
 * score, and the UI applies its own threshold as a filter.
 */
export interface MatchRequest {
    name?: string;
    course_code?: string;
    provider_id?: string;
    educational_level?: EducationalLevel;
    keywords?: string[];
    teaches?: string[];
    identifiers?: CourseIdentifier[];
    same_as?: string[];
}

// ─── Merge ───────────────────────────────────────────────────────────

/** Final state of a merge: applied (`Completed`) or undone (`Reversed`). */
export type MergeStatus = "Completed" | "Reversed";

/**
 * Body for `POST /api/courses/merge`. The duplicate is folded into the
 * surviving `main_course_id` and soft-deleted. `merge_reason` and
 * `merged_by` are recorded on the audit trail.
 */
export interface MergeRequest {
    main_course_id: string;
    duplicate_course_id: string;
    merge_reason?: string | null;
    merged_by?: string | null;
}

/**
 * Immutable audit record of one merge, including the `match_score` at
 * merge time and a `transferred_data` snapshot of what moved from the
 * duplicate onto the main course (kept `unknown` — opaque to the UI).
 */
export interface MergeRecord {
    id: string;
    main_course_id: string;
    duplicate_course_id: string;
    status: MergeStatus;
    merged_by?: string | null;
    merge_reason?: string | null;
    match_score?: number | null;
    transferred_data?: unknown;
    merged_at: string;
}

/** Result of a successful merge: the new {@link MergeRecord} plus the surviving course. */
export interface MergeResponse {
    merge_record: MergeRecord;
    main_course: Course;
}

// ─── Batch dedup ─────────────────────────────────────────────────────

/**
 * Tuning knobs for `POST /api/courses/deduplicate`. `threshold` gates
 * what counts as a candidate pair; pairs at or above
 * `auto_merge_threshold` are merged automatically, the rest queued for
 * human review. `max_candidates` caps fan-out per course.
 */
export interface BatchDeduplicationRequest {
    threshold?: number;
    max_candidates?: number;
    auto_merge_threshold?: number;
}

/** Summary counts plus the queued-for-review pairs from a batch dedup scan. */
export interface BatchDeduplicationResponse {
    courses_scanned: number;
    duplicates_found: number;
    auto_merged: number;
    queued_for_review: number;
    review_items: ReviewQueueItem[];
}

/** Disposition of a queued duplicate pair awaiting (or past) human review. */
export type ReviewStatus = "Pending" | "Confirmed" | "Rejected" | "AutoMerged";

/**
 * A candidate duplicate pair (`course_id_a` / `course_id_b`) captured
 * by a batch scan, with its score, quality band, and review status.
 */
export interface ReviewQueueItem {
    id: string;
    course_id_a: string;
    course_id_b: string;
    match_score: number;
    match_quality: string;
    status: ReviewStatus;
    created_at: string;
    reviewed_at?: string | null;
}

// ─── Audit ───────────────────────────────────────────────────────────

/**
 * One row of the HIPAA-style audit trail: who (`user_*`) did what
 * (`action`) to which entity, when (`created_at`), with before/after
 * snapshots (`old_values` / `new_values`, kept `unknown` — rendered
 * generically as JSON by the UI).
 */
export interface AuditEntry {
    id: string;
    entity_type: string;
    entity_id: string;
    action: string;
    user_id?: string | null;
    user_ip_address?: string | null;
    user_agent?: string | null;
    old_values?: unknown;
    new_values?: unknown;
    created_at: string;
}
