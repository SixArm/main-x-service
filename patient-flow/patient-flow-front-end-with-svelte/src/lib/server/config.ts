// Server-side (BFF) configuration. Never imported by browser code.

import { env } from "$env/dynamic/private";

/** Base URL of the patient-flow service the BFF proxies to. */
export const PATIENT_FLOW_API_URL: string =
  env.PATIENT_FLOW_API_URL ?? "http://localhost:5150";
