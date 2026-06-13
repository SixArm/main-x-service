<script lang="ts">
    import "../app.css";
    import { page } from "$app/state";
    import type { Snippet } from "svelte";
    import { auth } from "$lib/auth.svelte";

    let { children }: { children: Snippet } = $props();

    const navItems = [
        { href: "/", label: "Organizations" },
        { href: "/new", label: "New organization" },
    ];

    // Minimal session affordance. The access token is obtained
    // out-of-band from the central authentication-service (passwordless
    // magic-link); paste it here and it is attached to every API request
    // (see auth.svelte.ts). Full magic-link redirect wiring is a
    // follow-up; service-side enforcement is off by default.
    let draft = $state("");

    function saveToken() {
        auth.setToken(draft);
        draft = "";
    }
    function signOut() {
        auth.clearToken();
        draft = "";
    }
</script>

<div class="layout">
    <aside class="sidebar">
        <div class="brand">Main X · Organizations</div>
        <nav>
            {#each navItems as item (item.href)}
                <a href={item.href} aria-current={page.url.pathname === item.href ? "page" : undefined}>
                    {item.label}
                </a>
            {/each}
        </nav>
        <section class="session" aria-label="Session">
            <div class="session-title">Session</div>
            {#if auth.token}
                <p class="session-status">Signed in (token attached)</p>
                <button type="button" onclick={signOut}>Sign out</button>
            {:else}
                <label class="session-label" for="session-token">Access token</label>
                <input
                    id="session-token"
                    type="password"
                    placeholder="Paste bearer token"
                    bind:value={draft}
                    autocomplete="off"
                />
                <button type="button" onclick={saveToken} disabled={draft.trim().length === 0}>
                    Use token
                </button>
            {/if}
        </section>
    </aside>
    <main>{@render children()}</main>
</div>

<style>
    .layout {
        display: grid;
        grid-template-columns: 240px 1fr;
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
    main {
        padding: 1.5rem 2rem;
    }
    .session {
        margin-top: auto;
        padding-top: 1rem;
        border-top: 1px solid var(--mxi-border, #ddd);
        display: flex;
        flex-direction: column;
        gap: 0.4rem;
        font-size: 0.85rem;
    }
    .session-title {
        font-weight: 600;
    }
    .session-status {
        margin: 0;
        color: var(--mxi-muted, #555);
    }
    .session input {
        width: 100%;
        box-sizing: border-box;
        padding: 0.3rem 0.4rem;
        border: 1px solid var(--mxi-border, #ddd);
        border-radius: 6px;
    }
    .session button {
        padding: 0.3rem 0.5rem;
        border-radius: 6px;
        cursor: pointer;
    }
    .session button:disabled {
        cursor: not-allowed;
        opacity: 0.5;
    }
</style>
