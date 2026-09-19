// Pure helpers for T-28f (repo `tasks.md` EV-1): role-tailored navigation
// and landing view, driven by a deployment-declared `view` ABAC attribute
// (e.g. `view=executive`) read from `GET /api/auth/me`'s `attrs`
// (`$lib/server/auth.ts`'s `currentUser`). Extracted from `+layout.svelte`
// / the root `+page.svelte` so both are unit-testable without mounting a
// component — mirroring `merge-validation.ts`'s pattern.
//
// `view` is a vocabulary the DEPLOYMENT declares, not an enum this code
// owns (T-28f's own wording): there is no fixed list of valid values here.
// Instead, a `view` matches when `/${view}` is already one of the app's
// own nav hrefs (e.g. `/executive`, `/financials`, `/risk`, …) — an
// unrecognised value is simply ignored, never an error. Absent/unmatched
// `view` ⇒ today's behaviour, byte-for-byte: this is the "default is
// today's full nav" contract T-28f states explicitly.

export interface NavItem {
  href: string;
  label: string;
}

/**
 * The static set of nav destination hrefs (no labels — those are
 * i18n-dependent and live only in `+layout.svelte`'s `navItems`).
 * `landingRouteForView` needs this list **server-side**
 * (`+layout.server.ts`, to decide `/`'s redirect target before any
 * client-side i18n exists), where `navItems` itself is unavailable — so
 * this is a second, intentionally minimal copy of `navItems`'s hrefs,
 * not derived from it. Keep the two in sync when adding a nav route;
 * drift here only ever narrows what `view` can land on (a new route
 * missing here just can't be a landing view yet), never breaks nav
 * ordering, which checks `navItems` directly.
 */
export const KNOWN_NAV_HREFS = [
  "/plans",
  "/plans/merge",
  "/dashboard",
  "/prioritisation",
  "/lifecycle",
  "/reviews",
  "/automations",
  "/proposals",
  "/ideas",
  "/scenarios",
  "/objectives",
  "/gantt",
  "/engineering",
  "/calendar",
  "/executive",
  "/financials",
  "/technology",
  "/board",
  "/auditor",
  "/compliance",
  "/risk",
  "/security",
  "/regulator",
  "/capacity",
  "/reports",
  "/onboarding",
] as const;

/**
 * Reorder `items`, moving the item whose `href` is `/${view}` to
 * immediately after the first item (the brand/home link) — everything
 * else keeps its original relative order. `view` absent, empty, or not
 * matching any item's `href` (including the brand link itself, at index
 * 0) returns `items` unchanged.
 */
export function orderNavForView<T extends NavItem>(
  items: readonly T[],
  view: string | null | undefined,
): T[] {
  if (!view) return [...items];
  const target = `/${view}`;
  const index = items.findIndex((item) => item.href === target);
  if (index <= 0) return [...items];
  const matched = items[index] as T;
  return [
    items[0] as T,
    matched,
    ...items.slice(1, index),
    ...items.slice(index + 1),
  ];
}

/**
 * The route a signed-in visitor lands on when they open `/`: `/${view}`
 * when `view` is set and matches one of `knownHrefs`; `defaultRoute`
 * (today's `/plans`) otherwise.
 */
export function landingRouteForView(
  knownHrefs: readonly string[],
  view: string | null | undefined,
  defaultRoute: string,
): string {
  if (!view) return defaultRoute;
  const target = `/${view}`;
  return knownHrefs.includes(target) ? target : defaultRoute;
}

/**
 * The deployment-declared `view` attribute's first value, from
 * `CurrentUser.attrs` (`$lib/server/auth.ts`). `attrs` absent/empty, or
 * the `view` key absent/empty, both mean "no preference" (`null`).
 */
export function viewAttr(
  attrs: Record<string, string[]> | null | undefined,
): string | null {
  const values = attrs?.["view"];
  return values && values.length > 0 ? (values[0] ?? null) : null;
}
