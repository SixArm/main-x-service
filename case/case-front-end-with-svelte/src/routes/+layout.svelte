<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import type { Snippet } from "svelte";

  let { children }: { children: Snippet } = $props();

  const navItems = [
    { href: "/", label: "Cases" },
    { href: "/new", label: "New case" },
  ];
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
</style>
