<script lang="ts">
  import "../app.css";
  import { LOCALE_LABELS, i18n, isRtl, t } from "$lib/i18n.svelte";
  import { LocaleSelect } from "lily-design-system-svelte-locale-select";
  import { ThemeSelect } from "lily-design-system-svelte-theme-select";

  let { children } = $props();

  // Lily theme catalogue offered in the theme select (DaisyUI-style
  // slugs plus government/NHS design-system themes). Each slug has a
  // stylesheet at `static/assets/themes/<slug>.css` (a symlink to the
  // shared design-system themes) that ThemeSelect swaps in; labels are
  // title-cased from the slug by the component.
  const THEMES = [
    "abyss", "acid", "adobe-spectrum", "aqua", "autumn", "black",
    "bumblebee", "business", "caramellatte", "cmyk", "coffee",
    "corporate", "cupcake", "cyberpunk", "dark", "dim", "dracula",
    "emerald", "fantasy", "forest", "garden", "halloween", "lemonade",
    "light", "lofi", "luxury", "mozilla-protocol", "night", "nord",
    "pastel", "retro", "silk", "sunset", "synthwave",
    "united-kingdom-government-digital-service",
    "united-kingdom-national-health-service-england-for-patients",
    "united-kingdom-national-health-service-england-for-practitioners",
    "united-kingdom-national-health-service-scotland-for-patients",
    "united-kingdom-national-health-service-scotland-for-practitioners",
    "united-kingdom-national-health-service-wales-for-patients",
    "united-kingdom-national-health-service-wales-for-practitioners",
    "united-states-web-design-system", "valentine", "winter", "wireframe",
  ];


  $effect(() => {
    document.documentElement.lang = i18n.locale;
    document.documentElement.dir = isRtl(i18n.locale) ? "rtl" : "ltr";
  });
</script>

<nav class="top">
  <a class="brand" href="/">{t("brand.name")}</a>
  <a href="/contacts">{t("nav.contacts")}</a>
  <a href="/accounts">{t("nav.accounts")}</a>
  <a href="/leads">{t("nav.leads")}</a>
  <a href="/deals">{t("nav.deals")}</a>
  <a href="/campaigns">{t("nav.campaigns")}</a>
  <a href="/tickets">{t("nav.tickets")}</a>
  <a href="/articles">{t("nav.articles")}</a>
  <a href="/followups">{t("nav.followups")}</a>
  <a href="/executive">{t("nav.executive")}</a>
  <a href="/dpo">{t("nav.dpo")}</a>
  <a href="/engagement">{t("nav.engagement")}</a>
  <a href="/partners">{t("nav.partners")}</a>
  <span class="spacer"></span>
  <label class="locale">
    <span class="visually-hidden">{t("chrome.language")}</span>
    <LocaleSelect
      label={t("chrome.language")}
      locales={[...i18n.locales]}
      localeLabels={LOCALE_LABELS}
      value={i18n.locale}
      applyDir={false}
      onChange={(code) => i18n.set(code)}
    />
  </label>
  <ThemeSelect
    label="Theme"
    themesUrl="/assets/themes/"
    themes={THEMES}
    storageKey="mxi.crm.theme"
  />
  <a href="/signin">{t("nav.signin")}</a>
</nav>

<main>
  {@render children()}
</main>

<style>
  .spacer {
    flex: 1;
  }
  .locale :global(select),
  nav.top :global(select.theme-select) {
    font: inherit;
    padding: 0.15rem 0.3rem;
    max-width: 11rem;
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
