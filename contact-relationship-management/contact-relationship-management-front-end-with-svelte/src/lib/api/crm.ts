// Typed CRM API surface over the BFF proxy, plus the family `money()`
// formatter. Paths mirror the service routes one-to-one — the
// Playwright suite stubs these exact paths, so drift fails loudly.

import { api } from "$lib/api/client";

type FetchLike = { fetch?: typeof fetch };

/** One contact wrapper. */
export interface Contact {
  pid: string;
  person_ref: string;
  account_pid: string | null;
  owner_ref: string | null;
  display_name: string;
  status: string;
  job_title: string | null;
  preferred_channel: string;
  marketing_consent: string;
}

/** One account wrapper. */
export interface Account {
  pid: string;
  organization_ref: string;
  display_name: string;
  tier: string;
  industry: string | null;
}

/** One activity row. */
export interface Activity {
  pid: string;
  subject_kind: string;
  subject_pid: string;
  kind: string;
  occurred_at: string;
  summary: string;
}

/** One lead (score is derived). */
export interface Lead {
  pid: string;
  source: string;
  display_name: string;
  email: string | null;
  score: number;
  status: string;
}

/** One score-breakdown rule. */
export interface RuleScore {
  rule: string;
  points: number;
}

/** The score + explanation. */
export interface ScoreBreakdown {
  score: number;
  label: string;
  rules: RuleScore[];
}

/** One pipeline stage. */
export interface Stage {
  pid: string;
  name: string;
  position: number;
  probability_percent: number;
  is_won: boolean;
  is_lost: boolean;
}

/** One deal (amount may arrive null when masked). */
export interface Deal {
  pid: string;
  name: string;
  pipeline_pid: string;
  stage_pid: string;
  amount_minor: number | null;
  currency: string;
  won: boolean;
  closed_at: string | null;
}

/** One campaign with counters. */
export interface Campaign {
  pid: string;
  name: string;
  status: string;
  cost_minor: number;
  currency: string;
  recipients: number;
  delivered: number;
  opened: number;
  clicked: number;
}

/** One ticket with live breach flags. */
export interface Ticket {
  pid: string;
  title: string;
  priority: string;
  status: string;
  first_response_due_at: string | null;
  live_first_response_breached?: boolean;
  live_resolution_breached?: boolean;
}

/** One KB article. */
export interface Article {
  pid: string;
  title: string;
  body: string;
  status: string;
  version: number;
}

/** An honest ratio (null value on a zero denominator). */
export interface Ratio {
  numerator: number;
  denominator: number;
  value: number | null;
}

/**
 * Format minor units + ISO-4217 as a locale-aware money string.
 * `null` renders as an em dash (the masked/absent state) — never 0.
 */
export function money(
  minor: number | null | undefined,
  currency: string | null | undefined,
  locale?: string,
): string {
  if (minor === null || minor === undefined || !currency) return "—";
  return new Intl.NumberFormat(locale, {
    style: "currency",
    currency,
  }).format(minor / 100);
}

/** Contacts. */
export function listContacts(init?: FetchLike): Promise<Contact[]> {
  return api("/contacts", init);
}

/** One contact + timeline. */
export function getContact(
  pid: string,
  init?: FetchLike,
): Promise<{
  contact: Contact;
  activities: Activity[];
  deals: Deal[];
  tickets: Ticket[];
}> {
  return api(`/contacts/${pid}`, init);
}

/** Record a consent change. */
export function recordConsent(
  pid: string,
  action: "granted" | "withdrawn",
): Promise<unknown> {
  return api(`/contacts/${pid}/consent`, {
    method: "POST",
    body: { action, source: "operator ui" },
  });
}

/**
 * Erase (anonymise) a contact (CRM-R20). Destructive; the service
 * refuses `422` while an open deal, an open ticket, or an active
 * nurture enrolment exists.
 */
export function eraseContact(
  pid: string,
): Promise<{ erased: string; note: string }> {
  return api(`/contacts/${pid}/erase`, { method: "POST" });
}

