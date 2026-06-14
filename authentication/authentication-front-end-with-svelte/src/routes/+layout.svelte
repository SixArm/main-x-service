<!--
  Root layout for the auth SPA: a fixed sidebar (brand, nav, locale
  switcher, signed-in badge) wrapping the routed page content.

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
    import { page } from "$app/state";
    import { session } from "$lib/auth/session.svelte";
    import { i18n, t, LOCALE_LABELS, type StringKey } from "$lib/i18n.svelte";
    import type { Snippet } from "svelte";

    // The routed page content, injected by SvelteKit.
    let { children }: { children: Snippet } = $props();

    // Sidebar nav entries; `key` is an i18n key so labels follow the locale.
    const navItems: { href: string; key: StringKey }[] = [
        { href: "/", key: "nav.home" },
        { href: "/signin", key: "nav.signin" },
        { href: "/signup", key: "nav.signup" },
    ];

    // Locale <select> handler: push the chosen value into the i18n store,
    // which persists it and re-renders every translated string.
    function onLocaleChange(event: Event) {
        i18n.set((event.currentTarget as HTMLSelectElement).value);
    }
</script>

<div class="layout">
    <aside class="sidebar">
        <div class="brand">{t("brand")}</div>
        <nav>
            {#each navItems as item (item.href)}
                <!-- Mark the link matching the current path as the active page. -->
                <a
                    href={item.href}
                    aria-current={page.url.pathname === item.href ? "page" : undefined}
                >
                    {t(item.key)}
                </a>
            {/each}
        </nav>
        <label class="locale">
            <span>{t("nav.locale")}</span>
            <select
                aria-label={t("nav.locale")}
                value={i18n.locale}
                onchange={onLocaleChange}
            >
                {#each i18n.locales as locale (locale)}
                    <option value={locale}>{LOCALE_LABELS[locale]}</option>
                {/each}
            </select>
        </label>
        <!-- Signed-in badge: shown only when a session is present. -->
        {#if session.isAuthenticated && session.user}
            <div class="who">
                <small>{t("session.signedInAs")}</small>
                <div>{session.user.email}</div>
            </div>
        {/if}
    </aside>
    <main>
        <!-- Render the active route into the content column. -->
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
        border-right: 1px solid var(--mxi-border, #ddd);
        padding: 1rem;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
    .brand {
        font-weight: 700;
        margin-bottom: 1rem;
    }
    nav {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }
    nav a {
        text-decoration: none;
        padding: 0.4rem 0.5rem;
        border-radius: 6px;
        color: inherit;
    }
    nav a[aria-current="page"] {
        background: var(--mxi-accent-soft, #eef);
        font-weight: 600;
    }
    .locale {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
        margin-top: 1rem;
        font-size: 0.85rem;
    }
    .locale select {
        padding: 0.3rem 0.4rem;
        border-radius: 6px;
        border: 1px solid var(--mxi-border, #ddd);
    }
    .who {
        margin-top: auto;
        font-size: 0.85rem;
        word-break: break-all;
    }
    main {
        padding: 1.5rem 2rem;
    }
</style>
