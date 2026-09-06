<svelte:head><title>About · Main X Index</title></svelte:head>

<h1>About this site</h1>

<p>
    This is the public documentation front door for
    <a href="https://github.com/SixArm/main-x-service">main-x-service</a>,
    the Main X Index monorepo. It is itself a subproject of that
    monorepo — <code>main-x-service.github.io/</code> — built with
    SvelteKit and the Lily Design System, and published to GitHub Pages
    via <code>git subtree</code> into a sibling read-only repository
    (per <code>spec/monorepo-github-pages/index.md</code>). Edits happen
    in the monorepo, never in the exported copy.
</p>

<h2>Spec-driven development</h2>

<p>
    Every subproject in the family carries its own <code>spec/</code>
    directory as the single source of truth; code conforms to the spec,
    not the reverse. A behavioural change is one PR carrying all three
    parts: spec edit, code edit, test edit. See the root
    <a href="https://github.com/SixArm/main-x-service/blob/main/AGENTS.md">AGENTS.md</a>
    for the full discipline.
</p>

<h2>AI-assisted development</h2>

<p>
    This repository documents how AI coding tools are used to develop it,
    and the bounds on that use, in
    <a href="https://github.com/SixArm/main-x-service/blob/main/AI_STATEMENT.md">AI_STATEMENT.md</a>.
</p>

<h2>License &amp; contact</h2>

<ul>
    <li>Rust crates: MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR GPL-3.0-only</li>
    <li>Front-ends (including this site): MIT OR Apache-2.0</li>
    <li>Contact: Joel Parker Henderson (<a href="mailto:joel@joelparkerhenderson.com">joel@joelparkerhenderson.com</a>)</li>
</ul>