/** Accounts. */
export function listAccounts(init?: FetchLike): Promise<Account[]> {
  return api("/accounts", init);
}

/** The score-sorted lead queue. */
export function listLeads(init?: FetchLike): Promise<Lead[]> {
  return api("/leads", init);
}

/** One lead + live breakdown. */
export function getLead(
  pid: string,
  init?: FetchLike,
): Promise<{ lead: Lead; score: ScoreBreakdown }> {
  return api(`/leads/${pid}`, init);
}

/** Lead status transition. */
export function leadStatus(pid: string, to: string): Promise<Lead> {
  return api(`/leads/${pid}/status`, { method: "POST", body: { to } });
}

/** Pipelines with stages. */
export function listPipelines(
  init?: FetchLike,
): Promise<{ pipeline: { pid: string; name: string }; stages: Stage[] }[]> {
  return api("/pipelines", init);
}

/** Deals (optionally by pipeline). */
export function listDeals(
  pipeline?: string,
  init?: FetchLike,
): Promise<Deal[]> {
  return api(`/deals${pipeline ? `?pipeline=${pipeline}` : ""}`, init);
}

/** Move a deal to a stage. */
export function moveDeal(
  pid: string,
  stagePid: string,
  lostReason?: string,
): Promise<Deal> {
  return api(`/deals/${pid}/stage`, {
    method: "POST",
    body: {
      stage_pid: stagePid,
      ...(lostReason ? { lost_reason: lostReason } : {}),
    },
  });
}

/** The live forecast. */
export function forecast(init?: FetchLike): Promise<{
  as_of: string;
  open_deals: number;
  totals_minor: Record<string, number>;
}> {
  return api("/forecast", init);
}

/** Campaigns. */
export function listCampaigns(init?: FetchLike): Promise<Campaign[]> {
  return api("/campaigns", init);
}

/** One campaign's funnel + ROI. */
export function campaignFunnel(
  pid: string,
  init?: FetchLike,
): Promise<{
  campaign: Campaign;
  leads: number;
  won_deals: number;
  won_revenue_minor: number;
  roi: Ratio;
}> {
  return api(`/campaigns/${pid}/funnel`, init);
}

/** Run (simulated send). */
export function runCampaign(pid: string): Promise<Campaign> {
  return api(`/campaigns/${pid}/run`, { method: "POST" });
}

/** Campaign status transition. */
export function campaignStatus(pid: string, to: string): Promise<Campaign> {
  return api(`/campaigns/${pid}/status`, { method: "POST", body: { to } });
}

/** Tickets with live breach flags. */
export function listTickets(init?: FetchLike): Promise<Ticket[]> {
  return api("/tickets", init);
}

/** Ticket status transition. */
export function ticketStatus(pid: string, to: string): Promise<Ticket> {
  return api(`/tickets/${pid}/status`, { method: "POST", body: { to } });
}

/** Articles (searchable). */
export function listArticles(q?: string, init?: FetchLike): Promise<Article[]> {
  return api(`/articles${q ? `?q=${encodeURIComponent(q)}` : ""}`, init);
}

/** Publish / archive an article. */
export function articleStatus(pid: string, to: string): Promise<Article> {
  return api(`/articles/${pid}/status`, { method: "POST", body: { to } });
}

/** The sales dashboard. */
export function salesDashboard(init?: FetchLike): Promise<{
  win_rate: Ratio;
  open_deals: number;
  pipeline_by_stage: Record<string, unknown>;
}> {
  return api("/dashboards/sales", init);
}

/** The SLA dashboard. */
export function slaDashboard(init?: FetchLike): Promise<{
  open_tickets: number;
  by_priority: { priority: string; open: number; breached: number }[];
}> {
  return api("/dashboards/sla", init);
}

// ─── Insight views (read-only derivations; as_of + ETag) ────────────

