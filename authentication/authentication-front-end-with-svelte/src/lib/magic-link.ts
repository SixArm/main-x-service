// Whether this deployment offers magic-link sign-in and sign-up.
//
// Mirrors the auth service's `AUTH_MAGIC_LINK_ENABLED`: an instance that
// accepts only enterprise SSO (spec/nhs-wales-supplier-assurance NWS-A3)
// sets PUBLIC_MAGIC_LINK_SIGNIN_ENABLED=false, so the email form and the
// sign-up page are not offered — the service would answer 404 to them.
// Unset, empty, or anything but "false" keeps today's behaviour: on.

/** True unless `value` is "false" (any case, surrounding space ignored). */
export function magicLinkEnabled(value: string | undefined): boolean {
  return (value ?? "").trim().toLowerCase() !== "false";
}
