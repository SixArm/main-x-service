<!--
  Root layout: persistent sidebar (nav + session affordance) wrapping
  every route's content. Also the place the returning SSO token handoff
  is captured, before any child route fires an API call.

  $props:
    - children: Snippet — the active route's rendered output.

  $state:
    - draft: string — the manual "paste a token" input (dev fallback).

  Session reads `auth.token` reactively, so signing in/out re-renders the
  panel automatically. See `agents/share/jwt-enforcement.md`.
-->
<script lang="ts">
    import "../app.css";
    import { onMount } from "svelte";
    import { page } from "$app/state";
    import type { Snippet } from "svelte";
    import { auth, captureFromLocation } from "$lib/auth.svelte";
    import { signInUrl } from "$lib/config";

    let { children }: { children: Snippet } = $props();

    // Capture a returning SSO handoff (`…#access_token=<jwt>`) before any
    // route makes an API call, then strip the fragment. See
    // `agents/share/jwt-enforcement.md`.
    onMount(() => {
        captureFromLocation();
    });

    const navItems = [
        { href: "/", label: "Organizations" },
        { href: "/new", label: "New organization" },
    ];

    // Session affordance. Primary path: "Sign in" redirects to the
    // central authentication front-end, which hands the access token
    // back via the URL fragment (captured above). The manual paste field
    // is kept as a dev convenience. Service-side enforcement is off by
    // default.
    let draft = $state("");

    // Redirect to the central auth front-end; it returns via the URL
    // fragment, captured by `captureFromLocation` on the next mount.
    function signIn() {
        window.location.href = signInUrl();
    }
    // Dev fallback: store a hand-pasted token and clear the input.
    function saveToken() {
        auth.setToken(draft);
        draft = "";
    }
    // Sign out: drop the token (store + localStorage) and clear the draft.
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
                <button type="button" class="signin" onclick={signIn}>Sign in</button>
                <details class="paste">
                    <summary>Paste a token</summary>
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
                </details>
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
    .session button.signin {
        background: var(--mxi-accent, #356);
        color: #fff;
        border: none;
        font-weight: 600;
    }
    .session .paste summary {
        cursor: pointer;
        color: var(--mxi-muted, #555);
        margin-top: 0.2rem;
    }
    .session .paste {
        display: flex;
        flex-direction: column;
        gap: 0.4rem;
    }
</style>
