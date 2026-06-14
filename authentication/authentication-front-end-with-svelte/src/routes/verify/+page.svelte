<!--
  Verify page: the landing target of the emailed magic link
  (`/verify?token=…`). This is where the access token is captured and the
  session begins.

  On mount:
  1. Read the single-use `token` from the query string (missing ⇒ error view).
  2. Exchange it via GET /api/auth/magic-link/{token} for the access token
     + profile, and start the session (persisting the federation token).
  3. Decide the next hop with the pure `nextDestination`: an allowlisted
     parked `return_to` ⇒ a CROSS-ORIGIN handoff carrying the token in the
     URL fragment (`window.location.assign`, a full navigation — SvelteKit
     `goto` is same-origin only); otherwise stay on this app via `goto("/")`.
     The parked value is cleared either way so it cannot leak into a later
     sign-in.

  State:
  - `status` ($state) — "working" while verifying, "error" on failure;
  - `error` ($state) — the error message shown in the error view.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import { AuthRepository } from "$lib/api/auth";
    import { RETURN_TO_ALLOWLIST } from "$lib/config";
    import {
        clearReturnTo,
        nextDestination,
        parseAllowlist,
        readReturnTo,
    } from "$lib/auth/return-to";
    import { session } from "$lib/auth/session.svelte";
    import { t } from "$lib/i18n.svelte";

    const repo = AuthRepository.withFetch();

    let status = $state<"working" | "error">("working");
    let error = $state<string | null>(null);

    onMount(async () => {
        // Capture the single-use magic-link token from the URL query.
        const token = page.url.searchParams.get("token");
        if (!token) {
            // Link arrived without its token: nothing to verify.
            status = "error";
            error = t("verify.error.missingToken");
            return;
        }
        try {
            // Exchange the magic-link token for an access token + profile.
            const login = await repo.verify(token);
            // Persist the session (and the shared federation token key).
            session.start(login);
            // Cross-origin SSO handoff: if an allowlisted `return_to` was
            // parked on /signin or /signup, redirect there with the token
            // in the URL fragment; else go home. `nextDestination` is the
            // pure decision (unit-tested); we only navigate here.
            const dest = nextDestination(
                readReturnTo(),
                login.token,
                parseAllowlist(RETURN_TO_ALLOWLIST),
                window.location.origin,
            );
            // Drop the parked value regardless of the decision taken.
            clearReturnTo();
            if (dest.kind === "external") {
                // Cross-origin: must be a full navigation, not SvelteKit
                // `goto` (which is same-origin client-side routing).
                window.location.assign(dest.url);
            } else {
                // No handoff: stay on this app at the account dashboard.
                await goto("/");
            }
        } catch (err) {
            // Invalid/expired token (or network failure): show the error view.
            status = "error";
            error = err instanceof Error ? err.message : t("verify.error.invalid");
        }
    });
</script>

<svelte:head><title>{t("verify.working.title")} — {t("brand")}</title></svelte:head>

{#if status === "working"}
    <h1>{t("verify.working.title")}</h1>
    <p>{t("verify.working.body")}</p>
{:else}
    <h1>{t("verify.error.title")}</h1>
    <p class="banner" role="alert">{error}</p>
    <p><a class="button" href="/signin">{t("verify.error.requestNew")}</a></p>
{/if}
