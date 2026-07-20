<script lang="ts">
    // Root layout — the app shell wrapping every route.
    //
    // Purpose: render the top navigation bar (brand, nav, session controls) and the
    // active route via the `children` snippet.
    //
    // Props ($props): `children: Snippet` — the active page, rendered with
    // `{@render children()}`; `data: LayoutData` — carries `signedIn`,
    // resolved server-side from the httpOnly session cookie
    // (`+layout.server.ts`), so the chrome shows signed-in state without the
    // browser ever holding a token. See `agents/share/authentication-sessions.md`.
    //
    // Events: nav links are plain anchors; sign-out posts to the root page's
    // `signout` action (BFF); sign-in links to this app's own `/signin`.
    import "../app.css";
    import { browser } from "$app/environment";
    import { page } from "$app/state";
    import { enhance } from "$app/forms";
    import type { Snippet } from "svelte";
    import type { LayoutData } from "./$types";
    import { i18n, t, LOCALE_LABELS, isRtl } from "$lib/i18n.svelte";
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

    // `data.signedIn` is resolved server-side from the httpOnly session
    // cookie (`+layout.server.ts`).
    let { children, data }: { children: Snippet; data: LayoutData } = $props();

    // Hamburger toggle state for the top navigation bar (narrow viewports).
    let menuOpen = $state(false);

    // The i18n store is the single source of truth for the locale: this
    // effect mirrors it onto `<html lang>` / `<html dir>` (RTL for ar/ur).
    // SSR-guarded — only touches the document in the browser.
    $effect(() => {
        if (!browser) return;
        document.documentElement.lang = i18n.locale;
        document.documentElement.dir = isRtl(i18n.locale) ? "rtl" : "ltr";
    });

    // Nav items reference i18n keys; labels are resolved reactively in markup.
    const navItems = [
        { href: "/", key: "nav.pathways" as const },
        { href: "/new", key: "nav.newCarePathway" as const },
        { href: "/insights", key: "nav.insights" as const },
        { href: "/board", key: "nav.board" as const },
        { href: "/gantt", key: "nav.gantt" as const },
        { href: "/sequence", key: "nav.sequence" as const },
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
        <a href="/" class="brand">{t("brand.full")}</a>
        <nav id="primary-nav" class="primary-nav" class:open={menuOpen}>
            <ul>
                {#each navItems as item (item.href)}
                    <li>
                        <a href={item.href} aria-current={page.url.pathname === item.href ? "page" : undefined} onclick={() => (menuOpen = false)}>
                            {t(item.key)}
                        </a>
                    </li>
                {/each}
            </ul>
            <!-- Theme switcher: Lily ThemeSelect swaps the active theme
                 stylesheet (from static/assets/themes/<slug>.css) and persists
                 the choice. The app's design tokens bridge onto the theme's
                 `--color-*` tokens in app.css, so this restyles the whole UI. -->
            <ThemeSelect
                label={t("chrome.theme")}
                themesUrl="/assets/themes/"
                themes={THEMES}
                themeLabels={THEME_LABELS}
                storageKey="lily-theme"
            />
            <!-- Locale switcher: the Lily LocaleSelect. The i18n store
                 stays the single source of truth; the select reflects
                 value={i18n.locale} and writes back via onChange. -->
            <label class="locale">
                <span class="locale-label">{t("chrome.language")}</span>
                <LocaleSelect
                    label={t("chrome.language")}
                    locales={[...i18n.locales]}
                    localeLabels={LOCALE_LABELS}
                    value={i18n.locale}
                    applyDir={false}
                    onChange={(code) => i18n.set(code)}
                />
            </label>
            <!--
                Session panel. `data.signedIn` is server-resolved from the
                httpOnly session cookie: signed in shows a status badge + a
                Sign out form (posts to the BFF `signout` action); signed out
                links to this app's own per-app magic-link `/signin`.
            -->
            <div class="session">
                <div class="session-title">{t("session.title")}</div>
                {#if data.signedIn}
                    <div class="session-status" data-testid="session-status">{t("session.signedIn")}</div>
                    <!-- Sign-out posts to the root page's `signout` action
                         (BFF: revokes server-side + clears the cookie). -->
                    <form method="POST" action="/?/signout" use:enhance>
                        <button type="submit">{t("session.signOut")}</button>
                    </form>
                {:else}
                    <!-- Per-app magic-link login on this app's own origin. -->
                    <a class="signin" href="/signin">{t("session.signIn")}</a>
                {/if}
            </div>
        </nav>
    </header>
    <main>{@render children()}</main>
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
        border-bottom: 1px solid var(--mxi-color-border, #ddd);
    }
    .brand {
        font-weight: 700;
        color: inherit;
        text-decoration: none;
        white-space: nowrap;
    }
    .hamburger {
        /* Always visible at every width: the primary nav is always collapsed
           behind this toggle (not a responsive show-full-nav-on-desktop
           pattern). */
        display: block;
        width: 2.5rem;
        height: 2.5rem;
        padding: 0;
        background: transparent;
        border: 1px solid var(--mxi-color-border, #ddd);
        border-radius: 6px;
        cursor: pointer;
    }
    .hamburger-box,
    .hamburger-box::before,
    .hamburger-box::after {
        display: block;
        width: 1.1rem;
        height: 2px;
        margin: 0 auto;
        background: currentColor;
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
        min-width: 16rem;
        padding: 0.75rem;
        background: var(--mxi-color-surface, #fff);
        border: 1px solid var(--mxi-color-border, #ddd);
        border-radius: var(--mxi-radius, 6px);
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
        padding: 0.4rem 0.5rem;
        border-radius: 6px;
        color: inherit;
    }
    .primary-nav a[aria-current="page"] {
        background: var(--mxi-color-primary, #2563eb);
        color: var(--mxi-color-primary-fg, #fff);
        font-weight: 600;
    }
    main {
        width: 100%;
        padding: 1.5rem 2rem;
    }
    .locale {
        display: flex;
        flex-direction: column;
        align-items: stretch;
        gap: 0.4rem;
    }
    .locale-label {
        font-size: 0.75rem;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        color: var(--mxi-color-muted, #666);
    }
    .locale :global(select) {
        font: inherit;
        padding: 0.35rem 0.5rem;
        border-radius: 6px;
        border: 1px solid var(--mxi-color-border, #ddd);
        background: var(--mxi-color-bg, transparent);
        color: inherit;
    }
    .session {
        display: flex;
        flex-direction: column;
        align-items: stretch;
        gap: 0.5rem;
        padding-top: 0.5rem;
        border-top: 1px solid var(--mxi-color-border, #ddd);
    }
    .session-title {
        font-size: 0.75rem;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        color: var(--mxi-color-muted, #666);
    }
    .session-status {
        font-size: 0.85rem;
    }
    .session button {
        font: inherit;
        padding: 0.35rem 0.5rem;
        border-radius: 6px;
        border: 1px solid var(--mxi-color-border, #ddd);
        cursor: pointer;
    }
    .session .signin {
        display: inline-block;
        padding: 0.35rem 0.5rem;
        border-radius: 6px;
        background: var(--mxi-color-primary, #356);
        color: var(--mxi-color-primary-fg, #fff);
        text-decoration: none;
        font-weight: 600;
    }
    /* Theme select sits in the dropdown panel like the other chrome controls. */
    .primary-nav :global(.theme-select) {
        font: inherit;
        padding: 0.35rem 0.5rem;
        border-radius: 6px;
        border: 1px solid var(--mxi-color-border, #ddd);
        background: var(--mxi-color-bg, transparent);
        color: inherit;
    }
</style>
