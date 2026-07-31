<!--
  Site settings: the declarations everything else is scoped by.

  Read-only for now, and honest about it: showing an editable form that
  silently fails would be worse than showing the truth and saying which
  surface changes it. Webhook secrets are never listed, because the
  service returns them exactly once at registration.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import * as cms from "$lib/api/cms";
  import SitePicker from "$lib/components/SitePicker.svelte";
  import type { ContentType, Site } from "$lib/api/cms";

  let site = $state<string | null>(null);
  let sites = $state<Site[]>([]);
  let types = $state<ContentType[]>([]);
  let templates = $state<{ key?: string; name?: string }[]>([]);
  let menus = $state<{ key?: string; locale?: string }[]>([]);
  let redirects = $state<{ from_path?: string; to_path?: string; status?: number }[]>([]);
  let webhooks = $state<{ name?: string; url?: string; active?: boolean }[]>([]);
  let note = $state("");
  let failure = $state<string | null>(null);

  const selected = $derived(sites.find((s) => s.pid === site) ?? null);

  $effect(() => {
    const pid = site;
    if (!pid) return;
    Promise.all([
      cms.listContentTypes(pid),
      cms.listTemplates(pid),
      cms.listMenus(pid),
      cms.listRedirects(pid),
      cms.listWebhooks(pid),
    ])
      .then(([declared, tpl, menu, redirect, hooks]) => {
        types = declared;
        templates = tpl as typeof templates;
        menus = menu as typeof menus;
        redirects = redirect as typeof redirects;
        webhooks = hooks.webhooks as typeof webhooks;
        note = hooks.note;
      })
      .catch((e: unknown) => (failure = String(e)));
  });
</script>

<svelte:head><title>{t("nav.settings")}</title></svelte:head>

<h1>{t("nav.settings")}</h1>
<SitePicker bind:site bind:sites />

{#if failure}
  <div class="panel error">{t("common.error")}: {failure}</div>
{:else if selected}
  <section class="panel">
    <h2>{t("entry.locales")}</h2>
    <p>
      {t("translations.source")}: <strong>{selected.default_locale}</strong>
    </p>
    <p>{selected.locales.join(" · ")}</p>
    {#each Object.entries(selected.fallback_chains) as [locale, chain] (locale)}
      <p class="muted">{locale} → {chain.join(" → ")}</p>
    {/each}
    <p class="muted">{t("common.status")}: {selected.visibility}</p>
  </section>

  <section class="panel">
    <h2>{t("settings.contentTypes")}</h2>
    <table>
      <thead>
        <tr><th>{t("entry.key")}</th><th>{t("common.title")}</th><th>v</th><th>{t("common.path")}</th></tr>
      </thead>
      <tbody>
        {#each types as type (type.pid)}
          <tr>
            <td>{type.key}</td>
            <td>{type.name}</td>
            <td>{type.schema_version}</td>
            <td>{type.routable ? "✓" : "—"}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </section>

  <section class="panel">
    <h2>{t("settings.templates")}</h2>
    <p>{templates.map((tpl) => tpl.key ?? "").join(" · ") || t("common.noData")}</p>
  </section>

  <section class="panel">
    <h2>{t("settings.menus")}</h2>
    <p>{menus.map((menu) => `${menu.key} (${menu.locale})`).join(" · ") || t("common.noData")}</p>
  </section>

  <section class="panel">
    <h2>{t("settings.redirects")}</h2>
    {#if redirects.length === 0}
      <p class="no-data">{t("common.noData")}</p>
    {:else}
      <table>
        <thead><tr><th>{t("common.path")}</th><th>→</th><th>{t("common.status")}</th></tr></thead>
        <tbody>
          {#each redirects as redirect, index (index)}
            <tr>
              <td>{redirect.from_path}</td>
              <td>{redirect.to_path ?? "—"}</td>
              <td>{redirect.status}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>

  <section class="panel">
    <h2>{t("settings.webhooks")}</h2>
    {#if webhooks.length === 0}
      <p class="no-data">{t("common.noData")}</p>
    {:else}
      <table>
        <thead><tr><th>{t("common.title")}</th><th>URL</th><th>{t("common.status")}</th></tr></thead>
        <tbody>
          <!-- Keyed by position: two subscriptions may share a URL. -->
          {#each webhooks as hook, index (index)}
            <tr>
              <td>{hook.name}</td>
              <td>{hook.url}</td>
              <td>{hook.active ? "✓" : "—"}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
    {#if note}<p class="muted">{note}</p>{/if}
  </section>
{/if}
