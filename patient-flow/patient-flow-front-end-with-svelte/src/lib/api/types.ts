// Wire types mirroring the patient-flow service's API shapes
// (spec `whiteboard.md`, `capacity.md`, `domain-model.md`). Kept
// per-project by design — front-end drift is accepted family-wide.

/** Bed states (spec `bed-management.md`). */
export type BedState =
  | "available"
  | "reserved"
  | "occupied"
  | "awaiting_clean"
  | "cleaning"
  | "closed";

export type RedGreen = "red" | "green";

export interface InfectionChip {
  precaution: "contact" | "droplet" | "airborne" | "protective";
  organism: string | null;
  status: "suspected" | "confirmed" | "cleared";
}

/** One bed card on the ward whiteboard. */
export interface BedCard {
  bed_pid: string;
  bay_name: string;
  number: string;
  state: BedState;
  state_since: string;
  closure_reason: string | null;
  deep_clean_required: boolean;
  side_room: boolean;
  stay_pid: string | null;
  display_name: string | null;
  named_nurse_ref: string | null;
  consultant_ref: string | null;
  edd: string | null;
  edd_missing: boolean;
  edd_overdue: boolean;
  ccd_met: boolean;
  discharge_pathway: string | null;
  discharge_ready: boolean;
  dtoc: boolean;
  senior_review_today: boolean;
  red_green_today: RedGreen | null;
  infection: InfectionChip[];
  alerts: string[];
}

export interface Whiteboard {
  ward_pid: string;
  ward_name: string;
  ward_code: string;
  kind: "inpatient" | "assessment" | "virtual";
  closed_to_admissions: boolean;
  escalation: boolean;
  as_of: string;
  masked: boolean;
  cards: BedCard[];
}

export interface Ward {
  pid: string;
  site_pid: string;
  name: string;
  code: string;
  kind: string;
  specialty: string | null;
  open: boolean;
  escalation: boolean;
  closed_to_admissions: boolean;
}

export interface WardGlance {
  ward_pid: string;
  site_pid: string;
  name: string;
  code: string;
  kind: string;
  escalation: boolean;
  closed_to_admissions: boolean;
  beds_total: number;
  occupied: number;
  available: number;
  reserved: number;
  awaiting_clean: number;
  cleaning: number;
  closed: number;
  closed_for_infection: number;
  occupancy_pct: number;
  expected_discharges_today: number;
  edd_overdue: number;
  discharge_ready: number;
  dtoc: number;
  open_requests_targeting: number;
  long_stay_7: number;
  long_stay_21: number;
}

export interface SiteTiles {
  available_now: number;
  predicted_available_by_midnight: number;
  open_requests: { emergency: number; urgent: number; routine: number };
  dtoc: number;
  virtual_ward_census: number;
  escalation_beds_open: number;
}

export interface AtAGlance {
  as_of: string;
  wards: WardGlance[];
  site_tiles: SiteTiles;
}

export interface Stay {
  pid: string;
  person_ref: string;
  display_name: string;
  status: "admitted" | "discharge_ready" | "discharged";
  admitted_at: string;
  source: string;
  ward_pid: string | null;
  bed_pid: string | null;
  home_location_note: string | null;
  named_nurse_ref: string | null;
  consultant_ref: string | null;
  senior_review_at: string | null;
  edd: string | null;
  ccd: string | null;
  ccd_met: boolean;
  discharge_pathway: string | null;
  discharge_ready_at: string | null;
  discharged_at: string | null;
  discharge_destination: string | null;
  alerts: string[] | unknown;
}

export interface Transfer {
  pid: string;
  stay_pid: string;
  from_bed_pid: string | null;
  to_bed_pid: string | null;
  reason: string;
  moved_at: string;
}

export interface RedGreenDay {
  stay_pid: string;
  day: string;
  classification: RedGreen;
  delay_reasons: string[] | unknown;
  note: string | null;
}

export interface InfectionFlag extends InfectionChip {
  pid: string;
  stay_pid: string;
  requires_side_room: boolean;
  flagged_at: string;
  cleared_at: string | null;
}

export interface StayDetail {
  stay: Stay;
  transfers: Transfer[];
  red_green: RedGreenDay[];
  infection_flags: InfectionFlag[];
  length_of_stay_days: number;
  dtoc: boolean;
}

export interface BedRequest {
  pid: string;
  person_ref: string;
  origin: string;
  target_ward_pid: string | null;
  specialty: string | null;
  priority: "emergency" | "urgent" | "routine";
  status: "open" | "allocated" | "fulfilled" | "cancelled";
  allocated_bed_pid: string | null;
  requested_at: string;
  resolved_at: string | null;
  eligible_beds?: number;
}

export interface EligibleBed {
  bed_pid: string;
  number: string;
  ward_pid: string;
  ward_code: string;
  bay_name: string;
  side_room: boolean;
  right_ward: boolean;
}

export interface Locate {
  person_ref: string;
  display_name: string;
  status: string;
  stay_pid: string;
  site: string | null;
  ward: { pid: string; name: string; code: string; kind: string } | null;
  bay: string | null;
  bed: string | null;
  home_location_note: string | null;
  discharged_at: string | null;
}

export interface AuditEntry {
  id: number;
  created_at: string;
  entity: string;
  entity_pid: string;
  action: string;
  actor: string | null;
  snapshot: unknown;
}
