<!--
  Root layout for the auth SPA: a top navigation bar (brand, hamburger,
  nav, locale switcher, signed-in badge) wrapping the routed page content.

  Props:
  - `children`: Snippet — the active route, rendered via {@render children()}.

  State / reactivity:
  - reads `page.url.pathname` (reactive) to mark the active nav link;
  - reads `i18n.locale` / `t(...)` so the whole chrome re-renders on a
    locale switch;
  - reads `session.*` so the signed-in badge appears/disappears live.

  Events:
  - `onLocaleChange` — the locale <select> change handler.
-->
<script lang="ts">
    import "../app.css";
    import { browser } from "$app/environment";
    import { page } from "$app/state";
    import { i18n, t, isRtl, LOCALE_LABELS, type StringKey } from "$lib/i18n.svelte";
    import type { Snippet } from "svelte";
    import type { LayoutData } from "./$types";
    import { ThemeSelect } from "lily-design-system-svelte-theme-select";
    import { LocaleSelect } from "lily-design-system-svelte-locale-select";

    // Lily theme catalogue offered in the theme select (incl.
    // NHS England/Scotland/Wales patient & practitioner themes). Each slug
    // has a Lily stylesheet at `static/assets/themes/<slug>.css` (a symlink
    // to the shared design-system themes) that ThemeSelect swaps in.
    const THEMES = [
        "abyss", "acid", "aqua", "autumn", "black", "bumblebee", "business",
        "caramellatte", "cmyk", "coffee", "corporate", "cupcake", "cyberpunk",
        "dark", "dim", "dracula", "emerald", "fantasy", "forest", "garden",
        "halloween", "lemonade", "light", "lofi", "luxury", "night", "nord",
        "pastel", "retro", "silk", "sunset", "synthwave",
        "united-kingdom-national-health-service-england-for-patients",
        "united-kingdom-national-health-service-england-for-practitioners",
        "united-kingdom-national-health-service-scotland-for-patients",
        "united-kingdom-national-health-service-scotland-for-practitioners",
        "united-kingdom-national-health-service-wales-for-patients",
        "united-kingdom-national-health-service-wales-for-practitioners",
        "valentine", "winter", "wireframe"
    ];

    // Human-readable labels for the theme select — the FULL theme name for
    // each slug (DaisyUI names title-cased; the NHS slugs spelled out in full).
    const THEME_LABELS: Record<string, string> = {
        abyss: "Abyss", acid: "Acid", aqua: "Aqua", autumn: "Autumn",
        black: "Black", bumblebee: "Bumblebee", business: "Business",
        caramellatte: "Caramellatte", cmyk: "Cmyk", coffee: "Coffee",
        corporate: "Corporate", cupcake: "Cupcake", cyberpunk: "Cyberpunk",
        dark: "Dark", dim: "Dim", dracula: "Dracula", emerald: "Emerald",
        fantasy: "Fantasy", forest: "Forest", garden: "Garden",
        halloween: "Halloween", lemonade: "Lemonade", light: "Light",
        lofi: "Lofi", luxury: "Luxury", night: "Night", nord: "Nord",
        pastel: "Pastel", retro: "Retro", silk: "Silk", sunset: "Sunset",
        synthwave: "Synthwave", valentine: "Valentine", winter: "Winter",
        wireframe: "Wireframe",
        "united-kingdom-national-health-service-england-for-patients": "United Kingdom National Health Service England for Patients",
        "united-kingdom-national-health-service-england-for-practitioners": "United Kingdom National Health Service England for Practitioners",
        "united-kingdom-national-health-service-scotland-for-patients": "United Kingdom National Health Service Scotland for Patients",
        "united-kingdom-national-health-service-scotland-for-practitioners": "United Kingdom National Health Service Scotland for Practitioners",
        "united-kingdom-national-health-service-wales-for-patients": "United Kingdom National Health Service Wales for Patients",
        "united-kingdom-national-health-service-wales-for-practitioners": "United Kingdom National Health Service Wales for Practitioners",
    };

    // The routed page content + the layout load data (server-resolved
    // signed-in user from the httpOnly session cookie).
    let { children, data }: { children: Snippet; data: LayoutData } = $props();

    // The i18n store is the single source of truth for the locale: mirror it
    // onto `<html lang>` and `<html dir>` (rtl for ar/ur) whenever it changes.
    // SSR-guarded — `document` only exists in the browser.
    $effect(() => {
        const locale = i18n.locale;
        if (!browser || typeof document === "undefined") return;
        document.documentElement.lang = locale;
        document.documentElement.dir = isRtl(locale) ? "rtl" : "ltr";
    });

    // Hamburger toggle state for the top navigation bar (narrow viewports).
    let menuOpen = $state(false);

    // Top-bar nav entries; `key` is an i18n key so labels follow the locale.
    const navItems: { href: string; key: StringKey }[] = [
        { href: "/", key: "nav.home" },
        { href: "/signin", key: "nav.signin" },
        { href: "/signup", key: "nav.signup" },
    ];
</script>

