<!--
  Root layout — the site shell wrapping every route.

  Renders the persistent top navigation bar (brand, primary nav, the Lily
  theme picker) and a <main> slot for the active page, plus a footer
  linking back to the monorepo. Pure shell: this site has no data
  fetching, no auth, and no forms — it is a read-only public front door
  onto the main-x-service monorepo, not one of the family's operator
  front-ends.

  Props:
    - children: Snippet — the active route's content, rendered in <main>.
-->
<script lang="ts">
    import { page } from "$app/state";
    import type { Snippet } from "svelte";
    import { ThemePicker } from "lily-design-system-svelte-theme-picker";

    // Same theme catalogue as the family's operator front-ends (DaisyUI
    // themes plus the bespoke NHS England/Scotland/Wales themes) — each
    // slug has a Lily stylesheet at static/assets/themes/<slug>.css (a
    // symlink to the shared design-system themes) that ThemePicker swaps.
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

    let { children }: { children: Snippet } = $props();

    const navItems = [
        { href: "/", label: "Home" },
        { href: "/architecture/", label: "Architecture" },
        { href: "/subprojects/", label: "Subprojects" },
        { href: "/about/", label: "About" },
    ] as const;
</script>

<div class="layout">
    <header class="topbar">
        <a href="/" class="brand">Main X Index</a>
        <nav class="primary-nav">
            <ul>
                {#each navItems as item}
                    <li>
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
        <div class="chrome">
            <ThemePicker
                label="Theme"
                themesUrl="/assets/themes/"
                themes={THEMES}
                storageKey="mxi-github-io-theme"
            />
        </div>
    </header>
    <main>
        {@render children()}
    </main>
    <footer>
        <p>
            <a href="https://github.com/SixArm/main-x-service">GitHub</a> ·
            <a href="https://codeberg.org/SixArm/main-x-service">Codeberg</a> ·
            source published from the
            <code>main-x-service.github.io/</code> subproject via
            <code>git subtree</code>.
        </p>
    </footer>
</div>

<style>
    .layout {
        display: flex;
        flex-direction: column;
        min-height: 100vh;
    }
    .topbar {
        display: flex;
        align-items: center;
        flex-wrap: wrap;
        gap: 1.5rem;
        padding: 0.75rem 1.5rem;
        background: var(--mxi-color-surface);
        border-bottom: 1px solid var(--mxi-color-border);
    }
    .brand {
        font-size: 1.125rem;
        font-weight: 700;
        color: var(--mxi-color-fg);
        white-space: nowrap;
        text-decoration: none;
    }
    .primary-nav {
        flex: 1;
    }
    .primary-nav ul {
        list-style: none;
        display: flex;
        gap: 1.25rem;
        margin: 0;
        padding: 0;
    }
    .primary-nav a {
        color: var(--mxi-color-fg);
        text-decoration: none;
    }
    .primary-nav a:hover {
        color: var(--mxi-color-primary);
    }
    .primary-nav a[aria-current="page"] {
        color: var(--mxi-color-primary);
        font-weight: 600;
    }
    .chrome :global(.theme-picker-button) {
        padding: 0.375rem 0.5rem;
        font-size: 0.875rem;
        color: var(--mxi-color-fg);
        background: transparent;
        border: 1px solid var(--mxi-color-border);
        border-radius: 0.25rem;
        cursor: pointer;
    }
    main {
        flex: 1;
        width: 100%;
        max-width: 60rem;
        margin: 0 auto;
        padding: 2rem 1.5rem;
    }
    footer {
        padding: 1.5rem;
        text-align: center;
        font-size: 0.85rem;
        color: var(--mxi-color-muted);
        border-top: 1px solid var(--mxi-color-border);
    }
</style>
