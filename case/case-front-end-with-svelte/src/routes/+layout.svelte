<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import type { Snippet } from "svelte";
  import { token, setToken, clearToken, captureFromLocation } from "$lib/auth.svelte";
  import { signInUrl } from "$lib/config";

  let { children }: { children: Snippet } = $props();

  // Capture a returning SSO handoff (`…#access_token=<jwt>`) and strip the
  // fragment before any route makes an API call. See
  // `agents/share/jwt-enforcement.md`.
  onMount(() => {
    captureFromLocation();
  });

  const navItems = [
    { href: "/", label: "Cases" },
    { href: "/new", label: "New case" },
  ];

  // Session affordance. Primary path: "Sign in" redirects to the central
  // authentication front-end, which hands the access token back via the
  // URL fragment (captured above). The manual paste field is kept as a dev
  // convenience. Service-side enforcement is off by default.
  let draft = $state("");
  const signedIn = $derived(token() !== null);

  function signIn(): void {
    window.location.href = signInUrl();
  }

  function applyToken(): void {
    const value = draft.trim();
    if (value.length > 0) {
      setToken(value);
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
    <div class="brand">Main X · Cases</div>
    <nav>
      {#each navItems as item (item.href)}
        <a
          href={item.href}
          aria-current={page.url.pathname === item.href ? "page" : undefined}
        >
          {item.label}
        </a>
      {/each}
    </nav>

    <div class="session">
      <div class="session-title small muted">Session</div>
      {#if signedIn}
        <p class="small" data-testid="session-status">Token attached.</p>
        <button class="button danger small" type="button" onclick={signOut}>
          Clear token
        </button>
      {:else}
        <p class="small" data-testid="session-status">No token.</p>
        <button class="button primary small signin" type="button" onclick={signIn}>
          Sign in
        </button>
        <details class="paste">
          <summary class="small muted">Paste a token</summary>
          <input
            class="session-input"
            type="password"
            placeholder="Paste access token"
            aria-label="Access token"
            bind:value={draft}
          />
          <button
            class="button small"
            type="button"
            onclick={applyToken}
            disabled={draft.trim().length === 0}
          >
            Use token
          </button>
        </details>
      {/if}
      <p class="small muted">
        From the authentication-service (magic-link sign-in).
      </p>
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
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .session-input {
    width: 100%;
    box-sizing: border-box;
  }
  .session .signin {
    width: 100%;
  }
  .session .paste {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .session .paste summary {
    cursor: pointer;
  }
</style>