<div class="layout">
    <header class="topbar">
        <button
            type="button"
            class="hamburger"
            aria-expanded={menuOpen}
            aria-controls="primary-nav"
            aria-label={t("nav.toggle")}
            onclick={() => (menuOpen = !menuOpen)}
        >
            <span class="hamburger-box" aria-hidden="true"></span>
        </button>
        <a href="/" class="brand">{t("brand")}</a>
        <nav id="primary-nav" class="primary-nav" class:open={menuOpen}>
            <ul>
                {#each navItems as item (item.href)}
                    <!-- Mark the link matching the current path as the active page. -->
                    <li>
                        <a
                            href={item.href}
                            aria-current={page.url.pathname === item.href ? "page" : undefined}
                            onclick={() => (menuOpen = false)}
                        >
                            {t(item.key)}
                        </a>
                    </li>
                {/each}
            </ul>
            <div class="chrome">
                <ThemeSelect
                    label={t("nav.theme")}
                    themesUrl="/assets/themes/"
                    themes={THEMES}
                    themeLabels={THEME_LABELS}
                    storageKey="lily-theme"
                />
                <label class="locale">
                    <span>{t("nav.locale")}</span>
                    <LocaleSelect
                        label={t("nav.locale")}
                        locales={[...i18n.locales]}
                        localeLabels={LOCALE_LABELS}
                        value={i18n.locale}
                        applyDir={false}
                        onChange={(code) => i18n.set(code)}
                    />
                </label>
                <!-- Signed-in badge: shown only when a session is present
                     (resolved server-side from the httpOnly cookie). -->
                {#if data.user}
                    <div class="who">
                        <small>{t("session.signedInAs")}</small>
                        <div>{data.user.email}</div>
                    </div>
                {/if}
            </div>
        </nav>
    </header>
    <main>
        <!-- Render the active route into the content column. -->
        {@render children()}
    </main>
</div>

<style>
    .layout {
        display: flex;
        flex-direction: column;
        min-height: 100vh;
    }
    .topbar {
        position: relative;
        display: flex;
        align-items: center;
        flex-wrap: wrap;
        gap: 1rem;
        padding: 0.75rem 1.5rem;
        background: var(--mxi-color-surface);
        border-bottom: 1px solid var(--mxi-color-border);
    }
    .brand {
        font-weight: 700;
        color: var(--mxi-color-fg);
        text-decoration: none;
        white-space: nowrap;
    }
    .hamburger {
        /* Always-visible toggle: the primary nav is collapsed behind it at
           every viewport width (not a responsive desktop-vs-mobile pattern). */
        display: block;
        width: 2.5rem;
        height: 2.5rem;
        padding: 0;
        background: transparent;
        border: 1px solid var(--mxi-color-border);
        border-radius: var(--mxi-radius);
        cursor: pointer;
    }
    .hamburger-box,
    .hamburger-box::before,
    .hamburger-box::after {
        display: block;
        width: 1.1rem;
        height: 2px;
        margin: 0 auto;
        background: var(--mxi-color-fg);
        content: "";
    }
    .hamburger-box::before { transform: translateY(-5px); }
    .hamburger-box::after { transform: translateY(3px); }
    .primary-nav {
        /* Always collapsed behind the hamburger: hidden by default at every
           width, shown only when the toggle adds `.open`. Rendered as a
           dropdown panel overlaying content (position:absolute) so opening it
           does not reflow the header. */
        display: none;
        position: absolute;
        top: 100%;
        left: 1.5rem;
        z-index: 20;
        flex-direction: column;
        align-items: stretch;
        gap: 0.75rem;
        min-width: 14rem;
        padding: 0.75rem;
        background: var(--mxi-color-surface);
        border: 1px solid var(--mxi-color-border);
        border-radius: var(--mxi-radius);
        box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
    }
    .primary-nav.open { display: flex; }
    .primary-nav ul {
        list-style: none;
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
        margin: 0;
        padding: 0;
    }
    .primary-nav a {
        display: block;
        text-decoration: none;
        padding: 0.5rem 0.625rem;
        border-radius: var(--mxi-radius);
        color: var(--mxi-color-fg);
    }
    .primary-nav a:hover { background: var(--mxi-color-bg); }
    .primary-nav a[aria-current="page"] {
        background: var(--mxi-color-primary);
        color: var(--mxi-color-primary-fg);
        font-weight: 600;
    }
    .chrome {
        display: flex;
        flex-direction: column;
        align-items: stretch;
        gap: 0.75rem;
    }
    .chrome :global(.theme-select) {
        padding: 0.375rem 0.5rem;
        font-size: 0.875rem;
        color: var(--mxi-color-fg);
        background: var(--mxi-color-bg, transparent);
        border: 1px solid var(--mxi-color-border);
        border-radius: 0.25rem;
        cursor: pointer;
    }
    .locale {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        font-size: 0.85rem;
    }
    .locale :global(select) {
        padding: 0.3rem 0.4rem;
        border-radius: var(--mxi-radius);
        border: 1px solid var(--mxi-color-border);
    }
    .who {
        font-size: 0.85rem;
        word-break: break-all;
        text-align: left;
    }
    main {
        width: 100%;
        padding: 1.5rem 2rem;
    }
</style>