/** `GET /insights/stale-deals` row. */
export interface StaleDeal {
  pid: string;
  name: string;
  stage: string | null;
  owner_ref: string | null;
  amount_minor: number;
  currency: string;
  days_in_stage: number;
  stale: boolean;
}

/** One open follow-up (activity with a due date, not done). */
export interface Followup {
  pid: string;
  kind: string;
  summary: string;
  subject_kind: string;
  subject_pid: string;
  actor_ref: string | null;
  due_on: string;
  overdue_days: number | null;
}

/** One rule-disclosed finding. */
export interface Finding {
  rule: string;
  detail: string;
  [key: string]: unknown;
}

/** Stale-deal aging (server derivation disclosed). */
export function staleDeals(
  days?: number,
  init?: FetchLike,
): Promise<{
  as_of: string;
  derivation: string;
  threshold_days: number;
  open_deals: number;
  stale_deals: number;
  deals: StaleDeal[];
}> {
  return api(`/insights/stale-deals${days ? `?days=${days}` : ""}`, init);
}

/** Open follow-ups: overdue + next 30 days (optional kind filter —
 * the renewals convention: due-dated `task` activities). */
export function followups(
  kind?: string,
  init?: FetchLike,
): Promise<{
  as_of: string;
  note: string;
  overdue: Followup[];
  upcoming_30d: Followup[];
  open_by_recorder: Record<string, number>;
}> {
  return api(
    `/insights/followups${kind ? `?kind=${encodeURIComponent(kind)}` : ""}`,
    init,
  );
}

/** Pipeline-hygiene findings. */
export function pipelineHygiene(
  days?: number,
  init?: FetchLike,
): Promise<{ as_of: string; threshold_days: number; findings: Finding[] }> {
  return api(`/insights/pipeline-hygiene${days ? `?days=${days}` : ""}`, init);
}

/** The period sales executive pack. */
export function executivePack(init?: FetchLike): Promise<{
  as_of: string;
  window: { from: string; to: string };
  deals_won: number;
  deals_lost: number;
  won_value_by_currency_minor: Record<string, number>;
  lost_reasons: Record<string, number>;
  new_leads: number;
  tickets_opened: number;
  tickets_resolved: number;
  campaigns_started: number;
  activities_logged: number;
  consent_withdrawals: number;
  note: string;
}> {
  return api("/insights/executive", init);
}

/** The stored forecast-snapshot series (no interpolation). */
export function forecastTrends(init?: FetchLike): Promise<{
  as_of: string;
  note: string;
  series: Array<{ taken_on: string; totals: Record<string, number> }>;
}> {
  return api("/insights/forecast-trends", init);
}

/** The SLA breach register + per-assignee workload. */
export function slaRegister(init?: FetchLike): Promise<{
  as_of: string;
  derivation: string;
  breaches: Array<{
    pid: string;
    title: string;
    priority: string;
    status: string;
    assignee_ref: string | null;
    breached: string;
    overdue_hours: number;
  }>;
  workload: Array<{
    assignee_ref: string;
    open: number;
    breached: number;
    at_risk_4h: number;
  }>;
}> {
  return api("/insights/sla", init);
}

/** The DPO view: consent coverage + duplicates. */
export function dpo(init?: FetchLike): Promise<{
  as_of: string;
  note: string;
  contacts: number;
  consent_coverage: Record<string, number>;
  window_days: number;
  withdrawals_in_window: number;
  consent_events_by_source: Record<string, number>;
  duplicate_person_refs: Array<{
    person_ref: string;
    contacts: Array<{ pid: string; display_name: string }>;
  }>;
}> {
  return api("/insights/dpo", init);
}

/** Relationship-cadence aging (untouched contacts/accounts). */
export function cadence(
  days?: number,
  init?: FetchLike,
): Promise<{
  as_of: string;
  derivation: string;
  threshold_days: number;
  untouched_contacts: Array<{
    pid: string;
    display_name: string;
    stakeholder_role: string | null;
    days_since_touch: number;
    has_next_touch: boolean;
  }>;
  untouched_accounts: Array<{
    pid: string;
    display_name: string;
    stakeholder_role: string | null;
    days_since_touch: number;
  }>;
  contacts_without_next_touch: number;
}> {
  return api(`/insights/cadence${days ? `?days=${days}` : ""}`, init);
}

