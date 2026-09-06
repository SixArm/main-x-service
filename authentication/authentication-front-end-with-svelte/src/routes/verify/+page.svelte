<!--
  Verify page (BFF). The magic-link exchange happens entirely in the
  server `load` (`+page.server.ts`): it sets the httpOnly session cookie
  and redirects home on success, so this component renders ONLY when the
  link was missing/invalid. The browser never handles an access token.
-->
<script lang="ts">
    import type { PageData } from "./$types";
    import { t } from "$lib/i18n.svelte";

    let { data }: { data: PageData } = $props();

    // `load` redirects on success, so `data.error` is always set here.
    const message = $derived(
        data.error === "missingToken"
            ? t("verify.error.missingToken")
            : data.error === "serviceUnavailable"
              ? t("verify.error.serviceUnavailable")
              : t("verify.error.invalid"),
    );
</script>

<svelte:head><title>{t("verify.error.title")} — {t("brand")}</title></svelte:head>

<h1>{t("verify.error.title")}</h1>
<p class="banner" role="alert">{message}</p>
<p><a class="button" href="/signin">{t("verify.error.requestNew")}</a></p>
