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
): Promise<{ contact: Contact; activities: Activity[]; deals: Deal[]; tickets: Ticket[] }> {
  return api(`/contacts/${pid}`, init);
}

/** Record a consent change. */
export function recordConsent(pid: string, action: "granted" | "withdrawn"): Promise<unknown> {
  return api(`/contacts/${pid}/consent`, {
    method: "POST",
    body: { action, source: "operator ui" },
  });
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

/** Pipelines with stages. */
export function listPipelines(
  init?: FetchLike,
): Promise<{ pipeline: { pid: string; name: string }; stages: Stage[] }[]> {
  return api("/pipelines", init);
}

/** Deals (optionally by pipeline). */
export function listDeals(pipeline?: string, init?: FetchLike): Promise<Deal[]> {
  return api(`/deals${pipeline ? `?pipeline=${pipeline}` : ""}`, init);
}

/** Move a deal to a stage. */
export function moveDeal(pid: string, stagePid: string, lostReason?: string): Promise<Deal> {
  return api(`/deals/${pid}/stage`, {
    method: "POST",
    body: { stage_pid: stagePid, ...(lostReason ? { lost_reason: lostReason } : {}) },
  });
}

/** The live forecast. */
export function forecast(
  init?: FetchLike,
): Promise<{ as_of: string; open_deals: number; totals_minor: Record<string, number> }> {
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
): Promise<{ campaign: Campaign; leads: number; won_deals: number; won_revenue_minor: number; roi: Ratio }> {
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
export function salesDashboard(
  init?: FetchLike,
): Promise<{ win_rate: Ratio; open_deals: number; pipeline_by_stage: Record<string, unknown> }> {
  return api("/dashboards/sales", init);
}

/** The SLA dashboard. */
export function slaDashboard(
  init?: FetchLike,
): Promise<{ open_tickets: number; by_priority: { priority: string; open: number; breached: number }[] }> {
  return api("/dashboards/sla", init);
}