/** Engagement workload (touches, kinds, recorded sentiment). */
export function engagementWorkload(
  days?: number,
  init?: FetchLike,
): Promise<{
  as_of: string;
  window_days: number;
  touches: number;
  per_recorder_month: Record<string, number>;
  per_kind: Record<string, number>;
  sentiment: Record<string, number>;
  note: string;
}> {
  return api(`/insights/engagement${days ? `?days=${days}` : ""}`, init);
}

/** Stage funnel for one pipeline (audit-derived, honest ratios). */
export function funnel(
  pipelinePid: string,
  init?: FetchLike,
): Promise<{
  as_of: string;
  pipeline: { pid: string; name: string };
  derivation: string;
  stages: Array<{
    stage: string;
    position: number;
    is_won: boolean;
    is_lost: boolean;
    entered: number;
    conversion_from_previous: {
      numerator: number;
      denominator: number;
      value: number | null;
    } | null;
  }>;
}> {
  return api(
    `/insights/funnel?pipeline=${encodeURIComponent(pipelinePid)}`,
    init,
  );
}

/** Member-account health (+ silent list). */
export function membersHealth(
  days?: number,
  init?: FetchLike,
): Promise<{
  as_of: string;
  derivation: string;
  threshold_days: number;
  silent_accounts: number;
  accounts: Array<{
    pid: string;
    display_name: string;
    tier: string;
    stakeholder_role: string | null;
    membership: {
      status: string;
      joined_on: string;
      renewal_on: string | null;
    } | null;
    contacts: number;
    days_since_touch: number;
    silent: boolean;
    open_followups: number;
    open_tickets: number;
  }>;
}> {
  return api(`/insights/members${days ? `?days=${days}` : ""}`, init);
}

/** Per-account consent rollup (DPO). */
export function consentByAccount(
  days?: number,
  init?: FetchLike,
): Promise<{
  as_of: string;
  window_days: number;
  note: string;
  accounts: Array<{
    pid: string;
    display_name: string;
    consent_coverage: Record<string, number>;
    withdrawals_in_window: number;
  }>;
}> {
  return api(
    `/insights/consent-by-account${days ? `?days=${days}` : ""}`,
    init,
  );
}

/** The declared-stakeholder register + power–interest grid. */
export function stakeholdersView(init?: FetchLike): Promise<{
  as_of: string;
  note: string;
  by_role: Record<
    string,
    Array<{
      pid: string;
      display_name: string;
      marketing_consent: string;
      influence: number | null;
      interest: number | null;
      days_since_touch: number;
    }>
  >;
  grid: Record<string, number>;
  stakeholders_without_grid_scores: number;
  undeclared_contacts: number;
  account_roles: Array<{ pid: string; display_name: string; role: string }>;
}> {
  return api("/insights/stakeholders", init);
}

/** The innovation-partnership register. */
export function partnershipsRegister(init?: FetchLike): Promise<{
  as_of: string;
  by_kind: Record<string, number>;
  by_stage: Record<string, number>;
  register: Array<{
    pid: string;
    account_pid: string;
    account: string | null;
    kind: string;
    stage: string;
    summary: string;
    started_on: string | null;
  }>;
}> {
  return api("/insights/partnerships", init);
}

/** Membership renewals due + the lapsed list. */
export function membershipsView(
  days?: number,
  init?: FetchLike,
): Promise<{
  as_of: string;
  window_days: number;
  memberships: number;
  renewals_due: Array<{
    pid: string;
    account: string | null;
    status: string;
    joined_on: string;
    renewal_on: string | null;
  }>;
  lapsed: Array<{ pid: string; account: string | null; status: string }>;
}> {
  return api(`/insights/memberships${days ? `?days=${days}` : ""}`, init);
}
