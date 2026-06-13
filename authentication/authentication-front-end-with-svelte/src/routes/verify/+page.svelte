<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import { AuthRepository } from "$lib/api/auth";
    import { session } from "$lib/auth/session.svelte";
    import { t } from "$lib/i18n.svelte";

    const repo = AuthRepository.withFetch();

    let status = $state<"working" | "error">("working");
    let error = $state<string | null>(null);

    onMount(async () => {
        const token = page.url.searchParams.get("token");
        if (!token) {
            status = "error";
            error = t("verify.error.missingToken");
            return;
        }
        try {
            const login = await repo.verify(token);
            session.start(login);
            await goto("/");
        } catch (err) {
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
