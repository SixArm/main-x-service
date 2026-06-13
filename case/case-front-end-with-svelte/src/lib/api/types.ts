// Types mirroring the Case Service payload, which is the
// `case_matcher::Case` shape itself.
// Source of truth: case-matcher-rust-crate/src/case.rs.

/// Identifier schemes. Rust serializes unit variants as the bare string;
/// `Custom` as `{ "Custom": "label" }`.
export type IdentifierScheme =
  | "Docket"
  | "ExternalCaseId"
  | "Uri"
  | "Uuid"
  | "AgencyCaseNumber"
  | "LocalId"
  | { Custom: string };

export const ALL_SCHEMES: IdentifierScheme[] = [
  "Docket",
  "ExternalCaseId",
  "Uri",
  "Uuid",
  "AgencyCaseNumber",
  "LocalId",
];

/// The type / domain of a case (unit variants only here).
export type CaseType =
  | "Benefit"
  | "Legal"
  | "SocialServices"
  | "Healthcare"
  | "Housing"
  | "Immigration"
  | "Licensing"
  | "Complaint"
  | "Appeal"
  | "Investigation"
  | "Tax"
  | "Employment"
  | { Custom: string };

export const ALL_CASE_TYPES: CaseType[] = [
  "Benefit",
  "Legal",
  "SocialServices",
  "Healthcare",
  "Housing",
  "Immigration",
  "Licensing",
  "Complaint",
  "Appeal",
  "Investigation",
  "Tax",
  "Employment",
];

/// The lifecycle status of a case (unit variants only here).
export type CaseStatus =
  | "Open"
  | "InProgress"
  | "Pending"
  | "OnHold"
  | "Closed"
  | "Resolved"
  | "Rejected"
  | "Withdrawn"
  | { Custom: string };

export const ALL_STATUSES: CaseStatus[] = [
  "Open",
  "InProgress",
  "Pending",
  "OnHold",
  "Closed",
  "Resolved",
  "Rejected",
  "Withdrawn",
];

/// Case priority. Data only — never contributes to the score.
export type Priority = "Low" | "Normal" | "High" | "Urgent";

export const ALL_PRIORITIES: Priority[] = ["Low", "Normal", "High", "Urgent"];

export interface CaseIdentifier {
  scheme: IdentifierScheme;
  value: string;
}

export interface Case {
  title: string;
  alternate_titles?: string[];
  case_number?: string | null;
  agency_id?: string | null;
  agency_name?: string | null;
  case_type?: CaseType | null;
  status?: CaseStatus | null;
  priority?: Priority | null;
  opened_date?: string | null;
  subjects?: string[];
  keywords?: string[];
  identifiers?: CaseIdentifier[];
  same_as?: string[];
  in_language?: string[];
}

/// `{pid, title}` returned by create / list.
export interface CaseRef {
  pid: string;
  title: string;
}

/// A scored duplicate from /check-duplicates.
export interface ScoredRef {
  pid: string;
  title: string;
  score: number;
  confidence: string;
  is_match: boolean;
}
