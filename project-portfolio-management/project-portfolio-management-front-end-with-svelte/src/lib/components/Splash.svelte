<!--
  Splash — the welcoming home page shown to visitors who are not signed in.

  A hero with a sign-in call to action, then three tile areas (benefits,
  features, trust), each exactly six tiles so the grid divides evenly at
  every width: 1x6 on a phone, 2x3, 3x2, and 6x1 on a wide screen. All copy
  comes from the i18n catalog (`splash.*` keys).

  No props and no data fetching — purely presentational.
-->
<script lang="ts">
  import { t, type StringKey } from "#lib/i18n.svelte.js";

  const TILES = [1, 2, 3, 4, 5, 6] as const;
  const AREAS = [
    { id: "benefits", title: "splash.benefits.title" },
    { id: "features", title: "splash.features.title" },
    { id: "trust", title: "splash.trust.title" },
  ] as const;
</script>

<svelte:head><title>{t("brand.name")}</title></svelte:head>

<div class="splash">
  <section class="hero" aria-labelledby="hero-title">
    <p class="eyebrow">{t("brand.name")}</p>
    <h1 id="hero-title">{t("splash.hero.title")}</h1>
    <p class="lead">{t("splash.hero.subtitle")}</p>
    <div class="actions">
      <a class="cta primary" href="/signin">{t("auth.signin")}</a>
      <a class="cta secondary" href="/tour">{t("splash.hero.tour")}</a>
    </div>
  </section>

  {#each AREAS as area}
    <section class="area" id={area.id} aria-labelledby={`${area.id}-title`}>
      <h2 id={`${area.id}-title`}>{t(area.title as StringKey)}</h2>
      <ul class="tiles">
        {#each TILES as n}
          <li class="tile">
            <span class="badge" aria-hidden="true">{n}</span>
            <h3>
              {t(`splash.${area.id}.${n}.title` as StringKey)}
            </h3>
            <p>{t(`splash.${area.id}.${n}.body` as StringKey)}</p>
          </li>
        {/each}
      </ul>
    </section>
  {/each}

  <section class="closing" aria-labelledby="closing-title">
    <h2 id="closing-title">{t("splash.cta.title")}</h2>
    <p>{t("splash.cta.body")}</p>
    <a class="cta primary" href="/signin">{t("auth.signin")}</a>
  </section>
</div>

<style>
  .splash {
    display: flex;
    flex-direction: column;
    gap: 2.5rem;
    max-width: 90rem;
    margin: 0 auto;
  }
  .hero {
    padding: clamp(2rem, 6vw, 4.5rem) clamp(1.25rem, 5vw, 4rem);
    border-radius: calc(var(--mxi-radius) * 2);
    background: var(--mxi-color-primary);
    color: var(--mxi-color-primary-fg);
    text-align: center;
  }
  .eyebrow {
    margin: 0 0 0.75rem;
    font-size: 0.875rem;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    opacity: 0.85;
  }
  .hero h1 {
    margin: 0 auto 1rem;
    max-width: 24ch;
    font-size: clamp(2rem, 5vw, 3.25rem);
    line-height: 1.1;
  }
  .lead {
    margin: 0 auto 1.75rem;
    max-width: 48rem;
    font-size: clamp(1rem, 2vw, 1.25rem);
    line-height: 1.5;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.75rem;
  }
  .cta {
    display: inline-block;
    padding: 0.65rem 1.4rem;
    border-radius: var(--mxi-radius);
    font-weight: 600;
    text-decoration: none;
  }
  .cta:hover {
    text-decoration: none;
    filter: brightness(1.08);
  }
  .hero .cta.primary {
    background: var(--mxi-color-primary-fg);
    color: var(--mxi-color-primary);
  }
  .hero .cta.secondary {
    border: 1px solid currentColor;
    color: inherit;
  }
  .area h2,
  .closing h2 {
    margin: 0 0 1rem;
    text-align: center;
    font-size: clamp(1.4rem, 3vw, 2rem);
  }
  .tiles {
    display: grid;
    grid-template-columns: 1fr;
    gap: 1rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  /* Six tiles divide evenly at each step: 1x6, 2x3, 3x2, 6x1. */
  @media (min-width: 36rem) {
    .tiles {
      grid-template-columns: repeat(2, 1fr);
    }
  }
  @media (min-width: 60rem) {
    .tiles {
      grid-template-columns: repeat(3, 1fr);
    }
  }
  @media (min-width: 96rem) {
    .tiles {
      grid-template-columns: repeat(6, 1fr);
    }
  }
  .tile {
    padding: 1.25rem;
    border: 1px solid var(--mxi-color-border);
    border-radius: calc(var(--mxi-radius) * 1.5);
    background: var(--mxi-color-surface);
  }
  .tile h3 {
    margin: 0.75rem 0 0.4rem;
    font-size: 1.05rem;
  }
  .tile p {
    margin: 0;
    color: var(--mxi-color-muted);
    line-height: 1.5;
  }
  .badge {
    display: inline-grid;
    place-items: center;
    width: 2rem;
    height: 2rem;
    border-radius: 50%;
    background: var(--mxi-color-primary);
    color: var(--mxi-color-primary-fg);
    font-size: 0.875rem;
    font-weight: 700;
  }
  .closing {
    padding: 2rem 1.25rem;
    border: 1px solid var(--mxi-color-border);
    border-radius: calc(var(--mxi-radius) * 2);
    background: var(--mxi-color-surface);
    text-align: center;
  }
  .closing p {
    margin: 0 0 1.25rem;
    color: var(--mxi-color-muted);
  }
  .closing .cta.primary {
    background: var(--mxi-color-primary);
    color: var(--mxi-color-primary-fg);
  }
</style>
