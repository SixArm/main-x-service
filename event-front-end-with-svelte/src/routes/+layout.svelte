<script lang="ts">
    import "../app.css";
    import { page } from "$app/state";
    import type { Snippet } from "svelte";

    // Lily headless example — uncomment after `pnpm install` resolves the
    // file: dependency to use Lily's accessibility-primitive Button:
    // import Button from "lily-design-system-svelte-headless/src/lib/components/Button/Button.svelte";

    let { children }: { children: Snippet } = $props();

    const navItems = [
        { href: "/", label: "Dashboard" },
        { href: "/events", label: "Events" },
        { href: "/events/new", label: "New event" },
        { href: "/events/match", label: "Match check" },
        { href: "/events/merge", label: "Merge" },
    ];
</script>

<div class="layout">
    <aside class="sidebar">
        <h1 class="brand">Event<br /><span class="muted small">Main X Index</span></h1>
        <nav>
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
</style>
