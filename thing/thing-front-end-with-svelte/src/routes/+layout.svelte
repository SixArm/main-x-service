<!--
  +layout.svelte — root application shell wrapping every route.

  Purpose: renders the persistent sidebar (brand, primary navigation, theme
  picker, locale picker) and a <main> region into which the active page is
  rendered via the `children` snippet.

  $props:
    - children (Snippet): the active route's content.

  Reactive notes: nav items compare against `page.url.pathname` to set
  aria-current="page" on the active link.

  Lists THEMES / LOCALES feed the Lily theme/locale pickers; they are static
  config arrays, not reactive state.
-->
<script lang="ts">
    import "../app.css";
    import { page } from "$app/state";
    import type { Snippet } from "svelte";
    import ThemePicker from "lily-design-system-svelte-theme-picker/ThemePicker.svelte";

    // Theme ids offered by the Lily ThemePicker (loaded from /assets/themes/).
    const THEMES = [
        "abyss",
        "acid",
        "aqua",
        "autumn",
        "black",
        "bumblebee",
        "business",
        "caramellatte",
        "cmyk",
        "coffee",
        "corporate",
        "cupcake",
        "cyberpunk",
        "dark",
        "dim",
        "dracula",
        "emerald",
        "fantasy",
        "forest",
        "garden",
        "halloween",
        "lemonade",
        "light",
        "lofi",
        "luxury",
        "night",
        "nord",
        "pastel",
        "retro",
        "silk",
        "sunset",
        "synthwave",
        "united-kingdom-national-health-service-england-for-patients",
        "united-kingdom-national-health-service-england-for-practitioners",
        "united-kingdom-national-health-service-scotland-for-patients",
        "united-kingdom-national-health-service-scotland-for-practitioners",
        "united-kingdom-national-health-service-wales-for-patients",
        "united-kingdom-national-health-service-wales-for-practitioners",
        "valentine",
        "winter",
        "wireframe"
    ];

    import LocalePicker from "lily-design-system-svelte-locale-picker/LocalePicker.svelte";

    // BCP-47-ish locale codes offered by the Lily LocalePicker.
    const LOCALES = [
        "ar",
        "cy",
        "de",
        "en",
        "en_GB",
        "en_US",
        "es",
        "fa",
        "fr",
        "fr_CA",
        "he",
        "hi",
        "it",
        "ja",
        "ko",
        "nl",
        "pl",
        "pt",
        "pt_BR",
        "ru",
        "sv",
        "tr",
        "ur",
        "zh",
        "zh_TW"
    ];

    // Lily headless example — uncomment after `pnpm install` resolves the
    // file: dependency to use Lily's accessibility-primitive Button:
    // import Button from "lily-design-system-svelte-headless/src/lib/components/Button/Button.svelte";

    let { children }: { children: Snippet } = $props();

    // Primary sidebar navigation (href + visible label).
    const navItems = [
        { href: "/", label: "Dashboard" },
        { href: "/things", label: "Things" },
        { href: "/things/new", label: "New thing" },
        { href: "/things/match", label: "Match check" },
        { href: "/things/merge", label: "Merge" },
    ];
</script>

<div class="layout">
    <aside class="sidebar">
        <h1 class="brand">Thing<br /><span class="muted small">Main X Index</span></h1>
        <nav>
            <ul>
                {#each navItems as item}
                    <li>
                        <!-- Mark the link for the current path as the active page. -->
                        <a
                            href={item.href}
                            aria-current={page.url.pathname === item.href ? "page" : null}
                        >
                            {item.label}
                        </a>
                    </li>
                {/each}
            </ul>
        </nav>
        <div class="theme-section">
            <ThemePicker
                label="Theme"
                themesUrl="/assets/themes/"
                themes={THEMES}
                storageKey="lily-theme"
            />
        </div>
        <div class="locale-section">
            <LocalePicker
                label="Language"
                locales={LOCALES}
                defaultValue="en"
                storageKey="lily-locale"
                detectFromNavigator
            />
        </div>
    </aside>
    <main>
        {@render children()}
    </main>
</div>

<style>
    .layout {
        display: grid;
        grid-template-columns: 220px 1fr;
        min-height: 100vh;
    }
    .sidebar {
        background: var(--mxi-color-surface);
        border-right: 1px solid var(--mxi-color-border);
        padding: 1.25rem 1rem;
    }
    .brand { font-size: 1.125rem; margin-bottom: 1.25rem; }
    nav ul { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0.125rem; }
    nav a {
        display: block;
        padding: 0.5rem 0.625rem;
        border-radius: var(--mxi-radius);
        color: var(--mxi-color-fg);
    }
    nav a:hover { background: var(--mxi-color-bg); text-decoration: none; }
    nav a[aria-current="page"] {
        background: var(--mxi-color-primary);
        color: var(--mxi-color-primary-fg);
        font-weight: 600;
    }
    main { padding: 1.5rem; max-width: 1100px; width: 100%; }

    .theme-section {
        margin-top: 1.25rem;
        padding-top: 1rem;
        border-top: 1px solid var(--mxi-color-border);
    }
    .theme-section :global(.theme-picker) {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
        margin: 0;
        padding: 0;
        border: 0;
    }
    .theme-section :global(.theme-picker legend) {
        font-size: 0.875rem;
        color: var(--mxi-color-fg);
        margin-bottom: 0.25rem;
    }
    .theme-section :global(.theme-picker label) {
        display: flex;
        align-items: center;
        gap: 0.375rem;
        font-size: 0.875rem;
        color: var(--mxi-color-fg);
    }

    .locale-section {
        margin-top: 0.75rem;
    }
    .locale-section :global(fieldset) {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
        margin: 0;
        padding: 0;
        border: 0;
    }
    .locale-section :global(legend) {
        font-size: 0.875rem;
        color: var(--mxi-color-fg);
        margin-bottom: 0.25rem;
    }
    .locale-section :global(label) {
        display: flex;
        align-items: center;
        gap: 0.375rem;
        font-size: 0.875rem;
        color: var(--mxi-color-fg);
    }
</style>
