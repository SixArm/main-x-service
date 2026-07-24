// Minimal typed fetch wrapper over the BFF proxy.

import { API_BASE_URL } from "$lib/config";

/** Error carrying the upstream status + parsed body, for page handling. */
export class ApiError extends Error {
  status: number;
  body: unknown;
  constructor(status: number, body: unknown) {
    const description =
      typeof body === "object" && body !== null && "description" in body
        ? String((body as { description: unknown }).description)
        : `API error ${status}`;
    super(description);
    this.status = status;
    this.body = body;
  }
}

/** A conditional GET's outcome: `body` is `null` on `304 Not Modified`. */
export interface Conditional<T> {
  status: number;
  etag: string | null;
  body: T | null;
}

/**
 * Conditional GET with `If-None-Match` (the board-polling path,
 * PF-T17): `304` yields `body: null` so the caller keeps its current
 * render; anything else parses as usual.
 */
export async function apiConditional<T>(
  path: string,
  etag: string | null,
  init?: { fetch?: typeof fetch },
): Promise<Conditional<T>> {
  const doFetch = init?.fetch ?? fetch;
  const response = await doFetch(`${API_BASE_URL}${path}`, {
    headers: etag ? { "if-none-match": etag } : {},
  });
  const nextEtag = response.headers.get("etag");
  if (response.status === 304) {
    return { status: 304, etag: nextEtag ?? etag, body: null };
  }
  const text = await response.text();
  const parsed: unknown = text ? JSON.parse(text) : null;
  if (!response.ok) throw new ApiError(response.status, parsed);
  return { status: response.status, etag: nextEtag, body: parsed as T };
}

/** Perform one JSON request against the proxy; throws [`ApiError`] on non-2xx. */
export async function api<T>(
  path: string,
  init?: { method?: string; body?: unknown; fetch?: typeof fetch },
): Promise<T> {
  const doFetch = init?.fetch ?? fetch;
  const url = `${API_BASE_URL}${path}`;
  const response = await doFetch(url, {
    method: init?.method ?? "GET",
    headers:
      init?.body !== undefined ? { "content-type": "application/json" } : {},
    body: init?.body !== undefined ? JSON.stringify(init.body) : undefined,
  });
  const text = await response.text();
  const parsed: unknown = text ? JSON.parse(text) : null;
  if (!response.ok) throw new ApiError(response.status, parsed);
  return parsed as T;
}
