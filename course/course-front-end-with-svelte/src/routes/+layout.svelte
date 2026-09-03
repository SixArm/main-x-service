<!--
  Root layout — the app shell wrapping every route: a top navigation bar with the
  brand, primary nav, and Lily theme/locale pickers, plus the routed
  page content rendered via `children`. Imports global app.css.

  $props:
    - children: Snippet — the active route's rendered output.

  Reactive bits: nav links compare against `page.url.pathname` to set
  `aria-current="page"` on the active item.
-->
<script lang="ts">
    import "../app.css";
    import { page } from "$app/state";
    import { enhance } from "$app/forms";
    import type { Snippet } from "svelte";
    import type { LayoutData } from "./$types";
    import { ThemePicker } from "lily-design-system-svelte-theme-picker";

    // Lily theme catalogue offered in the theme select (incl.
    // NHS England/Scotland/Wales patient & practitioner themes). Each slug
    // has a Lily stylesheet at `static/assets/themes/<slug>.css` (a symlink
    // to the shared design-system themes) that ThemePicker swaps in.
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
        "wireframe",
    ];

    // Human-readable labels for the theme select — the FULL theme name for
    // each slug (DaisyUI names title-cased; the NHS slugs spelled out in full).
    const THEME_LABELS: Record<string, string> = {
        abyss: "Abyss",
        acid: "Acid",
        aqua: "Aqua",
        autumn: "Autumn",
        black: "Black",
        bumblebee: "Bumblebee",
        business: "Business",
        caramellatte: "Caramellatte",
        cmyk: "Cmyk",
        coffee: "Coffee",
        corporate: "Corporate",
        cupcake: "Cupcake",
        cyberpunk: "Cyberpunk",
        dark: "Dark",
        dim: "Dim",
        dracula: "Dracula",
        emerald: "Emerald",
        fantasy: "Fantasy",
        forest: "Forest",
        garden: "Garden",
        halloween: "Halloween",
        lemonade: "Lemonade",
        light: "Light",
        lofi: "Lofi",
        luxury: "Luxury",
        night: "Night",
        nord: "Nord",
        pastel: "Pastel",
        retro: "Retro",
        silk: "Silk",
        sunset: "Sunset",
        synthwave: "Synthwave",
        valentine: "Valentine",
        winter: "Winter",
        wireframe: "Wireframe",
        "united-kingdom-national-health-service-england-for-patients":
            "United Kingdom National Health Service England for Patients",
        "united-kingdom-national-health-service-england-for-practitioners":
            "United Kingdom National Health Service England for Practitioners",
        "united-kingdom-national-health-service-scotland-for-patients":
            "United Kingdom National Health Service Scotland for Patients",
        "united-kingdom-national-health-service-scotland-for-practitioners":
            "United Kingdom National Health Service Scotland for Practitioners",
        "united-kingdom-national-health-service-wales-for-patients":
            "United Kingdom National Health Service Wales for Patients",
        "united-kingdom-national-health-service-wales-for-practitioners":
            "United Kingdom National Health Service Wales for Practitioners",
    };

    import { LocalePicker } from "lily-design-system-svelte-locale-picker";
    import { browser } from "$app/environment";
    import {
        i18n,
        t,
        isRtl,
        LOCALE_LABELS,
        LOCALE_KEY,
        type StringKey,
    } from "$lib/i18n.svelte.js";

    // Locale codes offered in the language select — sourced from the i18n
    // store (all 13 translated locales) so the select can never drift to a
    // hardcoded subset. The i18n store is the single source of truth;
    // LocalePicker just reflects it and reports the user's choice via
    // `onChange`.
    const LOCALES = [...i18n.locales];

    // Lily headless example — uncomment after `pnpm install` resolves the
    // file: dependency to use Lily's accessibility-primitive Button:
    // import Button from "lily-design-system-svelte-headless/src/lib/components/Button/Button.svelte";

    // `data.signedIn` is resolved server-side from the httpOnly session
    // cookie (`+layout.server.ts`).
    let { children, data }: { children: Snippet; data: LayoutData } = $props();
    const signedIn = $derived(data.signedIn);

    // Hamburger toggle state for the top navigation bar (narrow viewports).
    let menuOpen = $state(false);

    // Locale selection bound to LocalePicker; seeded from the i18n store.
    // svelte-ignore state_referenced_locally

    // Reflect the active locale onto <html lang> and <html dir> (guarded for
    // SSR). RTL locales (ar / ur) flip the document direction to "rtl".
    $effect(() => {
        if (browser) {
            document.documentElement.lang = i18n.locale;
            document.documentElement.dir = isRtl(i18n.locale) ? "rtl" : "ltr";
        }
    });

    // Primary navigation (top bar): href ⇒ active route, label key ⇒ link text.
    const navItems: { href: string; labelKey: StringKey }[] = [
        { href: "/", labelKey: "nav.dashboard" },
        { href: "/courses", labelKey: "nav.courses" },
        { href: "/courses/new", labelKey: "nav.newCourse" },
        { href: "/courses/match", labelKey: "nav.matchCheck" },
        { href: "/courses/merge", labelKey: "nav.merge" },
        { href: "/calendar", labelKey: "nav.calendar" },
        { href: "/board", labelKey: "nav.board" },
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
        <a href="/" class="brand"
            >{t("brand.name")}
            <span class="muted small">{t("brand.tagline")}</span></a
        >
        <nav id="primary-nav" class="primary-nav" class:open={menuOpen}>
            <ul>
                {#each navItems as item}
                    <li>
                        <a
                            href={item.href}
                            aria-current={page.url.pathname === item.href
                                ? "page"
                                : null}
                            onclick={() => (menuOpen = false)}
                        >
                            {t(item.labelKey)}
                        </a>
                    </li>
                {/each}
            </ul>
            <div class="chrome">
                <ThemePicker
                    label={t("chrome.theme")}
                    themesUrl="/assets/themes/"
                    themes={THEMES}
                    themeLabels={THEME_LABELS}
                    storageKey="lily-theme"
                />
                <LocalePicker
                    label={t("chrome.language")}
                    locales={LOCALES}
                    localeLabels={LOCALE_LABELS}
                    value={i18n.locale}
                    applyDir={false}
                    onChange={(code) => i18n.set(code)}
                />
                <section class="session" aria-label="Session">
                    <div class="session-title">Session</div>
                    {#if signedIn}
                        <p class="session-status">Signed in</p>
                        <!-- Sign-out posts to the root page's `signout`
                             action (BFF: revokes server-side + clears
                             the cookie). -->
                        <form method="POST" action="/?/signout" use:enhance>
                            <button type="submit">Sign out</button>
                        </form>
                    {:else}
                        <!-- Per-app magic-link login on this app's own origin. -->
                        <a class="signin button" href="/signin">Sign in</a>
                    {/if}
                </section>
            </div>
        </nav>
    </header>
    <main>
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
        font-size: 1.125rem;
        font-weight: 600;
        color: var(--mxi-color-fg);
        white-space: nowrap;
    }
    .brand:hover {
        text-decoration: none;
    }
    .hamburger {
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
    .hamburger-box::before {
        transform: translateY(-5px);
    }
    .hamburger-box::after {
        transform: translateY(3px);
    }
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
        gap: 0.5rem;
        min-width: 14rem;
        padding: 0.75rem;
        background: var(--mxi-color-surface);
        border: 1px solid var(--mxi-color-border);
        border-radius: var(--mxi-radius);
        box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
    }
    .primary-nav.open {
        display: flex;
    }
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
        padding: 0.5rem 0.625rem;
        border-radius: var(--mxi-radius);
        color: var(--mxi-color-fg);
    }
    .primary-nav a:hover {
        background: var(--mxi-color-bg);
        text-decoration: none;
    }
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
    .chrome :global(.theme-picker-button),
    .chrome :global(.locale-picker-button) {
        padding: 0.375rem 0.5rem;
        font-size: 0.875rem;
        color: var(--mxi-color-fg);
        background: var(--mxi-color-bg, transparent);
        border: 1px solid var(--mxi-color-border);
        border-radius: 0.25rem;
        cursor: pointer;
    }
    main {
        width: 100%;
        padding: 1.5rem;
    }
    .session {
        display: flex;
        flex-direction: column;
        align-items: stretch;
        gap: 0.5rem;
        padding-top: 0.5rem;
        border-top: 1px solid var(--mxi-color-border);
        font-size: 0.85rem;
    }
    .session-title {
        font-weight: 600;
    }
    .session-status {
        margin: 0;
        color: var(--mxi-color-muted, #555);
    }
    .session button {
        padding: 0.3rem 0.5rem;
        border-radius: var(--mxi-radius);
        cursor: pointer;
    }
    .session .signin {
        display: inline-block;
        padding: 0.3rem 0.5rem;
        border-radius: var(--mxi-radius);
        background: var(--mxi-color-primary);
        color: var(--mxi-color-primary-fg);
        text-decoration: none;
        font-weight: 600;
    }
</style>
