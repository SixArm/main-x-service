// Typed patient-flow API calls (all via the BFF proxy).

import { api } from "./client";
import type {
  AtAGlance,
  AuditEntry,
  BedRequest,
  EligibleBed,
  Locate,
  StayDetail,
  Ward,
  Whiteboard,
} from "./types";

export const getWards = (f?: typeof fetch) =>
  api<Ward[]>("/api/wards", { fetch: f });

export const getWhiteboard = (wardPid: string, f?: typeof fetch) =>
  api<Whiteboard>(`/api/whiteboard/${wardPid}`, { fetch: f });

export const getAtAGlance = (f?: typeof fetch) =>
  api<AtAGlance>("/api/at-a-glance", { fetch: f });

export const getStay = (pid: string, f?: typeof fetch) =>
  api<StayDetail>(`/api/stays/${pid}`, { fetch: f });

export const updateStay = (pid: string, body: unknown) =>
  api(`/api/stays/${pid}`, { method: "PUT", body });

export const recordRedGreen = (
  pid: string,
  classification: "red" | "green",
  delay_reasons: string[],
  note?: string,
) =>
  api(`/api/stays/${pid}/red-green`, {
    method: "POST",
    body: { classification, delay_reasons, note },
  });

export const addInfectionFlag = (pid: string, body: unknown) =>
  api(`/api/stays/${pid}/infection-flags`, { method: "POST", body });

export const clearInfectionFlag = (pid: string, flagPid: string) =>
  api(`/api/stays/${pid}/infection-flags/${flagPid}/clear`, {
    method: "POST",
    body: {},
  });

export const transferStay = (pid: string, body: unknown) =>
  api(`/api/stays/${pid}/transfer`, { method: "POST", body });

export const dischargeReady = (pid: string, pathway: string) =>
  api(`/api/stays/${pid}/discharge-ready`, {
    method: "POST",
    body: { pathway },
  });

export const discharge = (pid: string, destination: string) =>
  api(`/api/stays/${pid}/discharge`, {
    method: "POST",
    body: { destination },
  });

export const bedTransition = (
  bedPid: string,
  transition: string,
  extra?: { reason?: string; deep_clean_done?: boolean },
) =>
  api(`/api/beds/${bedPid}/state`, {
    method: "POST",
    body: { transition, ...extra },
  });

export const admitStay = (body: unknown) =>
  api<{ pid: string; ward_pid: string; edd_missing: boolean }>("/api/stays", {
    method: "POST",
    body,
  });

export const getBedRequests = (status: string, f?: typeof fetch) =>
  api<BedRequest[]>(`/api/bed-requests?status=${status}`, { fetch: f });

export const createBedRequest = (body: unknown) =>
  api<{ pid: string }>("/api/bed-requests", { method: "POST", body });

export const getEligibleBeds = (pid: string, f?: typeof fetch) =>
  api<EligibleBed[]>(`/api/bed-requests/${pid}/eligible`, { fetch: f });

export const allocateBed = (pid: string, bedPid: string) =>
  api(`/api/bed-requests/${pid}/allocate`, {
    method: "POST",
    body: { bed_pid: bedPid },
  });

export const cancelBedRequest = (pid: string) =>
  api(`/api/bed-requests/${pid}/cancel`, { method: "POST", body: {} });

export const locatePerson = (personRef: string, f?: typeof fetch) =>
  api<Locate>(`/api/locate/${encodeURIComponent(personRef)}`, { fetch: f });

export const getRecentAudits = (f?: typeof fetch) =>
  api<AuditEntry[]>("/api/audits/recent", { fetch: f });

export const getHandover = (wardPid: string, since: string, f?: typeof fetch) =>
  api<AuditEntry[]>(
    `/api/audits?ward=${wardPid}&since=${encodeURIComponent(since)}`,
    { fetch: f },
  );
