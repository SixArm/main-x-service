<script lang="ts">
  import "../app.css";
  import { LOCALE_LABELS, i18n, isRtl, t } from "$lib/i18n.svelte";

  let { children } = $props();

  $effect(() => {
    document.documentElement.lang = i18n.locale;
    document.documentElement.dir = isRtl(i18n.locale) ? "rtl" : "ltr";
  });
</script>

<nav class="top">
  <a class="brand" href="/">{t("brand.name")}</a>
  <a href="/contacts">{t("nav.contacts")}</a>
  <a href="/leads">{t("nav.leads")}</a>
  <a href="/deals">{t("nav.deals")}</a>
  <a href="/campaigns">{t("nav.campaigns")}</a>
  <a href="/tickets">{t("nav.tickets")}</a>
  <a href="/articles">{t("nav.articles")}</a>
  <span class="spacer"></span>
  <label class="locale">
    <span class="visually-hidden">{t("chrome.language")}</span>
    <select
      value={i18n.locale}
      onchange={(event) => i18n.set(event.currentTarget.value)}
    >
      {#each i18n.locales as locale (locale)}
        <option value={locale}>{LOCALE_LABELS[locale]}</option>
      {/each}
    </select>
  </label>
  <a href="/signin">{t("nav.signin")}</a>
</nav>

<main>
  {@render children()}
</main>

<style>
  .spacer {
    flex: 1;
  }
  .locale select {
    font: inherit;
    padding: 0.15rem 0.3rem;
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
