<!--
  Tour — an in-depth, public walkthrough of how the app works.

  A "Before you begin" section (sign-in, header controls) followed by one
  section per workflow: a short summary, four numbered steps, and a link to
  the screen. A table of contents sits beside the sections on wide screens
  and above them on narrow ones. All copy comes from the i18n catalog
  (`tour.*` keys): `tour.start.*` for the first section and `tour.sN.*`
  (title, summary, step.1..step.4) for each workflow.

  Props:
    - sections: { href: string }[] — one entry per workflow, in order; the
      Nth entry's copy is `tour.s{N}.*`. `href` is the app screen to open.
-->
<script lang="ts">
  import { t, type StringKey } from "#lib/i18n.svelte.js";

  let { sections }: { sections: { href: string }[] } = $props();

  const STEPS = [1, 2, 3, 4] as const;
  const key = (k: string) => k as StringKey;

  // Section list including the generic "Before you begin" opener.
  const all = $derived([
    { id: "start", prefix: "tour.start", href: null as string | null },
    ...sections.map((s, i) => ({
      id: `s${i + 1}`,
      prefix: `tour.s${i + 1}`,
      href: s.href as string | null,
    })),
  ]);
</script>

<svelte:head><title>{t("nav.tour")} · {t("brand.name")}</title></svelte:head>

<div class="tour" id="top">
  <header class="tour-head">
    <p class="eyebrow">{t("brand.name")}</p>
    <h1>{t("tour.head")}</h1>
    <p class="lead">{t("tour.intro")}</p>
  </header>

  <div class="tour-body">
    <nav class="toc" aria-label={t("tour.toc")}>
      <h2>{t("tour.toc")}</h2>
      <ol>
        {#each all as s, i}
          <li>
            <a href={`#${s.id}`}>
              <span class="num" aria-hidden="true">{i + 1}</span>
              {t(key(`${s.prefix}.title`))}
            </a>
          </li>
        {/each}
      </ol>
    </nav>

    <div class="sections">
      {#each all as s, i}
        <section id={s.id} aria-labelledby={`${s.id}-title`}>
          <h2 id={`${s.id}-title`}>
            <span class="num" aria-hidden="true">{i + 1}</span>
            {t(key(`${s.prefix}.title`))}
          </h2>
          <p class="summary">{t(key(`${s.prefix}.summary`))}</p>
          <ol class="steps">
            {#each STEPS as n}
              <li>{t(key(`${s.prefix}.step.${n}`))}</li>
            {/each}
          </ol>
          <p class="section-links">
            {#if s.href}
              <a class="open" href={s.href}>{t("tour.open")}</a>
            {/if}
            <a class="top" href="#top">{t("tour.top")}</a>
          </p>
        </section>
      {/each}

      <section class="closing" aria-labelledby="tour-cta-title">
        <h2 id="tour-cta-title">{t("splash.cta.title")}</h2>
        <p>{t("splash.cta.body")}</p>
        <a class="cta" href="/signin">{t("auth.signin")}</a>
      </section>
    </div>
  </div>
</div>

<style>
  .tour {
    max-width: 72rem;
    margin: 0 auto;
  }
  .tour-head {
    margin-bottom: 2rem;
    padding: clamp(1.5rem, 4vw, 3rem) clamp(1.25rem, 4vw, 3rem);
    border-radius: calc(var(--mxi-radius) * 2);
    background: var(--mxi-color-primary);
    color: var(--mxi-color-primary-fg);
  }
  .eyebrow {
    margin: 0 0 0.5rem;
    font-size: 0.875rem;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    opacity: 0.85;
  }
  .tour-head h1 {
    margin: 0 0 0.75rem;
    font-size: clamp(1.75rem, 4vw, 2.75rem);
    line-height: 1.15;
  }
  .lead {
    margin: 0;
    max-width: 48rem;
    font-size: clamp(1rem, 2vw, 1.2rem);
    line-height: 1.5;
  }
  .tour-body {
    display: grid;
    grid-template-columns: 1fr;
    gap: 2rem;
    align-items: start;
  }
  @media (min-width: 60rem) {
    .tour-body {
      grid-template-columns: 16rem 1fr;
    }
    .toc {
      position: sticky;
      top: 1rem;
    }
  }
  .toc {
    padding: 1rem;
    border: 1px solid var(--mxi-color-border);
    border-radius: var(--mxi-radius);
    background: var(--mxi-color-surface);
  }
  .toc h2 {
    margin: 0 0 0.5rem;
    font-size: 0.875rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--mxi-color-muted);
  }
  .toc ol {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .toc a {
    display: flex;
    gap: 0.6rem;
    align-items: baseline;
    padding: 0.35rem 0.25rem;
    color: var(--mxi-color-fg);
  }
  .num {
    display: inline-grid;
    flex: none;
    place-items: center;
    width: 1.6rem;
    height: 1.6rem;
    border-radius: 50%;
    background: var(--mxi-color-primary);
    color: var(--mxi-color-primary-fg);
    font-size: 0.8rem;
    font-weight: 700;
  }
  .sections {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    min-width: 0;
  }
  section {
    scroll-margin-top: 1rem;
    padding: 1.5rem;
    border: 1px solid var(--mxi-color-border);
    border-radius: calc(var(--mxi-radius) * 1.5);
    background: var(--mxi-color-surface);
  }
  section h2 {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    margin: 0 0 0.5rem;
    font-size: clamp(1.2rem, 2.5vw, 1.5rem);
  }
  .summary {
    margin: 0 0 1rem;
    color: var(--mxi-color-muted);
    line-height: 1.55;
  }
  .steps {
    margin: 0;
    padding: 0;
    list-style: none;
    counter-reset: step;
  }
  .steps li {
    position: relative;
    margin: 0 0 0.75rem;
    padding-inline-start: 2.5rem;
    line-height: 1.55;
    counter-increment: step;
  }
  .steps li::before {
    content: counter(step);
    position: absolute;
    inset-inline-start: 0;
    top: 0.1rem;
    display: grid;
    place-items: center;
    width: 1.6rem;
    height: 1.6rem;
    border: 1px solid var(--mxi-color-primary);
    border-radius: 50%;
    color: var(--mxi-color-primary);
    font-size: 0.8rem;
    font-weight: 700;
  }
  .section-links {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    align-items: center;
    margin: 1rem 0 0;
  }
  .open {
    display: inline-block;
    padding: 0.45rem 1rem;
    border-radius: var(--mxi-radius);
    background: var(--mxi-color-primary);
    color: var(--mxi-color-primary-fg);
    font-weight: 600;
    text-decoration: none;
  }
  .top {
    font-size: 0.875rem;
  }
  .closing {
    text-align: center;
  }
  .closing h2 {
    justify-content: center;
  }
  .closing p {
    margin: 0 0 1.25rem;
    color: var(--mxi-color-muted);
  }
  .cta {
    display: inline-block;
    padding: 0.65rem 1.4rem;
    border-radius: var(--mxi-radius);
    background: var(--mxi-color-primary);
    color: var(--mxi-color-primary-fg);
    font-weight: 600;
    text-decoration: none;
  }
</style>
