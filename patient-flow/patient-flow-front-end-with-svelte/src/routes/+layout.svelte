<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import { LocaleSelect } from "lily-design-system-svelte-locale-select";
  import { ThemeSelect } from "lily-design-system-svelte-theme-select";

  let { children } = $props();

  // Kiosk routes are chrome-less (wall touchscreens).
  let kiosk = $derived(page.url.pathname.endsWith("/kiosk"));

  $effect(() => {
    document.body.classList.toggle("kiosk", kiosk);
  });

  // Lily theme catalogue offered in the theme select (DaisyUI-style
  // slugs plus government/NHS design-system themes — the NHS ones are
  // the natural fit here). Each slug has a stylesheet at
  // `static/assets/themes/<slug>.css` (a symlink to the shared
  // design-system themes) that ThemeSelect swaps in; labels are
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

  // The family's supported locales. Patient Flow has no translation
  // catalogue (yet) — the Lily LocaleSelect still owns the document
  // language + writing direction (`lang`/`dir`, RTL for ar/ur) and
  // persists the choice, so the chrome is ready for a catalogue.
  const LOCALES = [
    "en", "cy", "es", "fr", "de", "ar", "ru", "hi", "zh", "bn", "pt",
    "id", "ur",
  ];
</script>

{#if !kiosk}
  <nav class="top">
    <a class="brand" href="/">Patient Flow</a>
    <a href="/wards">Wards</a>
    <a href="/at-a-glance">At a glance</a>
    <a href="/bed-requests">Bed requests</a>
    <a href="/edd">EDD</a>
    <a href="/locate">Locate</a>
    <a href="/audits">Audits</a>
    <span class="spacer"></span>
    <ThemeSelect
      label="Theme"
      themesUrl="/assets/themes/"
      themes={THEMES}
      storageKey="mxi.patient-flow.theme"
    />
    <LocaleSelect
      label="Language"
      locales={LOCALES}
      storageKey="mxi.patient-flow.locale"
    />
    <a href="/signin">Sign in</a>
  </nav>
{/if}

<main>
  {@render children()}
</main>

<style>
  .spacer {
    flex: 1;
  }
  nav.top :global(select.theme-select),
  nav.top :global(select.locale-select) {
    font: inherit;
    padding: 0.15rem 0.3rem;
    max-width: 11rem;
  }
</style>
