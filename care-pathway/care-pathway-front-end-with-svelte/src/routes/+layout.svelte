<script lang="ts">
    import "../app.css";
    import { onMount } from "svelte";
    import { page } from "$app/state";
    import type { Snippet } from "svelte";
    import { token, setToken, clearToken, captureFromLocation } from "$lib/auth.svelte";
    import { signInUrl } from "$lib/config";

    let { children }: { children: Snippet } = $props();

    // Capture a returning SSO handoff (`…#access_token=<jwt>`) before any
    // route makes an API call, then strip the fragment. See
    // `agents/share/jwt-enforcement.md`.
    onMount(() => {
        captureFromLocation();
    });

    const navItems = [
        { href: "/", label: "Care pathways" },
        { href: "/new", label: "New care pathway" },
    ];

    // Session affordance. Primary path: "Sign in" redirects to the
    // central authentication front-end, which hands the access token back
    // via the URL fragment (captured above). The manual paste field is
    // kept as a dev convenience. The stored token is attached as
    // `Authorization: Bearer <token>` to every API request (see
    // `$lib/auth.svelte` + `$lib/api/client`).
    let draft = $state("");

    function signIn(): void {
        window.location.href = signInUrl();
    }

    function applyToken(): void {
        const trimmed = draft.trim();
        if (trimmed.length > 0) {
            setToken(trimmed);
            draft = "";
        }
    }

    function signOut(): void {
        clearToken();
        draft = "";
    }
</script>

<div class="layout">
    <aside class="sidebar">
        <div class="brand">Main X · Care Pathways</div>
        <nav>
            {#each navItems as item (item.href)}
                <a href={item.href} aria-current={page.url.pathname === item.href ? "page" : undefined}>
                    {item.label}
                </a>
            {/each}
        </nav>
        <div class="session">
            <div class="session-title">Session</div>
            {#if token()}
                <div class="session-status" data-testid="session-status">Signed in</div>
                <button type="button" onclick={signOut}>Sign out</button>
            {:else}
                <button type="button" class="signin" onclick={signIn}>Sign in</button>
                <details class="paste">
                    <summary>Paste a token</summary>
                    <input
                        type="password"
                        placeholder="Paste access token"
                        aria-label="Access token"
                        bind:value={draft}
                    />
                    <button type="button" onclick={applyToken} disabled={draft.trim().length === 0}>
                        Use token
                    </button>
                    <p class="session-hint">
                        Token comes from the authentication-service (magic-link sign-in).
                    </p>
                </details>
            {/if}
        </div>
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
    }
    .session-title {
        font-size: 0.75rem;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        color: var(--mxi-muted, #666);
    }
    .session-status {
        font-size: 0.85rem;
    }
    .session input,
    .session button {
        font: inherit;
        padding: 0.35rem 0.5rem;
        border-radius: 6px;
        border: 1px solid var(--mxi-border, #ddd);
    }
    .session button {
        cursor: pointer;
    }
    .session button:disabled {
        cursor: not-allowed;
        opacity: 0.6;
    }
    .session button.signin {
        background: var(--mxi-accent, #356);
        color: #fff;
        border: none;
        font-weight: 600;
    }
    .session .paste {
        display: flex;
        flex-direction: column;
        gap: 0.4rem;
    }
    .session .paste summary {
        cursor: pointer;
        font-size: 0.8rem;
        color: var(--mxi-muted, #666);
    }
    .session-hint {
        margin: 0;
        font-size: 0.72rem;
        color: var(--mxi-muted, #666);
    }
</style>
