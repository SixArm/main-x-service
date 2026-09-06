<svelte:head><title>Main X Index</title></svelte:head>

<h1>Main X Index</h1>

<p class="lede">
    A <strong>federated identity index</strong>: a family of small Rust
    services, one per kind of real-world entity — person, worker, place,
    thing, event, course, organization, care pathway, case, and plan/
    portfolio — each doing the same four things for its entity: store
    records, find likely duplicates, merge confirmed duplicates, and let
    other services and front-ends read and search them. A central
    single-sign-on provider and a cross-service link-graph aggregator tie
    the entities together without merging them into one big database.
</p>

<p>
    Every crate is self-contained — its own REST API, its own persistence,
    its own matching algorithm — and shares an architecture and
    documentation layout, not code. Every subproject's <code>spec/</code>
    directory is the single source of truth for that subproject; code
    conforms to the spec, not the reverse.
</p>

<h2>At a glance</h2>

<table>
    <tbody>
        <tr><th>Entity registry services</th><td>11 (10 matcher-backed entities + the central authentication service)</td></tr>
        <tr><th>Matcher crates</th><td>10 (dependency-light pairwise comparison libraries, crates.io-publishable)</td></tr>
        <tr><th>Library crates</th><td>3 (peer-side offline verification, integrity MACs, the cross-service link contract)</td></tr>
        <tr><th>Front-end projects</th><td>11 SvelteKit operator SPAs, one per service</td></tr>
        <tr><th>Consumer applications</th><td>5 (each with its own service + front-end edition)</td></tr>
        <tr><th>Cross-cutting services</th><td>1 (the read-model link-graph aggregator)</td></tr>
        <tr><th>Stack</th><td>Rust + loco.rs (Axum, SeaORM, PostgreSQL 18) · SvelteKit 2 + Svelte 5 runes + Lily Design System</td></tr>
    </tbody>
</table>

<p>
    See <a href="/architecture/">Architecture</a> for the layered request
    flow and the two internal service shapes, and
    <a href="/subprojects/">Subprojects</a> for the full family index with
    links to each crate's source and spec.
</p>

<h2>Elsewhere</h2>

<ul>
    <li><a href="https://github.com/SixArm/main-x-service">Source on GitHub</a></li>
    <li><a href="https://codeberg.org/SixArm/main-x-service">Source on Codeberg</a></li>
    <li><a href="https://github.com/SixArm/main-x-service/blob/main/llms.txt">llms.txt</a> / <a href="https://github.com/SixArm/main-x-service/blob/main/llms.json">llms.json</a> — a curated, size-bounded map of the repo for AI agents</li>
</ul>

<style>
    .lede {
        font-size: 1.05rem;
    }
</style>
