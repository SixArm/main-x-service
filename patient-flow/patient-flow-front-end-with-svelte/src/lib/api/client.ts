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
