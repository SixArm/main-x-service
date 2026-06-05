// TypeScript domain types mirroring the Rust Course Service models
// (schema.org/Course-aligned). Reference:
// course-service-rust-crate/AGENTS/models.md.

// ─── HTTP envelope ───────────────────────────────────────────────────

export interface ApiResponse<T> {
    success: boolean;
    data: T | null;
    error: ApiErrorBody | null;
}

export interface ApiErrorBody {
    code: string;
    message: string;
    details?: unknown;
}

// ─── Identifier types ────────────────────────────────────────────────

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

// Per spec §6 FR-20, these short-circuit matching to 1.0.
export const DETERMINISTIC_TYPES: IdentifierType[] = [
    "Doi", "Wikidata", "Lom", "Oer", "Uri", "Uuid",
];

export const IDENTIFIER_TYPE_OPTIONS: Exclude<IdentifierType, { Custom: string }>[] = [
    "LmsCourseId", "CourseCode", "PlatformSlug", "Oer", "Doi", "Lom",
    "Wikidata", "Isced", "Ror", "Uri", "Uuid",
];

export interface CourseIdentifier {
    property_id: IdentifierType;
    value: string;
    name?: string | null;
    url?: string | null;
}

export function blankCourseIdentifier(): CourseIdentifier {
    return { property_id: "CourseCode", value: "", name: null, url: null };
}

// ─── Educational level / learning resource type ──────────────────────

export type EducationalLevel =
    | "Beginner" | "Intermediate" | "Advanced" | "Expert"
    | "PrimaryEducation" | "SecondaryEducation" | "HigherEducation"
    | "Undergraduate" | "Graduate" | "Postgraduate"
    | "Vocational" | "ProfessionalDevelopment"
    | { Custom: string };

export const EDUCATIONAL_LEVEL_OPTIONS: Exclude<EducationalLevel, { Custom: string }>[] = [
    "Beginner", "Intermediate", "Advanced", "Expert",
    "PrimaryEducation", "SecondaryEducation", "HigherEducation",
    "Undergraduate", "Graduate", "Postgraduate",
    "Vocational", "ProfessionalDevelopment",
];

export type LearningResourceType =
    | "Lecture" | "Tutorial" | "Workshop" | "Assignment" | "Reading"
    | "Video" | "Audio" | "Exam" | "Simulation" | "Project" | "Discussion"
    | { Custom: string };

export const LEARNING_RESOURCE_TYPE_OPTIONS: Exclude<LearningResourceType, { Custom: string }>[] = [
    "Lecture", "Tutorial", "Workshop", "Assignment", "Reading",
    "Video", "Audio", "Exam", "Simulation", "Project", "Discussion",
];

export type InteractivityType = "active" | "expositive" | "mixed";
export const INTERACTIVITY_TYPES: InteractivityType[] = ["active", "expositive", "mixed"];

export type CourseStatus = "draft" | "published" | "archived" | "retired";
export const COURSE_STATUSES: CourseStatus[] = ["draft", "published", "archived", "retired"];

// ─── Educational credential ──────────────────────────────────────────

export type CredentialCategory =
    | "Certificate" | "Diploma" | "Degree" | "Badge"
    | "Microcredential" | "License" | { Custom: string };

export interface EducationalCredential {
    name: string;
    category?: CredentialCategory | null;
    educational_level?: string | null;
    recognized_by?: string | null;
    url?: string | null;
}

// ─── Course-to-course links ──────────────────────────────────────────

export type LinkType = "replaces" | "replaced-by" | "seealso" | "prerequisite" | "successor";

export interface CourseLink {
    other_course_id: string;
    link_type: LinkType;
}

// ─── Syllabus ────────────────────────────────────────────────────────

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

export type CourseMode = "online" | "onsite" | "blended" | "self_paced";
export const COURSE_MODES: CourseMode[] = ["online", "onsite", "blended", "self_paced"];

export type CourseInstanceStatus =
    | "scheduled" | "enrollment_open" | "enrollment_closed"
    | "in_progress" | "completed" | "cancelled";
export const COURSE_INSTANCE_STATUSES: CourseInstanceStatus[] = [
    "scheduled", "enrollment_open", "enrollment_closed",
    "in_progress", "completed", "cancelled",
];

export interface Session {
    start: string;
    end?: string | null;
    label?: string | null;
}

export interface Schedule {
    start_date?: string | null;
    end_date?: string | null;
    time_zone?: string | null;
    recurrence?: string | null;
    sessions?: Session[];
}

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

export type MatchConfidence = "High" | "Medium" | "Low";

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

export interface MatchRequest {
    name?: string;
    course_code?: string;
    provider_id?: string;
    educational_level?: EducationalLevel;
    keywords?: string[];
    teaches?: string[];
    identifiers?: CourseIdentifier[];
    same_as?: string[];
    threshold?: number;
    max_candidates?: number;
}

// ─── Merge ───────────────────────────────────────────────────────────

export type MergeStatus = "Completed" | "Reversed";

export interface MergeRequest {
    main_course_id: string;
    duplicate_course_id: string;
    merge_reason?: string | null;
    merged_by?: string | null;
}

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

export interface MergeResponse {
    merge_record: MergeRecord;
    main_course: Course;
}

// ─── Batch dedup ─────────────────────────────────────────────────────

export interface BatchDeduplicationRequest {
    threshold?: number;
    max_candidates?: number;
    auto_merge_threshold?: number;
}

export interface BatchDeduplicationResponse {
    courses_scanned: number;
    duplicates_found: number;
    auto_merged: number;
    queued_for_review: number;
    review_items: ReviewQueueItem[];
}

export type ReviewStatus = "Pending" | "Confirmed" | "Rejected" | "AutoMerged";

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
