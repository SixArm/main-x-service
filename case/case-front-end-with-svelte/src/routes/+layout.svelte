<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import type { Snippet } from "svelte";
  import { token, setToken, clearToken } from "$lib/auth.svelte";

  let { children }: { children: Snippet } = $props();

  const navItems = [
    { href: "/", label: "Cases" },
    { href: "/new", label: "New case" },
  ];

  // Minimal session affordance: paste / clear the bearer token the API
  // client attaches to every request. The token is issued by the central
  // authentication-service (passwordless magic-link); full redirect wiring
  // is a follow-up, so for now an operator pastes it here.
  let draft = $state("");
  const signedIn = $derived(token() !== null);

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
</style>
